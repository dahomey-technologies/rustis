use crate::{
    resp::{BulkString, Value},
    tests::get_default_addr,
    Connection, ConnectionCommandResult, Error,
    GenericCommands, GetExOptions, Result, SetCondition, SetExpiration, StringCommands,
};
use serial_test::serial;
use std::time::{Duration, SystemTime};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn append() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let new_size = connection.append("key", "12").send().await?;
    assert_eq!(7, new_size);

    let value: String = connection.get("key").send().await?;
    assert_eq!("value12", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decr() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let value = connection.decr("key").send().await?;
    assert_eq!(-1, value);

    connection.set("key", "12").send().await?;

    let value = connection.decr("key").send().await?;
    assert_eq!(11, value);

    connection.set("key", "value").send().await?;

    let result = connection.decr("key").send().await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn decrby() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let value = connection.decrby("key", 2).send().await?;
    assert_eq!(-2, value);

    connection.set("key", "12").send().await?;

    let value = connection.decrby("key", 2).send().await?;
    assert_eq!(10, value);

    connection.set("key", "value").send().await?;

    let result = connection.decrby("key", 2).send().await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_and_set() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.set("key", "value").send().await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_ex() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;
    let value: String = connection.getex("key", GetExOptions::Ex(1)).send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pex() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;
    let value: String = connection.getex("key", GetExOptions::Px(1000)).send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_exat() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    let value: String = connection
        .getex("key", GetExOptions::Exat(time))
        .send()
        .await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_pxat() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    let value: String = connection
        .getex("key", GetExOptions::Pxat(time as u64))
        .send()
        .await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn get_persist() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;
    let value: String = connection.getex("key", GetExOptions::Ex(1)).send().await?;
    assert_eq!("value", value);

    let value: String = connection.getex("key", GetExOptions::Persist).send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert_eq!(-1, ttl);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getrange() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let value: String = connection.getrange("key", 1, 3).send().await?;
    assert_eq!("alu", value);

    let value: String = connection.getrange("key", 1, -3).send().await?;
    assert_eq!("al", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getset() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let value: String = connection.getset("key", "newvalue").send().await?;
    assert_eq!("value", value);

    connection.del("key").send().await?;

    let value: Value = connection.getset("key", "newvalue").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incr() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let value = connection.incr("key").send().await?;
    assert_eq!(1, value);

    connection.set("key", "12").send().await?;

    let value = connection.incr("key").send().await?;
    assert_eq!(13, value);

    connection.set("key", "value").send().await?;

    let result = connection.incr("key").send().await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrby() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let value = connection.incrby("key", 2).send().await?;
    assert_eq!(2, value);

    connection.set("key", "12").send().await?;

    let value = connection.incrby("key", 2).send().await?;
    assert_eq!(14, value);

    connection.set("key", "value").send().await?;

    let result = connection.incrby("key", 2).send().await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e == "ERR value is not an integer or out of range")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn incrbyfloat() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.set("key", "10.50").send().await?;

    let value = connection.incrbyfloat("key", 0.1).send().await?;
    assert_eq!(10.6, value);

    let value = connection.incrbyfloat("key", -5f64).send().await?;
    assert_eq!(5.6, value);

    connection.set("key", "5.0e3").send().await?;

    let value = connection.incrbyfloat("key", 2.0e2f64).send().await?;
    assert_eq!(5200f64, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lcs() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2"]).send().await?;

    connection
        .mset([("key1", "ohmytext"), ("key2", "mynewtext")])
        .send()
        .await?;

    let result: String = connection.lcs("key1", "key2").send().await?;
    assert_eq!("mytext", result);

    let result = connection.lcs_len("key1", "key2").send().await?;
    assert_eq!(6, result);

    let result = connection.lcs_idx("key1", "key2", None, false).send().await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(((4, 7), (5, 8), None), result.matches[0]);
    assert_eq!(((2, 3), (0, 1), None), result.matches[1]);

    let result = connection
        .lcs_idx("key1", "key2", Some(4), false)
        .send()
        .await?;
    assert_eq!(6, result.len);
    assert_eq!(1, result.matches.len());
    assert_eq!(((4, 7), (5, 8), None), result.matches[0]);

    let result = connection.lcs_idx("key1", "key2", None, true).send().await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection
        .del(["key1", "key2", "key3", "key4"])
        .send()
        .await?;

    connection
        .mset([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .send()
        .await?;

    let values: Vec<Option<String>> = connection
        .mget(["key1", "key2", "key3", "key4"])
        .send()
        .await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection
        .del(["key1", "key2", "key3", "key4"])
        .send()
        .await?;

    let success = connection
        .msetnx([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .send()
        .await?;
    assert!(success);

    let values: Vec<Option<String>> = connection
        .mget(["key1", "key2", "key3", "key4"])
        .send()
        .await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    let success = connection
        .msetnx([("key1", "value1"), ("key4", "value4")])
        .send()
        .await?;
    assert!(!success);

    let values: Vec<Option<String>> = connection.mget(["key1", "key4"]).send().await?;
    assert_eq!(2, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert_eq!(values[1], None);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn psetex() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.psetex("key", 1000, "value").send().await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn set_with_options() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // EX
    connection
        .set_with_options(
            "key",
            "value",
            Default::default(),
            SetExpiration::Ex(1),
            false,
        )
        .send()
        .await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    // PX
    connection
        .set_with_options(
            "key",
            "value",
            Default::default(),
            SetExpiration::Px(1000),
            false,
        )
        .send()
        .await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    // EXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    connection
        .set_with_options(
            "key",
            "value",
            Default::default(),
            SetExpiration::Exat(time),
            false,
        )
        .send()
        .await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    // PXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    connection
        .set_with_options(
            "key",
            "value",
            Default::default(),
            SetExpiration::Pxat(time as u64),
            false,
        )
        .send()
        .await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    // NX
    connection.del("key").send().await?;
    let result = connection
        .set_with_options("key", "value", SetCondition::NX, Default::default(), false)
        .send()
        .await?;
    assert!(result);
    let result = connection
        .set_with_options("key", "value", SetCondition::NX, Default::default(), false)
        .send()
        .await?;
    assert!(!result);

    // XX
    connection.del("key").send().await?;
    let result = connection
        .set_with_options("key", "value", SetCondition::XX, Default::default(), false)
        .send()
        .await?;
    assert!(!result);
    connection.set("key", "value").send().await?;
    let result = connection
        .set_with_options("key", "value", SetCondition::XX, Default::default(), false)
        .send()
        .await?;
    assert!(result);

    // GET
    connection.del("key").send().await?;
    let result: Option<String> = connection
        .set_get_with_options(
            "key",
            "value",
            Default::default(),
            Default::default(),
            false,
        )
        .send()
        .await?;
    assert!(result.is_none());
    connection.set("key", "value").send().await?;
    let result: String = connection
        .set_get_with_options(
            "key",
            "value1",
            Default::default(),
            Default::default(),
            false,
        )
        .send()
        .await?;
    assert_eq!("value", result);
    let value: String = connection.get("key").send().await?;
    assert_eq!("value1", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setex() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.setex("key", 1, "value").send().await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    let ttl = connection.pttl("key").send().await?;
    assert!(ttl <= 1000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setnx() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let result = connection.setnx("key", "value").send().await?;
    let value: String = connection.get("key").send().await?;
    assert!(result);
    assert_eq!("value", value);

    let result = connection.setnx("key", "value1").send().await?;
    assert!(!result);
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setrange() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.set("key", "Hello World").send().await?;

    let new_len = connection.setrange("key", 6, "Redis").send().await?;
    assert_eq!(11, new_len);

    let value: String = connection.get("key").send().await?;
    assert_eq!("Hello Redis", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn strlen() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "Hello World").send().await?;

    let len = connection.strlen("key").send().await?;
    assert_eq!(11, len);

    let len = connection.strlen("nonexisting").send().await?;
    assert_eq!(0, len);

    Ok(())
}
