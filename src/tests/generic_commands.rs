use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, ListCommands, Result,
    SetCommands, StringCommands,
};
use serial_test::serial;
use std::time::SystemTime;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn copy() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database0 = connection.get_database(0);
    let database1 = connection.get_database(1);

    // cleanup
    database0.del(["key", "key1"]).await?;
    database1.del(["key", "key1"]).await?;

    database0.set("key", "value").await?;

    let result = database0.copy("key", "key1").execute().await?;
    assert!(result);
    let value: String = database0.get("key1").await?;
    assert_eq!("value", value);

    database0.set("key", "new_value").await?;
    let result = database0.copy("key", "key1").execute().await?;
    assert!(!result);
    let value: String = database0.get("key1").await?;
    assert_eq!("value", value);

    let result = database0.copy("key", "key1").replace().execute().await?;
    assert!(result);
    let value: String = database0.get("key1").await?;
    assert_eq!("new_value", value);

    let result = database0.copy("key", "key").db(1).execute().await?;
    assert!(result);
    let value: String = database1.get("key").await?;
    assert_eq!("new_value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn del() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let deleted = database.del("key").await?;
    assert_eq!(1, deleted);

    let deleted = database.del("key").await?;
    assert_eq!(0, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn del_multiple() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key1", "value1").await?;
    database.set("key2", "value2").await?;
    database.set("key3", "value3").await?;

    let deleted = database.del("key1").await?;
    assert_eq!(1, deleted);

    let deleted = database.del(["key1", "key2", "key3"]).await?;
    assert_eq!(2, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn exists() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2"]).await?;

    database.set("key1", "value1").await?;

    let result = database.exists("key1").await?;
    assert_eq!(1, result);

    let result = database.exists(["key1", "key2"]).await?;
    assert_eq!(1, result);

    let result = database.exists("key2").await?;
    assert_eq!(0, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expire() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // no option
    database.set("key", "value").await?;
    let result = database.expire("key", 10).execute().await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // xx
    database.set("key", "value").await?;
    let result = database.expire("key", 10).xx().await?;
    assert!(!result);
    assert_eq!(-1, database.ttl("key").await?);

    // nx
    let result = database.expire("key", 10).nx().await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // gt
    let result = database.expire("key", 5).gt().await?;
    assert!(!result);
    assert_eq!(10, database.ttl("key").await?);
    let result = database.expire("key", 15).gt().await?;
    assert!(result);
    assert_eq!(15, database.ttl("key").await?);

    // lt
    let result = database.expire("key", 20).lt().await?;
    assert!(!result);
    assert_eq!(15, database.ttl("key").await?);
    let result = database.expire("key", 5).lt().await?;
    assert!(result);
    assert_eq!(5, database.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expireat() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();

    // no option
    database.set("key", "value").await?;
    let result = database.expireat("key", now + 10).execute().await?;
    assert!(result);
    let ttl = database.ttl("key").await?;
    assert!(9 <= ttl && ttl <= 10);

    // xx
    database.set("key", "value").await?;
    let result = database.expireat("key", now + 10).xx().await?;
    assert!(!result);
    assert_eq!(-1, database.ttl("key").await?);

    // nx
    let result = database.expireat("key", now + 10).nx().await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // gt
    let result = database.expireat("key", now + 5).gt().await?;
    assert!(!result);
    assert_eq!(10, database.ttl("key").await?);
    let result = database.expireat("key", now + 15).gt().await?;
    assert!(result);
    assert_eq!(15, database.ttl("key").await?);

    // lt
    let result = database.expireat("key", now + 20).lt().await?;
    assert!(!result);
    assert_eq!(15, database.ttl("key").await?);
    let result = database.expireat("key", now + 5).lt().await?;
    assert!(result);
    assert_eq!(5, database.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expiretime() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    assert!(database.expireat("key", 33177117420).execute().await?);
    let time = database.expiretime("key").await?;
    assert_eq!(time, 33177117420);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn move_() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database0 = connection.get_database(0);
    let database1 = connection.get_database(1);

    // cleanup
    database0.del("key").await?;
    database1.del("key").await?;

    database0.set("key", "value").await?;
    database0.move_("key", 1).await?;
    assert_eq!(0, database0.exists("key").await?);
    assert_eq!(1, database1.exists("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn persist() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    assert!(database.expire("key", 10).execute().await?);
    assert_eq!(10, database.ttl("key").await?);
    assert!(database.persist("key").await?);
    assert_eq!(-1, database.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpire() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // no option
    database.set("key", "value").await?;
    let result = database.pexpire("key", 10000).execute().await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // xx
    database.set("key", "value").await?;
    let result = database.pexpire("key", 10000).xx().await?;
    assert!(!result);
    assert_eq!(-1, database.ttl("key").await?);

    // nx
    let result = database.pexpire("key", 10000).nx().await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // gt
    let result = database.pexpire("key", 5000).gt().await?;
    assert!(!result);
    assert_eq!(10, database.ttl("key").await?);
    let result = database.pexpire("key", 15000).gt().await?;
    assert!(result);
    assert_eq!(15, database.ttl("key").await?);

    // lt
    let result = database.pexpire("key", 20000).lt().await?;
    assert!(!result);
    assert_eq!(15, database.ttl("key").await?);
    let result = database.pexpire("key", 5000).lt().await?;
    assert!(result);
    assert_eq!(5, database.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpireat() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis() as u64;

    // no option
    database.set("key", "value").await?;
    let result = database.pexpireat("key", now + 10000).execute().await?;
    assert!(result);
    assert!(10000 > database.pttl("key").await?);

    // xx
    database.set("key", "value").await?;
    let result = database.pexpireat("key", now + 10000).xx().await?;
    assert!(!result);
    assert_eq!(-1, database.pttl("key").await?);

    // nx
    let result = database.pexpireat("key", now + 10000).nx().await?;
    assert!(result);
    assert!(10000 > database.pttl("key").await?);

    // gt
    let result = database.pexpireat("key", now + 5000).gt().await?;
    assert!(!result);
    assert!(10000 > database.pttl("key").await?);
    let result = database.pexpireat("key", now + 15000).gt().await?;
    assert!(result);
    assert!(15000 > database.pttl("key").await?);

    // lt
    let result = database.pexpireat("key", now + 20000).lt().await?;
    assert!(!result);
    assert!(20000 > database.pttl("key").await?);
    let result = database.pexpireat("key", now + 5000).lt().await?;
    assert!(result);
    assert!(5000 > database.pttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpiretime() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    assert!(database.pexpireat("key", 33177117420000).execute().await?);
    let time = database.pexpiretime("key").await?;
    assert_eq!(time, 33177117420000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn type_() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3"]).await?;

    database.set("key1", "value").await?;
    database.lpush("key2", "value").await?;
    database.sadd("key3", "value").await?;

    let result = database.type_("key1").await?;
    assert_eq!(&result, "string");

    let result = database.type_("key2").await?;
    assert_eq!(&result, "list");

    let result = database.type_("key3").await?;
    assert_eq!(&result, "set");

    Ok(())
}
