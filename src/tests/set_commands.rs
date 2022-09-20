use std::collections::HashSet;

use crate::{
    tests::get_default_addr, ConnectionMultiplexer, DatabaseCommandResult, GenericCommands, Result,
    SetCommands, NONE_ARG,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sadd() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    let len = database.sadd("key", ["value1", "value2", "value3"]).send().await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn scard() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;
    let len = database.scard("key").send().await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sdiff() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let members: HashSet<String> = database.sdiff(["key1", "key2", "key3"]).send().await?;
    assert_eq!(2, members.len());
    assert!(members.contains("b"));
    assert!(members.contains("d"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sdiffstore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3", "key4"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = database
        .sdiffstore("key4", ["key1", "key2", "key3"])
        .send().await?;
    assert_eq!(2, len);

    let members: HashSet<String> = database.smembers("key4").send().await?;
    assert_eq!(2, members.len());
    assert!(members.contains("b"));
    assert!(members.contains("d"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sinter() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let members: HashSet<String> = database.sinter(["key1", "key2", "key3"]).send().await?;
    assert_eq!(1, members.len());
    assert!(members.contains("c"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sintercard() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = database.sintercard(["key1", "key2", "key3"], 0).send().await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sinterstore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3", "key4"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = database
        .sinterstore("key4", ["key1", "key2", "key3"])
        .send().await?;
    assert_eq!(1, len);

    let members: HashSet<String> = database.smembers("key4").send().await?;
    assert_eq!(1, members.len());
    assert!(members.contains("c"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sismember() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let result = database.sismember("key", "value1").send().await?;
    assert!(result);

    let result = database.sismember("key", "value4").send().await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn smembers() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let members: HashSet<String> = database.smembers("key").send().await?;
    assert_eq!(3, members.len());
    assert!(members.contains("value1"));
    assert!(members.contains("value2"));
    assert!(members.contains("value3"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn smismember() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let result = database.smismember("key", ["value1", "value4"]).send().await?;
    assert_eq!(2, result.len());
    assert!(result[0]);
    assert!(!result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn smove() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2"]).send().await?;

    database
        .sadd("key1", ["value1", "value2", "value3"])
        .send().await?;
    database
        .sadd("key2", ["value4", "value5", "value6"])
        .send().await?;

    let result = database.smove("key1", "key2", "value3").send().await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn spop() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let result: HashSet<String> = database.spop("key", 2).send().await?;
    assert_eq!(2, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn srandmember() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let result: HashSet<String> = database.srandmember("key", 2).send().await?;
    assert_eq!(2, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn srem() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let result = database.srem("key", ["value1", "value2", "value4"]).send().await?;
    assert_eq!(2, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sscan() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").send().await?;

    database.sadd("key", ["value1", "value2", "value3"]).send().await?;

    let result: (u64, Vec<String>) = database.sscan("key", 0, NONE_ARG, None).send().await?;
    assert_eq!(0, result.0);
    assert_eq!(3, result.1.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sunion() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let members: HashSet<String> = database.sunion(["key1", "key2", "key3"]).send().await?;
    assert_eq!(5, members.len());
    assert!(members.contains("a"));
    assert!(members.contains("b"));
    assert!(members.contains("c"));
    assert!(members.contains("d"));
    assert!(members.contains("e"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sunionstore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "key3", "key4"]).send().await?;

    database.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    database.sadd("key2", "c").send().await?;
    database.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = database
        .sunionstore("key4", ["key1", "key2", "key3"])
        .send().await?;
    assert_eq!(5, len);

    let members: HashSet<String> = database.smembers("key4").send().await?;
    assert_eq!(5, members.len());
    assert!(members.contains("a"));
    assert!(members.contains("b"));
    assert!(members.contains("c"));
    assert!(members.contains("d"));
    assert!(members.contains("e"));

    Ok(())
}
