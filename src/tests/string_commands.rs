use crate::{
    commands::{GenericCommands, GetExOptions, SetCondition, SetExpiration, StringCommands},
    resp::Value,
    tests::get_test_client,
    Error, RedisError, RedisErrorKind, Result,
};
use serial_test::serial;
use std::time::{Duration, SystemTime};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn append() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "value").await?;

    let new_size = client.append("key", "12").await?;
    assert_eq!(7, new_size);

    let value: String = client.get("key").await?;
    assert_eq!("value12", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decr() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decrby() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_and_set() -> Result<()> {
    let mut client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_ex() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pex() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Px(1000)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_exat() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pxat() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_persist() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let value: String = client.getex("key", GetExOptions::Persist).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert_eq!(-1, ttl);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getrange() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "value").await?;

    let value: String = client.getrange("key", 1, 3).await?;
    assert_eq!("alu", value);

    let value: String = client.getrange("key", 1, -3).await?;
    assert_eq!("al", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getset() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "value").await?;

    let value: String = client.getset("key", "newvalue").await?;
    assert_eq!("value", value);

    client.del("key").await?;

    let value: Value = client.getset("key", "newvalue").await?;
    assert!(matches!(value, Value::Nil));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incr() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrby() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrbyfloat() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lcs() -> Result<()> {
    let mut client = get_test_client().await?;

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
    assert_eq!(((4, 7), (5, 8), None), result.matches[0]);
    assert_eq!(((2, 3), (0, 1), None), result.matches[1]);

    let result = client.lcs_idx("key1", "key2", Some(4), false).await?;
    assert_eq!(6, result.len);
    assert_eq!(1, result.matches.len());
    assert_eq!(((4, 7), (5, 8), None), result.matches[0]);

    let result = client.lcs_idx("key1", "key2", None, true).await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(((4, 7), (5, 8), Some(4)), result.matches[0]);
    assert_eq!(((2, 3), (0, 1), Some(2)), result.matches[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn mget_mset() -> Result<()> {
    let mut client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).await?;

    client
        .mset([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;

    let values: Vec<Option<String>> = client.mget(["key1", "key2", "key3", "key4"]).await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn msetnx() -> Result<()> {
    let mut client = get_test_client().await?;

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

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn psetex() -> Result<()> {
    let mut client = get_test_client().await?;

    client.psetex("key", 1000, "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn set_with_options() -> Result<()> {
    let mut client = get_test_client().await?;

    // EX
    client
        .set_with_options(
            "key",
            "value",
            Default::default(),
            SetExpiration::Ex(1),
            false,
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
            Default::default(),
            SetExpiration::Px(1000),
            false,
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
            Default::default(),
            SetExpiration::Exat(time),
            false,
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
            Default::default(),
            SetExpiration::Pxat(time as u64),
            false,
        )
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // NX
    client.del("key").await?;
    let result = client
        .set_with_options("key", "value", SetCondition::NX, Default::default(), false)
        .await?;
    assert!(result);
    let result = client
        .set_with_options("key", "value", SetCondition::NX, Default::default(), false)
        .await?;
    assert!(!result);

    // XX
    client.del("key").await?;
    let result = client
        .set_with_options("key", "value", SetCondition::XX, Default::default(), false)
        .await?;
    assert!(!result);
    client.set("key", "value").await?;
    let result = client
        .set_with_options("key", "value", SetCondition::XX, Default::default(), false)
        .await?;
    assert!(result);

    // GET
    client.del("key").await?;
    let result: Option<String> = client
        .set_get_with_options(
            "key",
            "value",
            Default::default(),
            Default::default(),
            false,
        )
        .await?;
    assert!(result.is_none());
    client.set("key", "value").await?;
    let result: String = client
        .set_get_with_options(
            "key",
            "value1",
            Default::default(),
            Default::default(),
            false,
        )
        .await?;
    assert_eq!("value", result);
    let value: String = client.get("key").await?;
    assert_eq!("value1", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setex() -> Result<()> {
    let mut client = get_test_client().await?;

    client.setex("key", 1, "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setnx() -> Result<()> {
    let mut client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let result = client.setnx("key", "value").await?;
    let value: String = client.get("key").await?;
    assert!(result);
    assert_eq!("value", value);

    let result = client.setnx("key", "value1").await?;
    assert!(!result);
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setrange() -> Result<()> {
    let mut client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.set("key", "Hello World").await?;

    let new_len = client.setrange("key", 6, "Redis").await?;
    assert_eq!(11, new_len);

    let value: String = client.get("key").await?;
    assert_eq!("Hello Redis", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn strlen() -> Result<()> {
    let mut client = get_test_client().await?;

    client.set("key", "Hello World").await?;

    let len = client.strlen("key").await?;
    assert_eq!(11, len);

    let len = client.strlen("nonexisting").await?;
    assert_eq!(0, len);

    Ok(())
}
