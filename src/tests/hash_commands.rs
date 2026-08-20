use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use crate::{
    ClientError, ErrorKind, RedisError, RedisErrorKind, Result,
    commands::{
        ExpireOption, FlushingMode, GenericCommands, GetExOptions, HScanOptions, HScanResult,
        HSetExCondition, HashCommands, ServerCommands, SetExpiration,
    },
    tests::get_test_client,
};
use serde::{Deserialize, Serialize};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn hdel() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let len = client.hdel("key", "field").await?;
    assert_eq!(1, len);

    let len = client.hdel("key", "field").await?;
    assert_eq!(0, len);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hexpire() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // no option
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hexpire("key", 10, None, "field").await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);

    // xx
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hexpire("key", 10, ExpireOption::Xx, "field").await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![-1]);

    // nx
    let result: Vec<i64> = client.hexpire("key", 10, ExpireOption::Nx, "field").await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);

    // gt
    let result: Vec<i64> = client.hexpire("key", 5, ExpireOption::Gt, "field").await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);
    let result: Vec<i64> = client.hexpire("key", 15, ExpireOption::Gt, "field").await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![15]);

    // lt
    let result: Vec<i64> = client.hexpire("key", 20, ExpireOption::Lt, "field").await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![15]);
    let result: Vec<i64> = client.hexpire("key", 5, ExpireOption::Lt, "field").await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![5]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hexpireat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();

    // Absolute expiries are set relative to `now` (captured in whole seconds),
    // but the server evaluates HTTL against its own clock; a second boundary
    // crossed between the two makes the remaining TTL one lower. Assert HTTL
    // within that one-second window rather than on an exact value.
    let assert_httl_near = |ttl: Vec<i64>, expected: i64| {
        assert_eq!(ttl.len(), 1, "unexpected httl shape: {ttl:?}");
        assert!(
            (expected - 1..=expected).contains(&ttl[0]),
            "httl {ttl:?} not within one second of {expected}"
        );
    };

    // no option
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hexpireat("key", now + 10, None, "field").await?;
    assert_eq!(result, vec![1]);
    assert_httl_near(client.httl("key", "field").await?, 10);

    // xx
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client
        .hexpireat("key", now + 10, ExpireOption::Xx, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![-1]);

    // nx
    let result: Vec<i64> = client
        .hexpireat("key", now + 10, ExpireOption::Nx, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_httl_near(client.httl("key", "field").await?, 10);

    // gt
    let result: Vec<i64> = client
        .hexpireat("key", now + 5, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_httl_near(client.httl("key", "field").await?, 10);
    let result: Vec<i64> = client
        .hexpireat("key", now + 15, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_httl_near(client.httl("key", "field").await?, 15);

    // lt
    let result: Vec<i64> = client
        .hexpireat("key", now + 20, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_httl_near(client.httl("key", "field").await?, 15);
    let result: Vec<i64> = client
        .hexpireat("key", now + 5, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_httl_near(client.httl("key", "field").await?, 5);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hexpiretime() -> Result<()> {
    let client = get_test_client().await?;

    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hexpireat("key", 33177117420, None, "field").await?;
    assert_eq!(result, vec![1]);
    let time: Vec<i64> = client.hexpiretime("key", "field").await?;
    assert_eq!(time, vec![33177117420]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hpersist() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.hset("key", ("field", "value")).await?;

    // field exists but has no expiration set
    let result: Vec<i64> = client.hpersist("key", "field").await?;
    assert_eq!(result, vec![-1]);

    // field with an expiration set: it is removed
    client.hexpire::<Vec<i64>>("key", 10, None, "field").await?;
    let result: Vec<i64> = client.hpersist("key", "field").await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![-1]);

    // missing field
    let result: Vec<i64> = client.hpersist("key", "missing").await?;
    assert_eq!(result, vec![-2]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hexists() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let result = client.hexists("key", "field").await?;
    assert!(result);

    let result = client.hexists("key", "unknown").await?;
    assert!(!result);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hget() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hgetdel() {
    let client = get_test_client().await.unwrap();

    // cleanup
    client.flushall(FlushingMode::Sync).await.unwrap();

    client
        .hset(
            "key",
            [("field1", "Hello"), ("field2", "World"), ("field3", "!")],
        )
        .await
        .unwrap();
    let values: Vec<Option<String>> = client.hgetdel("key", ["field3", "field4"]).await.unwrap();
    assert_eq!(values, vec![Some("!".to_string()), None]);

    let result: Vec<(String, String)> = client.hgetall("key").await.unwrap();
    assert_eq!(
        result,
        vec![
            ("field1".to_string(), "Hello".to_string()),
            ("field2".to_string(), "World".to_string())
        ]
    );

    let values: Vec<String> = client.hgetdel("key", ["field1", "field2"]).await.unwrap();
    assert_eq!(values, vec!["Hello".to_string(), "World".to_string()]);

    let result = client.exists("key").await.unwrap();
    assert_eq!(result, 0);
}

#[tokio::test]
#[serial]
async fn hgetex() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushall(FlushingMode::Sync).await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;

    let values: [String; 1] = client
        .hgetex("key", GetExOptions::Ex(120), "field1")
        .await?;
    assert_eq!(values, ["Hello".to_string()]);

    let values: [String; 1] = client
        .hgetex("key", GetExOptions::Ex(100), "field2")
        .await?;
    assert_eq!(values, ["World".to_string()]);

    let result: [i64; 3] = client.httl("key", ["field1", "field2", "field3"]).await?;
    assert_eq!(result, [120, 100, -2]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hget_all() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let result: HashMap<String, String> = client.hgetall("key").await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(&"Hello".to_owned()), result.get("field1"));
    assert_eq!(Some(&"World".to_owned()), result.get("field2"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn hincrby() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "5")).await?;
    let value = client.hincrby("key", "field", 1).await?;
    assert_eq!(6, value);
    let value = client.hincrby("key", "field", -1).await?;
    assert_eq!(5, value);
    let value = client.hincrby("key", "field", -10).await?;
    assert_eq!(-5, value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hincrbyfloat() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "10.50")).await?;
    let value = client.hincrbyfloat("key", "field", 0.1).await?;
    assert_eq!(10.6, value);
    let value = client.hincrbyfloat("key", "field", -5.0).await?;
    assert_eq!(5.6, value);
    client.hset("key", ("field", "5.0e3")).await?;
    let value = client.hincrbyfloat("key", "field", 2.0e2).await?;
    assert_eq!(5200.0, value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hkeys() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let fields: Vec<String> = client.hkeys("key").await?;
    assert_eq!(2, fields.len());
    assert_eq!("field1".to_owned(), fields[0]);
    assert_eq!("field2".to_owned(), fields[1]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hlen() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let len = client.hlen("key").await?;
    assert_eq!(2, len);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hmget() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    // A field the hash does not hold answers nil, so the element type carries
    // the hole: `String` would read it as `""`, which a present field can hold.
    let values: Vec<Option<String>> = client.hmget("key", ["field1", "field2", "nofield"]).await?;
    assert_eq!(
        vec![Some("Hello".to_owned()), Some("World".to_owned()), None],
        values
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn hpexpire() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // no option
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hpexpire("key", 10000, None, "field").await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 10000);

    // xx
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client
        .hpexpire("key", 10000, ExpireOption::Xx, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.hpttl::<Vec<i64>>("key", "field").await?, vec![-1]);

    // nx
    let result: Vec<i64> = client
        .hpexpire("key", 10000, ExpireOption::Nx, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 10000);

    // gt
    let result: Vec<i64> = client
        .hpexpire("key", 5000, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 10000);
    let result: Vec<i64> = client
        .hpexpire("key", 15000, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 15000);

    // lt
    let result: Vec<i64> = client
        .hpexpire("key", 20000, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 15000);
    let result: Vec<i64> = client
        .hpexpire("key", 5000, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 5000);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hpexpireat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis() as u64;

    // no option
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hpexpireat("key", now + 10000, None, "field").await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 10000);

    // xx
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client
        .hpexpireat("key", now + 10000, ExpireOption::Xx, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.hpttl::<Vec<i64>>("key", "field").await?, vec![-1]);

    // nx
    let result: Vec<i64> = client
        .hpexpireat("key", now + 10000, ExpireOption::Nx, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 10000);

    // gt
    let result: Vec<i64> = client
        .hpexpireat("key", now + 5000, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 10000);
    let result: Vec<i64> = client
        .hpexpireat("key", now + 15000, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 15000);

    // lt
    let result: Vec<i64> = client
        .hpexpireat("key", now + 20000, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 15000);
    let result: Vec<i64> = client
        .hpexpireat("key", now + 5000, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert!(client.hpttl::<Vec<i64>>("key", "field").await?[0] <= 5000);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hpexpiretime() -> Result<()> {
    let client = get_test_client().await?;

    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client
        .hpexpireat("key", 33177117420000, None, "field")
        .await?;
    assert_eq!(result, vec![1]);
    let time: Vec<i64> = client.hpexpiretime("key", "field").await?;
    assert_eq!(time, vec![33177117420000]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hrandfield() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("coin").await?;

    let fields_and_values = [("heads", "obverse"), ("tails", "reverse"), ("edge", "")];
    client.hset("coin", fields_and_values).await?;

    let value: String = client.hrandfield("coin").await?;
    assert!(fields_and_values.iter().any(|v| v.0 == value));

    let values: Vec<String> = client.hrandfields("coin", -5).await?;
    assert_eq!(5, values.len());
    for value in values {
        assert!(fields_and_values.iter().any(|v| v.0 == value));
    }

    let values: Vec<String> = client.hrandfields("coin", 5).await?;
    assert_eq!(3, values.len());
    for value in values {
        assert!(fields_and_values.iter().any(|v| v.0 == value));
    }

    let values: Vec<(String, String)> = client.hrandfields_with_values("coin", 5).await?;
    assert_eq!(3, values.len());
    for value in values {
        assert!(
            fields_and_values
                .iter()
                .any(|v| v.0 == value.0 && v.1 == value.1)
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn hscan() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let fields_and_values: Vec<_> = (1..21)
        .map(|i| (format!("field{i}"), format!("value{i}")))
        .collect();

    client.hset("key", fields_and_values).await?;

    let result: HScanResult<String, String> = client
        .hscan("key", 0, HScanOptions::default().count(20))
        .await?;

    assert_eq!(0, result.cursor);
    assert_eq!(20, result.elements.len());
    assert_eq!(
        ("field1".to_owned(), "value1".to_owned()),
        result.elements[0]
    );
    assert_eq!(
        ("field2".to_owned(), "value2".to_owned()),
        result.elements[1]
    );
    assert_eq!(
        ("field3".to_owned(), "value3".to_owned()),
        result.elements[2]
    );
    assert_eq!(
        ("field4".to_owned(), "value4".to_owned()),
        result.elements[3]
    );

    Ok(())
}

/// `HSCAN key cursor [MATCH pattern] [COUNT count] [NOVALUES]`. With NOVALUES the
/// server answers the field names alone, not field/value pairs.
#[tokio::test]
#[serial]
async fn hscan_no_values() -> Result<()> {
    let client = get_test_client().await?;

    client.del("key").await?;
    client
        .hset("key", [("field1", "value1"), ("field2", "value2")])
        .await?;

    let (cursor, fields): (u64, Vec<String>) = client
        .hscan_no_values("key", 0, HScanOptions::default().count(20))
        .await?;

    assert_eq!(0, cursor);
    assert_eq!(vec!["field1".to_owned(), "field2".to_owned()], fields);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hsetex() -> Result<()> {
    let client = get_test_client().await?;

    // EX
    client
        .hsetex("key", None, Some(SetExpiration::Ex(1)), ("field", "value"))
        .await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let ttl: Vec<i64> = client.hpttl("key", "field").await?;
    assert!(ttl[0] <= 1000);

    // PX
    client
        .hsetex("key", None, SetExpiration::Px(1000), ("field", "value"))
        .await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let ttl: Vec<i64> = client.hpttl("key", "field").await?;
    assert!(ttl[0] <= 1000);

    // EXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    client
        .hsetex("key", None, SetExpiration::Exat(time), ("field", "value"))
        .await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let ttl: Vec<i64> = client.hpttl("key", "field").await?;
    assert!(ttl[0] <= 1000);

    // PXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    client
        .hsetex(
            "key",
            None,
            SetExpiration::Pxat(time as u64),
            ("field", "value"),
        )
        .await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let ttl: Vec<i64> = client.hpttl("key", "field").await?;
    assert!(ttl[0] <= 1000);

    // FNX
    client.del("key").await?;
    let result = client
        .hsetex("key", HSetExCondition::FNX, None, ("field", "value"))
        .await?;
    assert!(result);
    let result = client
        .hsetex("key", HSetExCondition::FNX, None, ("field", "value"))
        .await?;
    assert!(!result);

    // FXX
    client.del("key").await?;
    let result = client
        .hsetex("key", HSetExCondition::FXX, None, ("field", "value"))
        .await?;
    assert!(!result);
    client.hset("key", ("field", "value")).await?;
    let result = client
        .hsetex("key", HSetExCondition::FXX, None, ("field", "value"))
        .await?;
    assert!(result);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn hsetnx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let result = client.hsetnx("key", "field", "Hello").await?;
    assert!(result);

    let result = client.hsetnx("key", "field", "World").await?;
    assert!(!result);

    let value: String = client.hget("key", "field").await?;
    assert_eq!("Hello", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hstrlen() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;

    let len = client.hstrlen("key", "field").await?;
    assert_eq!(5, len);

    Ok(())
}

#[tokio::test]
#[serial]
async fn hvals() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;

    let values: Vec<String> = client.hvals("key").await?;
    assert_eq!(2, values.len());
    assert_eq!("Hello", values[0]);
    assert_eq!("World", values[1]);

    Ok(())
}

/// A `struct` goes into a hash and comes back out of it in one call: `HSET`
/// serializes it as flat field/value pairs, taking the field names from the
/// struct's own, and `HGETALL` deserializes the reply back into it.
#[tokio::test]
#[serial]
async fn hset_hgetall_struct() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Person {
        id: u32,
        name: String,
        height: f64,
        active: bool,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    let person = Person {
        id: 12,
        name: "Foo".to_owned(),
        height: 1.75,
        active: true,
    };

    // sends HSET person id 12 name Foo height 1.75 active 1
    let len = client.hset("person", &person).await?;
    assert_eq!(4, len);

    // the hash really holds one Redis field per struct field
    let mut fields: Vec<String> = client.hkeys("person").await?;
    fields.sort();
    assert_eq!(["active", "height", "id", "name"], fields.as_slice());

    let name: String = client.hget("person", "name").await?;
    assert_eq!("Foo", name);

    let read: Person = client.hgetall("person").await?;
    assert_eq!(person, read);

    Ok(())
}

/// A hash holds every value as a bulk string, whatever its type, so a struct of
/// primitives makes the round trip through that single wire form: the argument
/// serializer writes each value as text, and the deserializer parses it back
/// into the target type.
#[tokio::test]
#[serial]
async fn hset_hgetall_struct_of_primitives() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Primitives {
        small_signed: i8,
        signed: i64,
        small_unsigned: u8,
        unsigned: u64,
        single: f32,
        double: f64,
        flag: bool,
        letter: char,
        text: String,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("primitives").await?;

    let primitives = Primitives {
        small_signed: -8,
        signed: i64::MIN,
        small_unsigned: 255,
        unsigned: u64::MAX,
        single: 1.75,
        double: -12.5e-3,
        flag: true,
        letter: 'm',
        text: "hello".to_owned(),
    };

    let len = client.hset("primitives", &primitives).await?;
    assert_eq!(9, len);

    // the text actually stored, which is what another client would read
    let stored: HashMap<String, String> = client.hgetall("primitives").await?;
    assert_eq!(Some(&"-8".to_owned()), stored.get("small_signed"));
    assert_eq!(Some(&"255".to_owned()), stored.get("small_unsigned"));
    assert_eq!(
        Some(&"18446744073709551615".to_owned()),
        stored.get("unsigned")
    );
    assert_eq!(Some(&"1.75".to_owned()), stored.get("single"));
    // a `bool` goes out as 1/0, and reads back from either that or true/false
    assert_eq!(Some(&"1".to_owned()), stored.get("flag"));
    assert_eq!(Some(&"m".to_owned()), stored.get("letter"));

    let read: Primitives = client.hgetall("primitives").await?;
    assert_eq!(primitives, read);

    // field by field, through the same deserializer
    let signed: i64 = client.hget("primitives", "signed").await?;
    assert_eq!(i64::MIN, signed);
    let double: f64 = client.hget("primitives", "double").await?;
    assert_eq!(-12.5e-3, double);
    let flag: bool = client.hget("primitives", "flag").await?;
    assert!(flag);

    Ok(())
}

/// Reading a missing field gives a nil, which is `None` - and, for a non-`Option`
/// target, the type's zero rather than an error.
#[tokio::test]
#[serial]
async fn hget_missing_field() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;

    let missing: Option<String> = client.hget("key", "unknown").await?;
    assert_eq!(None, missing);

    let missing: Option<u32> = client.hget("key", "unknown").await?;
    assert_eq!(None, missing);

    // Asking for the value of an absent field as a bare `u32` has no honest
    // answer: `0` is the value a present field can hold.
    let error = client
        .hget::<u32>("key", "unknown")
        .await
        .expect_err("a missing field is not a u32");
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

/// 128-bit integers make the round trip like any other width. Both directions
/// matter: the deserializer has always read them, so a serializer that could not
/// write them made a type readable but not writable.
#[tokio::test]
#[serial]
async fn hset_hgetall_struct_with_128_bit_integers() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Wide {
        signed: i128,
        unsigned: u128,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("wide").await?;

    let wide = Wide {
        signed: i128::MIN,
        unsigned: u128::MAX,
    };
    client.hset("wide", &wide).await?;

    let stored: String = client.hget("wide", "unsigned").await?;
    assert_eq!(u128::MAX.to_string(), stored);

    let read: Wide = client.hgetall("wide").await?;
    assert_eq!(wide, read);

    Ok(())
}

/// The field names on the wire are the serialized ones, so `rename`/`rename_all`
/// decide the hash layout and are honored in both directions.
#[tokio::test]
#[serial]
async fn hset_hgetall_struct_with_renamed_fields() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Person {
        first_name: String,
        #[serde(rename = "yob")]
        year_of_birth: u16,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    let person = Person {
        first_name: "Foo".to_owned(),
        year_of_birth: 1976,
    };

    client.hset("person", &person).await?;

    let mut fields: Vec<String> = client.hkeys("person").await?;
    fields.sort();
    assert_eq!(["firstName", "yob"], fields.as_slice());

    let read: Person = client.hgetall("person").await?;
    assert_eq!(person, read);

    Ok(())
}

/// Reading a struct out of a hash tolerates fields the struct does not know:
/// serde skips them. A hash written by a newer version of the application, or
/// shared by several of them, still deserializes.
#[tokio::test]
#[serial]
async fn hgetall_struct_ignores_unknown_fields() -> Result<()> {
    #[derive(Debug, PartialEq, Deserialize)]
    struct Person {
        id: u32,
        name: String,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    client
        .hset(
            "person",
            [("id", "12"), ("name", "Foo"), ("added_later", "ignored")],
        )
        .await?;

    let read: Person = client.hgetall("person").await?;
    assert_eq!(
        Person {
            id: 12,
            name: "Foo".to_owned()
        },
        read
    );

    Ok(())
}

/// `None` serializes to no argument at all, which would leave its field name
/// paired with the next field's. An optional field must therefore be skipped
/// whole with `skip_serializing_if`; on the way back, the missing field
/// deserializes to `None`.
#[tokio::test]
#[serial]
async fn hset_hgetall_struct_with_optional_field() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Person {
        id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    let with_nickname = Person {
        id: 12,
        nickname: Some("Foo".to_owned()),
    };
    client.hset("person", &with_nickname).await?;
    let read: Person = client.hgetall("person").await?;
    assert_eq!(with_nickname, read);

    // cleanup
    client.del("person").await?;

    let without_nickname = Person {
        id: 12,
        nickname: None,
    };
    client.hset("person", &without_nickname).await?;
    let fields: Vec<String> = client.hkeys("person").await?;
    assert_eq!(["id"], fields.as_slice());
    let read: Person = client.hgetall("person").await?;
    assert_eq!(without_nickname, read);

    Ok(())
}

/// The counterpart of the test above: an `Option` field left unskipped writes
/// its name with no value, and the server rejects the odd argument count. The
/// error is the good case here - a struct with two `None` fields would produce
/// an even count and silently store a field name as another field's value.
#[tokio::test]
#[serial]
async fn hset_struct_with_unskipped_none_is_rejected() -> Result<()> {
    #[derive(Debug, Serialize)]
    struct Person {
        id: u32,
        nickname: Option<String>,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    // sends HSET person id 12 nickname
    let result = client
        .hset(
            "person",
            &Person {
                id: 12,
                nickname: None,
            },
        )
        .await;

    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(
            redis @ RedisError {
                kind: RedisErrorKind::Err,
                ..
            },
        ) if redis.description().contains("wrong number of arguments")
    ));

    Ok(())
}

/// `HMGET` returns values only, in the order the fields were asked for, so it
/// deserializes into a tuple - or into a struct read positionally, as long as
/// the fields are requested in declaration order.
#[tokio::test]
#[serial]
async fn hmget_struct() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Person {
        id: u32,
        name: String,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    let person = Person {
        id: 12,
        name: "Foo".to_owned(),
    };
    client.hset("person", &person).await?;

    let (id, name): (u32, String) = client.hmget("person", ["id", "name"]).await?;
    assert_eq!(12, id);
    assert_eq!("Foo", name);

    let read: Person = client.hmget("person", ["id", "name"]).await?;
    assert_eq!(person, read);

    Ok(())
}

/// A hash is flat, so a nested struct cannot map onto it: the inner fields
/// would flatten into the outer pairs and shift them. `#[serde(flatten)]` is
/// the supported way to spread an inner struct over the same hash, and a nested
/// value that must stay nested belongs in a single field, serialized on its own.
#[cfg(feature = "json")]
#[tokio::test]
#[serial]
async fn hset_hgetall_nested_struct() -> Result<()> {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Address {
        street: String,
        city: String,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Flattened {
        id: u32,
        #[serde(flatten)]
        address: Address,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Nested {
        id: u32,
        /// Held as a JSON string in a single hash field.
        address: String,
    }

    let client = get_test_client().await?;

    // cleanup
    client.del("person").await?;

    let address = Address {
        street: "1 Foo street".to_owned(),
        city: "Bar".to_owned(),
    };

    // flattened: one hash field per leaf
    let flattened = Flattened {
        id: 12,
        address: Address {
            street: address.street.clone(),
            city: address.city.clone(),
        },
    };
    let len = client.hset("person", &flattened).await?;
    assert_eq!(3, len);
    let mut fields: Vec<String> = client.hkeys("person").await?;
    fields.sort();
    assert_eq!(["city", "id", "street"], fields.as_slice());

    let read: Flattened = client.hgetall("person").await?;
    assert_eq!(flattened, read);

    // cleanup
    client.del("person").await?;

    // nested: the sub-struct is serialized by the caller into one field
    let nested = Nested {
        id: 12,
        address: serde_json::to_string(&address).unwrap(),
    };
    let len = client.hset("person", &nested).await?;
    assert_eq!(2, len);

    let read: Nested = client.hgetall("person").await?;
    assert_eq!(nested, read);
    let read_address: Address = serde_json::from_str(&read.address).unwrap();
    assert_eq!(address, read_address);

    Ok(())
}
