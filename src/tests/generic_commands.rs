use crate::{
    resp::{BulkString, Value},
    tests::get_default_addr,
    ConnectionMultiplexer, ExpireOption, FlushingMode, GenericCommands, ListCommands, Result,
    ServerCommands, SetCommands, SortOptions, StringCommands, NONE_ARG,
};
use serial_test::serial;
use std::{collections::HashSet, time::SystemTime};

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

    let result = database0.copy("key", "key1", None, false).await?;
    assert!(result);
    let value: String = database0.get("key1").await?;
    assert_eq!("value", value);

    database0.set("key", "new_value").await?;
    let result = database0.copy("key", "key1", None, false).await?;
    assert!(!result);
    let value: String = database0.get("key1").await?;
    assert_eq!("value", value);

    let result = database0.copy("key", "key1", None, true).await?;
    assert!(result);
    let value: String = database0.get("key1").await?;
    assert_eq!("new_value", value);

    let result = database0.copy("key", "key", Some(1), false).await?;
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

    database.set("key1", "value1").await?;
    database.set("key2", "value2").await?;
    database.set("key3", "value3").await?;

    let deleted = database.del("key1").await?;
    assert_eq!(1, deleted);

    let deleted = database.del(["key1", "key2", "key3"]).await?;
    assert_eq!(2, deleted);

    let deleted = database.del("key1").await?;
    assert_eq!(0, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn dump() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let dump = database.dump("key").await?;
    assert!(dump.len() > 0);

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
    let result = database.expire("key", 10, None).await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // xx
    database.set("key", "value").await?;
    let result = database.expire("key", 10, Some(ExpireOption::Xx)).await?;
    assert!(!result);
    assert_eq!(-1, database.ttl("key").await?);

    // nx
    let result = database.expire("key", 10, Some(ExpireOption::Nx)).await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // gt
    let result = database.expire("key", 5, Some(ExpireOption::Gt)).await?;
    assert!(!result);
    assert_eq!(10, database.ttl("key").await?);
    let result = database.expire("key", 15, Some(ExpireOption::Gt)).await?;
    assert!(result);
    assert_eq!(15, database.ttl("key").await?);

    // lt
    let result = database.expire("key", 20, Some(ExpireOption::Lt)).await?;
    assert!(!result);
    assert_eq!(15, database.ttl("key").await?);
    let result = database.expire("key", 5, Some(ExpireOption::Lt)).await?;
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
    let result = database.expireat("key", now + 10, None).await?;
    assert!(result);
    let ttl = database.ttl("key").await?;
    assert!(9 <= ttl && ttl <= 10);

    // xx
    database.set("key", "value").await?;
    let result = database
        .expireat("key", now + 10, Some(ExpireOption::Xx))
        .await?;
    assert!(!result);
    assert_eq!(-1, database.ttl("key").await?);

    // nx
    let result = database
        .expireat("key", now + 10, Some(ExpireOption::Nx))
        .await?;
    assert!(result);
    assert!(9 <= ttl && ttl <= 10);

    // gt
    let result = database
        .expireat("key", now + 5, Some(ExpireOption::Gt))
        .await?;
    assert!(!result);
    assert!(9 <= ttl && ttl <= 10);
    let result = database
        .expireat("key", now + 15, Some(ExpireOption::Gt))
        .await?;
    assert!(result);
    let ttl = database.ttl("key").await?;
    assert!(14 <= ttl && ttl <= 15);

    // lt
    let result = database
        .expireat("key", now + 20, Some(ExpireOption::Lt))
        .await?;
    assert!(!result);
    let ttl = database.ttl("key").await?;
    assert!(14 <= ttl && ttl <= 15);
    let result = database
        .expireat("key", now + 5, Some(ExpireOption::Lt))
        .await?;
    assert!(result);
    let ttl = database.ttl("key").await?;
    assert!(4 <= ttl && ttl <= 5);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expiretime() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    assert!(database.expireat("key", 33177117420, None).await?);
    let time = database.expiretime("key").await?;
    assert_eq!(time, 33177117420);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn keys() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;
    database
        .mset([
            ("firstname", "Jack"),
            ("lastname", "Stuntman"),
            ("age", "35"),
        ])
        .await?;

    let keys: HashSet<String> = database.keys("*name*").await?;
    assert_eq!(2, keys.len());
    assert!(keys.contains("firstname"));
    assert!(keys.contains("lastname"));

    let keys: HashSet<String> = database.keys("a??").await?;
    assert_eq!(1, keys.len());
    assert!(keys.contains("age"));

    let keys: HashSet<String> = database.keys("*").await?;
    assert_eq!(3, keys.len());
    assert!(keys.contains("firstname"));
    assert!(keys.contains("lastname"));
    assert!(keys.contains("age"));

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
async fn object_encoding() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del(["key1", "key2", "unknown"]).await?;
    database.set("key1", "value").await?;
    database.set("key2", "12").await?;

    let encoding: String = database.object_encoding("key1").await?;
    assert_eq!("embstr", encoding);

    let encoding: String = database.object_encoding("key2").await?;
    assert_eq!("int", encoding);

    let encoding: String = database.object_encoding("unknown").await?;
    assert_eq!("", encoding);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_freq() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del("key").await?;
    database.set("key", "value").await?;

    let frequency = database.object_freq("key").await;
    // ERR An LFU maxmemory policy is not selected, access frequency not tracked.
    assert!(frequency.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_idle_time() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del("key").await?;
    database.set("key", "value").await?;

    let idle_time = database.object_idle_time("key").await?;
    assert!(idle_time < 1);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_refcount() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del("key").await?;
    database.set("key", "value").await?;

    let refcount = database.object_refcount("key").await?;
    assert_eq!(1, refcount);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn persist() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    assert!(database.expire("key", 10, None).await?);
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
    let result = database.pexpire("key", 10000, None).await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // xx
    database.set("key", "value").await?;
    let result = database
        .pexpire("key", 10000, Some(ExpireOption::Xx))
        .await?;
    assert!(!result);
    assert_eq!(-1, database.ttl("key").await?);

    // nx
    let result = database
        .pexpire("key", 10000, Some(ExpireOption::Nx))
        .await?;
    assert!(result);
    assert_eq!(10, database.ttl("key").await?);

    // gt
    let result = database
        .pexpire("key", 5000, Some(ExpireOption::Gt))
        .await?;
    assert!(!result);
    assert_eq!(10, database.ttl("key").await?);
    let result = database
        .pexpire("key", 15000, Some(ExpireOption::Gt))
        .await?;
    assert!(result);
    assert_eq!(15, database.ttl("key").await?);

    // lt
    let result = database
        .pexpire("key", 20000, Some(ExpireOption::Lt))
        .await?;
    assert!(!result);
    assert_eq!(15, database.ttl("key").await?);
    let result = database
        .pexpire("key", 5000, Some(ExpireOption::Lt))
        .await?;
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
    let result = database.pexpireat("key", now + 10000, None).await?;
    assert!(result);
    assert!(10000 >= database.pttl("key").await?);

    // xx
    database.set("key", "value").await?;
    let result = database
        .pexpireat("key", now + 10000, Some(ExpireOption::Xx))
        .await?;
    assert!(!result);
    assert_eq!(-1, database.pttl("key").await?);

    // nx
    let result = database
        .pexpireat("key", now + 10000, Some(ExpireOption::Nx))
        .await?;
    assert!(result);
    assert!(10000 >= database.pttl("key").await?);

    // gt
    let result = database
        .pexpireat("key", now + 5000, Some(ExpireOption::Gt))
        .await?;
    assert!(!result);
    assert!(10000 >= database.pttl("key").await?);
    let result = database
        .pexpireat("key", now + 15000, Some(ExpireOption::Gt))
        .await?;
    assert!(result);
    assert!(15000 >= database.pttl("key").await?);

    // lt
    let result = database
        .pexpireat("key", now + 20000, Some(ExpireOption::Lt))
        .await?;
    assert!(!result);
    assert!(20000 >= database.pttl("key").await?);
    let result = database
        .pexpireat("key", now + 5000, Some(ExpireOption::Lt))
        .await?;
    assert!(result);
    assert!(5000 >= database.pttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpiretime() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;
    assert!(database.pexpireat("key", 33177117420000, None).await?);
    let time = database.pexpiretime("key").await?;
    assert_eq!(time, 33177117420000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn randomkey() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;
    database.set("key1", "value1").await?;
    database.set("key2", "value2").await?;
    database.set("key3", "value3").await?;

    let key: String = database.randomkey().await?;
    assert!(["key1", "key2", "key3"].contains(&key.as_str()));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rename() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;
    database.set("key1", "value1").await?;

    database.rename("key1", "key2").await?;
    let value: Value = database.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));
    let value: String = database.get("key2").await?;
    assert_eq!("value1", value);

    let result = database.rename("unknown", "key2").await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn renamenx() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;
    database.set("key1", "value1").await?;

    let success = database.renamenx("key1", "key2").await?;
    assert!(success);

    database.set("key1", "value1").await?;
    let success = database.renamenx("key1", "key2").await?;
    assert!(!success);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn restore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let dump = database.dump("key").await?;
    database.del("key").await?;
    database
        .restore("key", 0, dump, false, false, None, None)
        .await?;
    let value: String = database.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn scan() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;

    database.set("key1", "value").await?;
    database.set("key2", "value").await?;
    database.set("key3", "value").await?;

    let keys: (u64, HashSet<String>) = database.scan(0, NONE_ARG, None, NONE_ARG).await?;
    assert_eq!(3, keys.1.len());
    assert!(keys.1.contains("key1"));
    assert!(keys.1.contains("key2"));
    assert!(keys.1.contains("key3"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sort() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;

    database
        .rpush("key", ["member3", "member1", "member2"])
        .await?;

    let values: Vec<String> = database
        .sort("key", <SortOptions>::default().alpha())
        .await?;
    assert_eq!(3, values.len());
    assert_eq!("member1".to_owned(), values[0]);
    assert_eq!("member2".to_owned(), values[1]);
    assert_eq!("member3".to_owned(), values[2]);

    let len = database
        .sort_and_store("key", "out", <SortOptions>::default().alpha())
        .await?;
    assert_eq!(3, len);

    let values: Vec<String> = database.lrange("out", 0, -1).await?;
    assert_eq!(3, values.len());
    assert_eq!("member1".to_owned(), values[0]);
    assert_eq!("member2".to_owned(), values[1]);
    assert_eq!("member3".to_owned(), values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn touch() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key1", "Hello").await?;
    database.set("key2", "World").await?;

    let num_keys = database.touch(["key1", "key2"]).await?;
    assert_eq!(2, num_keys);

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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unlink() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key1", "value1").await?;
    database.set("key2", "value2").await?;
    database.set("key3", "value3").await?;

    let unlinked = database.unlink("key1").await?;
    assert_eq!(1, unlinked);

    let unlinked = database.unlink(["key1", "key2", "key3"]).await?;
    assert_eq!(2, unlinked);

    Ok(())
}
