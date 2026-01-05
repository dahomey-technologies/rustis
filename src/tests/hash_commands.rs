use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use crate::{
    Result,
    commands::{
        ExpireOption, FlushingMode, GenericCommands, GetExOptions, HScanOptions, HScanResult,
        HSetExCondition, HashCommands, ServerCommands, SetExpiration,
    },
    tests::get_test_client,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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
    let result: Vec<i64> = client
        .hexpire("key", 10, ExpireOption::Xx, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![-1]);

    // nx
    let result: Vec<i64> = client
        .hexpire("key", 10, ExpireOption::Nx, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);

    // gt
    let result: Vec<i64> = client
        .hexpire("key", 5, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);
    let result: Vec<i64> = client
        .hexpire("key", 15, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![15]);

    // lt
    let result: Vec<i64> = client
        .hexpire("key", 20, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![15]);
    let result: Vec<i64> = client
        .hexpire("key", 5, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![5]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hexpireat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();

    // no option
    client.hset("key", ("field", "value")).await?;
    let result: Vec<i64> = client.hexpireat("key", now + 10, None, "field").await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);

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
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);

    // gt
    let result: Vec<i64> = client
        .hexpireat("key", now + 5, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![10]);
    let result: Vec<i64> = client
        .hexpireat("key", now + 15, ExpireOption::Gt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![15]);

    // lt
    let result: Vec<i64> = client
        .hexpireat("key", now + 20, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![0]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![15]);
    let result: Vec<i64> = client
        .hexpireat("key", now + 5, ExpireOption::Lt, "field")
        .await?;
    assert_eq!(result, vec![1]);
    assert_eq!(client.httl::<Vec<i64>>("key", "field").await?, vec![5]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hgetdel() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushall(FlushingMode::Sync).await?;

    client
        .hset(
            "key",
            [("field1", "Hello"), ("field2", "World"), ("field3", "!")],
        )
        .await?;
    let values: Vec<Option<String>> = client.hgetdel("key", ["field3", "field4"]).await?;
    assert_eq!(values, vec![Some("!".to_string()), None]);

    let result: Vec<(String, String)> = client.hgetall("key").await?;
    assert_eq!(
        result,
        vec![
            ("field1".to_string(), "Hello".to_string()),
            ("field2".to_string(), "World".to_string())
        ]
    );

    let values: Vec<String> = client.hgetdel("key", ["field1", "field2"]).await?;
    assert_eq!(values, vec!["Hello".to_string(), "World".to_string()]);

    let result = client.exists("key").await?;
    assert_eq!(result, 0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hmget() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let values: Vec<String> = client.hmget("key", ["field1", "field2", "nofield"]).await?;
    assert_eq!(3, values.len());
    assert_eq!("Hello".to_owned(), values[0]);
    assert_eq!("World".to_owned(), values[1]);
    assert_eq!("".to_owned(), values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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
        .hsetex(
            "key",
            None,
            SetExpiration::Px(1000),
            ("field", "value"),
        )
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
        .hsetex(
            "key",
            None,
            SetExpiration::Exat(time),
            ("field", "value"),
        )
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
        .hsetex(
            "key",
            HSetExCondition::FNX,
            None,
            ("field", "value"),
        )
        .await?;
    assert!(result);
    let result = client
        .hsetex(
            "key",
            HSetExCondition::FNX,
            None,
            ("field", "value"),
        )
        .await?;
    assert!(!result);

    // FXX
    client.del("key").await?;
    let result = client
        .hsetex(
            "key",
            HSetExCondition::FXX,
            None,
            ("field", "value"),
        )
        .await?;
    assert!(!result);
    client.hset("key", ("field", "value")).await?;
    let result = client
        .hsetex(
            "key",
            HSetExCondition::FXX,
            None,
            ("field", "value"),
        )
        .await?;
    assert!(result);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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
