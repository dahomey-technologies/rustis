use crate::{
    resp::{BulkString, Value},
    tests::get_default_addr,
    Connection, ConnectionCommandResult, ExpireOption, FlushingMode, GenericCommands, ListCommands,
    RestoreOptions, Result, ScanOptions, ServerCommands, SetCommands, SortOptions, StringCommands, ConnectionCommands,
};
use serial_test::serial;
use std::{collections::HashSet, time::SystemTime};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn copy() -> Result<()> {
    let connection0 = Connection::connect(get_default_addr()).await?;
    let connection1 = Connection::connect(get_default_addr()).await?;
    connection1.select(1).send().await?;

    // cleanup
    connection0.del(["key", "key1"]).send().await?;
    connection1.del(["key", "key1"]).send().await?;

    connection0.set("key", "value").send().await?;

    let result = connection0.copy("key", "key1", None, false).send().await?;
    assert!(result);
    let value: String = connection0.get("key1").send().await?;
    assert_eq!("value", value);

    connection0.set("key", "new_value").send().await?;
    let result = connection0.copy("key", "key1", None, false).send().await?;
    assert!(!result);
    let value: String = connection0.get("key1").send().await?;
    assert_eq!("value", value);

    let result = connection0.copy("key", "key1", None, true).send().await?;
    assert!(result);
    let value: String = connection0.get("key1").send().await?;
    assert_eq!("new_value", value);

    let result = connection0.copy("key", "key", Some(1), false).send().await?;
    assert!(result);
    let value: String = connection1.get("key").send().await?;
    assert_eq!("new_value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn del() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key1", "value1").send().await?;
    connection.set("key2", "value2").send().await?;
    connection.set("key3", "value3").send().await?;

    let deleted = connection.del("key1").send().await?;
    assert_eq!(1, deleted);

    let deleted = connection.del(["key1", "key2", "key3"]).send().await?;
    assert_eq!(2, deleted);

    let deleted = connection.del("key1").send().await?;
    assert_eq!(0, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn dump() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let dump = connection.dump("key").send().await?;
    assert!(dump.serialized_value.len() > 0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn exists() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2"]).send().await?;

    connection.set("key1", "value1").send().await?;

    let result = connection.exists("key1").send().await?;
    assert_eq!(1, result);

    let result = connection.exists(["key1", "key2"]).send().await?;
    assert_eq!(1, result);

    let result = connection.exists("key2").send().await?;
    assert_eq!(0, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expire() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // no option
    connection.set("key", "value").send().await?;
    let result = connection
        .expire("key", 10, ExpireOption::None)
        .send()
        .await?;
    assert!(result);
    assert_eq!(10, connection.ttl("key").send().await?);

    // xx
    connection.set("key", "value").send().await?;
    let result = connection
        .expire("key", 10, ExpireOption::Xx)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(-1, connection.ttl("key").send().await?);

    // nx
    let result = connection
        .expire("key", 10, ExpireOption::Nx)
        .send()
        .await?;
    assert!(result);
    assert_eq!(10, connection.ttl("key").send().await?);

    // gt
    let result = connection.expire("key", 5, ExpireOption::Gt).send().await?;
    assert!(!result);
    assert_eq!(10, connection.ttl("key").send().await?);
    let result = connection
        .expire("key", 15, ExpireOption::Gt)
        .send()
        .await?;
    assert!(result);
    assert_eq!(15, connection.ttl("key").send().await?);

    // lt
    let result = connection
        .expire("key", 20, ExpireOption::Lt)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(15, connection.ttl("key").send().await?);
    let result = connection.expire("key", 5, ExpireOption::Lt).send().await?;
    assert!(result);
    assert_eq!(5, connection.ttl("key").send().await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expireat() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();

    // no option
    connection.set("key", "value").send().await?;
    let result = connection
        .expireat("key", now + 10, ExpireOption::default())
        .send()
        .await?;
    assert!(result);
    let ttl = connection.ttl("key").send().await?;
    assert!(9 <= ttl && ttl <= 10);

    // xx
    connection.set("key", "value").send().await?;
    let result = connection
        .expireat("key", now + 10, ExpireOption::Xx)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(-1, connection.ttl("key").send().await?);

    // nx
    let result = connection
        .expireat("key", now + 10, ExpireOption::Nx)
        .send()
        .await?;
    assert!(result);
    assert!(9 <= ttl && ttl <= 10);

    // gt
    let result = connection
        .expireat("key", now + 5, ExpireOption::Gt)
        .send()
        .await?;
    assert!(!result);
    assert!(9 <= ttl && ttl <= 10);
    let result = connection
        .expireat("key", now + 15, ExpireOption::Gt)
        .send()
        .await?;
    assert!(result);
    let ttl = connection.ttl("key").send().await?;
    assert!(14 <= ttl && ttl <= 15);

    // lt
    let result = connection
        .expireat("key", now + 20, ExpireOption::Lt)
        .send()
        .await?;
    assert!(!result);
    let ttl = connection.ttl("key").send().await?;
    assert!(14 <= ttl && ttl <= 15);
    let result = connection
        .expireat("key", now + 5, ExpireOption::Lt)
        .send()
        .await?;
    assert!(result);
    let ttl = connection.ttl("key").send().await?;
    assert!(4 <= ttl && ttl <= 5);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expiretime() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;
    assert!(
        connection
            .expireat("key", 33177117420, ExpireOption::default())
            .send()
            .await?
    );
    let time = connection.expiretime("key").send().await?;
    assert_eq!(time, 33177117420);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn keys() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.flushdb(FlushingMode::Sync).send().await?;
    connection
        .mset([
            ("firstname", "Jack"),
            ("lastname", "Stuntman"),
            ("age", "35"),
        ])
        .send()
        .await?;

    let keys: HashSet<String> = connection.keys("*name*").send().await?;
    assert_eq!(2, keys.len());
    assert!(keys.contains("firstname"));
    assert!(keys.contains("lastname"));

    let keys: HashSet<String> = connection.keys("a??").send().await?;
    assert_eq!(1, keys.len());
    assert!(keys.contains("age"));

    let keys: HashSet<String> = connection.keys("*").send().await?;
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
    let connection0 = Connection::connect(get_default_addr()).await?;
    let connection1 = Connection::connect(get_default_addr()).await?;
    connection1.select(1).send().await?;

    // cleanup
    connection0.del("key").send().await?;
    connection1.del("key").send().await?;

    connection0.set("key", "value").send().await?;
    connection0.move_("key", 1).send().await?;
    assert_eq!(0, connection0.exists("key").send().await?);
    assert_eq!(1, connection1.exists("key").send().await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_encoding() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.del(["key1", "key2", "unknown"]).send().await?;
    connection.set("key1", "value").send().await?;
    connection.set("key2", "12").send().await?;

    let encoding: String = connection.object_encoding("key1").send().await?;
    assert_eq!("embstr", encoding);

    let encoding: String = connection.object_encoding("key2").send().await?;
    assert_eq!("int", encoding);

    let encoding: String = connection.object_encoding("unknown").send().await?;
    assert_eq!("", encoding);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_freq() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.del("key").send().await?;
    connection.set("key", "value").send().await?;

    let frequency = connection.object_freq("key").send().await;
    // ERR An LFU maxmemory policy is not selected, access frequency not tracked.
    assert!(frequency.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_idle_time() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.del("key").send().await?;
    connection.set("key", "value").send().await?;

    let idle_time = connection.object_idle_time("key").send().await?;
    assert!(idle_time < 1);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_refcount() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.del("key").send().await?;
    connection.set("key", "value").send().await?;

    let refcount = connection.object_refcount("key").send().await?;
    assert_eq!(1, refcount);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn persist() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;
    assert!(
        connection
            .expire("key", 10, ExpireOption::None)
            .send()
            .await?
    );
    assert_eq!(10, connection.ttl("key").send().await?);
    assert!(connection.persist("key").send().await?);
    assert_eq!(-1, connection.ttl("key").send().await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpire() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // no option
    connection.set("key", "value").send().await?;
    let result = connection
        .pexpire("key", 10000, ExpireOption::default())
        .send()
        .await?;
    assert!(result);
    assert_eq!(10, connection.ttl("key").send().await?);

    // xx
    connection.set("key", "value").send().await?;
    let result = connection
        .pexpire("key", 10000, ExpireOption::Xx)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(-1, connection.ttl("key").send().await?);

    // nx
    let result = connection
        .pexpire("key", 10000, ExpireOption::Nx)
        .send()
        .await?;
    assert!(result);
    assert_eq!(10, connection.ttl("key").send().await?);

    // gt
    let result = connection
        .pexpire("key", 5000, ExpireOption::Gt)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(10, connection.ttl("key").send().await?);
    let result = connection
        .pexpire("key", 15000, ExpireOption::Gt)
        .send()
        .await?;
    assert!(result);
    assert_eq!(15, connection.ttl("key").send().await?);

    // lt
    let result = connection
        .pexpire("key", 20000, ExpireOption::Lt)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(15, connection.ttl("key").send().await?);
    let result = connection
        .pexpire("key", 5000, ExpireOption::Lt)
        .send()
        .await?;
    assert!(result);
    assert_eq!(5, connection.ttl("key").send().await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpireat() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis() as u64;

    // no option
    connection.set("key", "value").send().await?;
    let result = connection
        .pexpireat("key", now + 10000, ExpireOption::default())
        .send()
        .await?;
    assert!(result);
    assert!(10000 >= connection.pttl("key").send().await?);

    // xx
    connection.set("key", "value").send().await?;
    let result = connection
        .pexpireat("key", now + 10000, ExpireOption::Xx)
        .send()
        .await?;
    assert!(!result);
    assert_eq!(-1, connection.pttl("key").send().await?);

    // nx
    let result = connection
        .pexpireat("key", now + 10000, ExpireOption::Nx)
        .send()
        .await?;
    assert!(result);
    assert!(10000 >= connection.pttl("key").send().await?);

    // gt
    let result = connection
        .pexpireat("key", now + 5000, ExpireOption::Gt)
        .send()
        .await?;
    assert!(!result);
    assert!(10000 >= connection.pttl("key").send().await?);
    let result = connection
        .pexpireat("key", now + 15000, ExpireOption::Gt)
        .send()
        .await?;
    assert!(result);
    assert!(15000 >= connection.pttl("key").send().await?);

    // lt
    let result = connection
        .pexpireat("key", now + 20000, ExpireOption::Lt)
        .send()
        .await?;
    assert!(!result);
    assert!(20000 >= connection.pttl("key").send().await?);
    let result = connection
        .pexpireat("key", now + 5000, ExpireOption::Lt)
        .send()
        .await?;
    assert!(result);
    assert!(5000 >= connection.pttl("key").send().await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpiretime() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;
    assert!(
        connection
            .pexpireat("key", 33177117420000, ExpireOption::default())
            .send()
            .await?
    );
    let time = connection.pexpiretime("key").send().await?;
    assert_eq!(time, 33177117420000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn randomkey() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.flushdb(FlushingMode::Sync).send().await?;
    connection.set("key1", "value1").send().await?;
    connection.set("key2", "value2").send().await?;
    connection.set("key3", "value3").send().await?;

    let key: String = connection.randomkey().send().await?;
    assert!(["key1", "key2", "key3"].contains(&key.as_str()));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rename() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.flushdb(FlushingMode::Sync).send().await?;
    connection.set("key1", "value1").send().await?;

    connection.rename("key1", "key2").send().await?;
    let value: Value = connection.get("key1").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));
    let value: String = connection.get("key2").send().await?;
    assert_eq!("value1", value);

    let result = connection.rename("unknown", "key2").send().await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn renamenx() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.flushdb(FlushingMode::Sync).send().await?;
    connection.set("key1", "value1").send().await?;

    let success = connection.renamenx("key1", "key2").send().await?;
    assert!(success);

    connection.set("key1", "value1").send().await?;
    let success = connection.renamenx("key1", "key2").send().await?;
    assert!(!success);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn restore() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key", "value").send().await?;

    let dump = connection.dump("key").send().await?;
    connection.del("key").send().await?;
    connection
        .restore("key", 0, dump.serialized_value, RestoreOptions::default())
        .send()
        .await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn scan() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.flushdb(FlushingMode::Sync).send().await?;

    connection.set("key1", "value").send().await?;
    connection.set("key2", "value").send().await?;
    connection.set("key3", "value").send().await?;

    let keys: (u64, HashSet<String>) = connection.scan(0, ScanOptions::default()).send().await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    connection.flushdb(FlushingMode::Sync).send().await?;

    connection
        .rpush("key", ["member3", "member1", "member2"])
        .send()
        .await?;

    let values: Vec<String> = connection
        .sort("key", SortOptions::default().alpha())
        .send()
        .await?;
    assert_eq!(3, values.len());
    assert_eq!("member1".to_owned(), values[0]);
    assert_eq!("member2".to_owned(), values[1]);
    assert_eq!("member3".to_owned(), values[2]);

    let len = connection
        .sort_and_store("key", "out", SortOptions::default().alpha())
        .send()
        .await?;
    assert_eq!(3, len);

    let values: Vec<String> = connection.lrange("out", 0, -1).send().await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key1", "Hello").send().await?;
    connection.set("key2", "World").send().await?;

    let num_keys = connection.touch(["key1", "key2"]).send().await?;
    assert_eq!(2, num_keys);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn type_() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2", "key3"]).send().await?;

    connection.set("key1", "value").send().await?;
    connection.lpush("key2", "value").send().await?;
    connection.sadd("key3", "value").send().await?;

    let result = connection.type_("key1").send().await?;
    assert_eq!(&result, "string");

    let result = connection.type_("key2").send().await?;
    assert_eq!(&result, "list");

    let result = connection.type_("key3").send().await?;
    assert_eq!(&result, "set");

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unlink() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.set("key1", "value1").send().await?;
    connection.set("key2", "value2").send().await?;
    connection.set("key3", "value3").send().await?;

    let unlinked = connection.unlink("key1").send().await?;
    assert_eq!(1, unlinked);

    let unlinked = connection.unlink(["key1", "key2", "key3"]).send().await?;
    assert_eq!(2, unlinked);

    Ok(())
}
