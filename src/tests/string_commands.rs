use crate::{
    resp::{BulkString, Value},
    tests::get_default_addr,
    ConnectionMultiplexer, Error, GenericCommands, GetExOptions, Result, SetCondition,
    SetExpiration, StringCommands,
};
use serial_test::serial;
use std::time::{Duration, SystemTime};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn append() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let new_size = database.append("key", "12").await?;
    assert_eq!(7, new_size);

    let value: String = database.get("key").await?;
    assert_eq!("value12", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decr() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let value = database.decr("key").await?;
    assert_eq!(-1, value);

    database.set("key", "12").await?;

    let value = database.decr("key").await?;
    assert_eq!(11, value);

    database.set("key", "value").await?;

    let result = database.decr("key").await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decrby() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let value = database.decrby("key", 2).await?;
    assert_eq!(-2, value);

    database.set("key", "12").await?;

    let value = database.decrby("key", 2).await?;
    assert_eq!(10, value);

    database.set("key", "value").await?;

    let result = database.decrby("key", 2).await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_and_set() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database.set("key", "value").await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_ex() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    let value: String = database.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pex() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    let value: String = database.getex("key", GetExOptions::Px(1000)).await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_exat() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    let value: String = database.getex("key", GetExOptions::Exat(time)).await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pxat() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    let value: String = database
        .getex("key", GetExOptions::Pxat(time as u64))
        .await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_persist() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    let value: String = database.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let value: String = database.getex("key", GetExOptions::Persist).await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getrange() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let value: String = database.getrange("key", 1, 3).await?;
    assert_eq!("alu", value);

    let value: String = database.getrange("key", 1, -3).await?;
    assert_eq!("al", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getset() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let value: String = database.getset("key", "newvalue").await?;
    assert_eq!("value", value);

    database.del("key").await?;

    let value: Value = database.getset("key", "newvalue").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incr() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let value = database.incr("key").await?;
    assert_eq!(1, value);

    database.set("key", "12").await?;

    let value = database.incr("key").await?;
    assert_eq!(13, value);

    database.set("key", "value").await?;

    let result = database.incr("key").await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrby() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let value = database.incrby("key", 2).await?;
    assert_eq!(2, value);

    database.set("key", "12").await?;

    let value = database.incrby("key", 2).await?;
    assert_eq!(14, value);

    database.set("key", "value").await?;

    let result = database.incrby("key", 2).await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrbyfloat() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database.set("key", "10.50").await?;

    let value = database.incrbyfloat("key", 0.1).await?;
    assert_eq!(10.6, value);

    let value = database.incrbyfloat("key", -5f64).await?;
    assert_eq!(5.6, value);

    database.set("key", "5.0e3").await?;

    let value = database.incrbyfloat("key", 2.0e2f64).await?;
    assert_eq!(5200f64, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lcs() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2"]).await?;

    database
        .mset([("key1", "ohmytext"), ("key2", "mynewtext")])
        .await?;

    let result: String = database.lcs("key1", "key2").await?;
    assert_eq!("mytext", result);

    let result = database.lcs_len("key1", "key2").await?;
    assert_eq!(6, result);

    let result = database.lcs_idx("key1", "key2", None, false).await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(((4, 7), (5, 8), None), result.matches[0]);
    assert_eq!(((2, 3), (0, 1), None), result.matches[1]);

    let result = database.lcs_idx("key1", "key2", Some(4), false).await?;
    assert_eq!(6, result.len);
    assert_eq!(1, result.matches.len());
    assert_eq!(((4, 7), (5, 8), None), result.matches[0]);

    let result = database.lcs_idx("key1", "key2", None, true).await?;
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3", "key4"]).await?;

    database
        .mset([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;

    let values: Vec<Option<String>> = database.mget(["key1", "key2", "key3", "key4"]).await?;
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3", "key4"]).await?;

    let success = database
        .msetnx([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;
    assert!(success);

    let values: Vec<Option<String>> = database.mget(["key1", "key2", "key3", "key4"]).await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    let success = database
        .msetnx([("key1", "value1"), ("key4", "value4")])
        .await?;
    assert!(!success);

    let values: Vec<Option<String>> = database.mget(["key1", "key4"]).await?;
    assert_eq!(2, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert_eq!(values[1], None);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn psetex() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.psetex("key", 1000, "value").await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn set_with_options() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // EX
    database
        .set_with_options("key", "value", None, Some(SetExpiration::Ex(1)), false)
        .await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    // PX
    database
        .set_with_options("key", "value", None, Some(SetExpiration::Px(1000)), false)
        .await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    // EXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    database
        .set_with_options("key", "value", None, Some(SetExpiration::Exat(time)), false)
        .await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    // PXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    database
        .set_with_options(
            "key",
            "value",
            None,
            Some(SetExpiration::Pxat(time as u64)),
            false,
        )
        .await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    // NX
    database.del("key").await?;
    let result = database
        .set_with_options("key", "value", Some(SetCondition::NX), None, false)
        .await?;
    assert!(result);
    let result = database
        .set_with_options("key", "value", Some(SetCondition::NX), None, false)
        .await?;
    assert!(!result);

    // XX
    database.del("key").await?;
    let result = database
        .set_with_options("key", "value", Some(SetCondition::XX), None, false)
        .await?;
    assert!(!result);
    database.set("key", "value").await?;
    let result = database
        .set_with_options("key", "value", Some(SetCondition::XX), None, false)
        .await?;
    assert!(result);

    // GET
    database.del("key").await?;
    let result: Option<String> = database
        .set_get_with_options("key", "value", None, None, false)
        .await?;
    assert!(result.is_none());
    database.set("key", "value").await?;
    let result: String = database
        .set_get_with_options("key", "value1", None, None, false)
        .await?;
    assert_eq!("value", result);
    let value: String = database.get("key").await?;
    assert_eq!("value1", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setex() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.setex("key", 1, "value").await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let value: Value = database.get("key").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setnx() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let result = database.setnx("key", "value").await?;
    let value: String = database.get("key").await?;
    assert!(result);
    assert_eq!("value", value);

    let result = database.setnx("key", "value1").await?;
    assert!(!result);
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setrange() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database.set("key", "Hello World").await?;

    let new_len = database.setrange("key", 6, "Redis").await?;
    assert_eq!(11, new_len);

    let value: String = database.get("key").await?;
    assert_eq!("Hello Redis", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn strlen() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "Hello World").await?;

    let len = database.strlen("key").await?;
    assert_eq!(11, len);

    let len = database.strlen("nonexisting").await?;
    assert_eq!(0, len);

    Ok(())
}
