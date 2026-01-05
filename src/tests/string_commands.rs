use crate::{
    Error, RedisError, RedisErrorKind, Result,
    commands::{
        FlushingMode, GenericCommands, GetExOptions, LcsMatch, ServerCommands, SetCondition,
        SetExpiration, StringCommands,
    },
    resp::Value,
    tests::get_test_client,
};
use serial_test::serial;
use std::time::{Duration, SystemTime};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn append() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let new_size = client.append("key", "12").await?;
    assert_eq!(7, new_size);

    let value: String = client.get("key").await?;
    assert_eq!("value12", value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.decr("key").await?;
    assert_eq!(-1, value);

    client.set("key", "12").await?;

    let value = client.decr("key").await?;
    assert_eq!(11, value);

    client.set("key", "value").await?;

    let result = client.decr("key").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decrby() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.decrby("key", 2).await?;
    assert_eq!(-2, value);

    client.set("key", "12").await?;

    let value = client.decrby("key", 2).await?;
    assert_eq!(10, value);

    client.set("key", "value").await?;

    let result = client.decrby("key", 2).await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_and_set() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_ex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Px(1000)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_exat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    let value: String = client.getex("key", GetExOptions::Exat(time)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pxat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    let value: String = client.getex("key", GetExOptions::Pxat(time as u64)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_persist() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let value: String = client.getex("key", GetExOptions::Persist).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert_eq!(-1, ttl);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getrange() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("mykey", "This is a string").await?;

    let value: String = client.getrange("mykey", 0, 3).await?;
    assert_eq!("This", value);
    let value: String = client.getrange("mykey", -3, -1).await?;
    assert_eq!("ing", value);
    let value: String = client.getrange("mykey", 0, -1).await?;
    assert_eq!("This is a string", value);
    let value: String = client.getrange("mykey", 10, 100).await?;
    assert_eq!("string", value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getset() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let value: String = client.getset("key", "newvalue").await?;
    assert_eq!("value", value);

    client.del("key").await?;

    let value: Value = client.getset("key", "newvalue").await?;
    assert!(matches!(value, Value::Nil));

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.incr("key").await?;
    assert_eq!(1, value);

    client.set("key", "12").await?;

    let value = client.incr("key").await?;
    assert_eq!(13, value);

    client.set("key", "value").await?;

    let result = client.incr("key").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrby() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.incrby("key", 2).await?;
    assert_eq!(2, value);

    client.set("key", "12").await?;

    let value = client.incrby("key", 2).await?;
    assert_eq!(14, value);

    client.set("key", "value").await?;

    let result = client.incrby("key", 2).await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrbyfloat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    client.set("key", "10.50").await?;

    let value = client.incrbyfloat("key", 0.1).await?;
    assert_eq!(10.6, value);

    let value = client.incrbyfloat("key", -5f64).await?;
    assert_eq!(5.6, value);

    client.set("key", "5.0e3").await?;

    let value = client.incrbyfloat("key", 2.0e2f64).await?;
    assert_eq!(5200f64, value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lcs() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client
        .mset([("key1", "ohmytext"), ("key2", "mynewtext")])
        .await?;

    let result: String = client.lcs("key1", "key2").await?;
    assert_eq!("mytext", result);

    let result = client.lcs_len("key1", "key2").await?;
    assert_eq!(6, result);

    let result = client.lcs_idx("key1", "key2", None, false).await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(LcsMatch((4, 7), (5, 8), None), result.matches[0]);
    assert_eq!(LcsMatch((2, 3), (0, 1), None), result.matches[1]);

    let result = client.lcs_idx("key1", "key2", Some(4), false).await?;
    assert_eq!(6, result.len);
    assert_eq!(1, result.matches.len());
    assert_eq!(LcsMatch((4, 7), (5, 8), None), result.matches[0]);

    let result = client.lcs_idx("key1", "key2", None, true).await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(LcsMatch((4, 7), (5, 8), Some(4)), result.matches[0]);
    assert_eq!(LcsMatch((2, 3), (0, 1), Some(2)), result.matches[1]);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn mget_mset() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    let keys = ["key1", "key2", "key3", "key4"];

    // cleanup
    client.del(keys).await?;

    let items = [("key1", "value1"), ("key2", "value2"), ("key3", "value3")];
    client.mset(items).await?;

    let values: Vec<Option<String>> = client.mget(keys).await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn msetnx() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).await?;

    let success = client
        .msetnx([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;
    assert!(success);

    let values: Vec<Option<String>> = client.mget(["key1", "key2", "key3", "key4"]).await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    let success = client
        .msetnx([("key1", "value1"), ("key4", "value4")])
        .await?;
    assert!(!success);

    let values: Vec<Option<String>> = client.mget(["key1", "key4"]).await?;
    assert_eq!(2, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert_eq!(values[1], None);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn psetex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.psetex("key", 1000, "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn set_with_options() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // EX
    client
        .set_with_options(
            "key",
            "value",
            None,
            Some(SetExpiration::Ex(1)),
        )
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // PX
    client
        .set_with_options(
            "key",
            "value",
            None,
            Some(SetExpiration::Px(1000)),
        )
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // EXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    client
        .set_with_options(
            "key",
            "value",
            None,
            Some(SetExpiration::Exat(time)),
        )
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // PXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    client
        .set_with_options(
            "key",
            "value",
            None,
            Some(SetExpiration::Pxat(time as u64)),
        )
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // NX
    client.del("key").await?;
    let result = client
        .set_with_options("key", "value", Some(SetCondition::NX), None)
        .await?;
    assert!(result);
    let result = client
        .set_with_options("key", "value", Some(SetCondition::NX), None)
        .await?;
    assert!(!result);

    // XX
    client.del("key").await?;
    let result = client
        .set_with_options("key", "value", Some(SetCondition::XX), None)
        .await?;
    assert!(!result);
    client.set("key", "value").await?;
    let result = client
        .set_with_options("key", "value", Some(SetCondition::XX), None)
        .await?;
    assert!(result);

    // GET
    client.del("key").await?;
    let result: Option<String> = client
        .set_get_with_options(
            "key",
            "value",
            None,
            None,
        )
        .await?;
    assert!(result.is_none());
    client.set("key", "value").await?;
    let result: String = client
        .set_get_with_options(
            "key",
            "value1",
            None,
            None,
        )
        .await?;
    assert_eq!("value", result);
    let value: String = client.get("key").await?;
    assert_eq!("value1", value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.setex("key", 1, "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setnx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushall(FlushingMode::Sync).await?;

    let result = client.setnx("key", "value").await?;
    let value: String = client.get("key").await?;
    assert!(result);
    assert_eq!("value", value);

    let result = client.setnx("key", "value1").await?;
    assert!(!result);
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setrange() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "Hello World").await?;

    let new_len = client.setrange("key", 6, "Redis").await?;
    assert_eq!(11, new_len);

    let value: String = client.get("key").await?;
    assert_eq!("Hello Redis", value);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn strlen() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "Hello World").await?;

    let len = client.strlen("key").await?;
    assert_eq!(11, len);

    let len = client.strlen("nonexisting").await?;
    assert_eq!(0, len);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn substr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("mykey", "This is a string").await?;

    let value: String = client.substr("mykey", 0, 3).await?;
    assert_eq!("This", value);
    let value: String = client.substr("mykey", -3, -1).await?;
    assert_eq!("ing", value);
    let value: String = client.substr("mykey", 0, -1).await?;
    assert_eq!("This is a string", value);
    let value: String = client.substr("mykey", 10, 100).await?;
    assert_eq!("string", value);

    client.close().await?;

    Ok(())
}
