use std::collections::HashSet;

use crate::{tests::get_test_client, commands::{GenericCommands, SScanOptions, SetCommands}, Result};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sadd() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let len = client.sadd("key", ["value1", "value2", "value3"]).await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn scard() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;
    let len = client.scard("key").await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sdiff() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let members: HashSet<String> = client.sdiff(["key1", "key2", "key3"]).await?;
    assert_eq!(2, members.len());
    assert!(members.contains("b"));
    assert!(members.contains("d"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sdiffstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let len = client.sdiffstore("key4", ["key1", "key2", "key3"]).await?;
    assert_eq!(2, len);

    let members: HashSet<String> = client.smembers("key4").await?;
    assert_eq!(2, members.len());
    assert!(members.contains("b"));
    assert!(members.contains("d"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sinter() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let members: HashSet<String> = client.sinter(["key1", "key2", "key3"]).await?;
    assert_eq!(1, members.len());
    assert!(members.contains("c"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sintercard() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let len = client.sintercard(["key1", "key2", "key3"], 0).await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sinterstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let len = client.sinterstore("key4", ["key1", "key2", "key3"]).await?;
    assert_eq!(1, len);

    let members: HashSet<String> = client.smembers("key4").await?;
    assert_eq!(1, members.len());
    assert!(members.contains("c"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sismember() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let result = client.sismember("key", "value1").await?;
    assert!(result);

    let result = client.sismember("key", "value4").await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn smembers() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let members: HashSet<String> = client.smembers("key").await?;
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
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let result = client.smismember("key", ["value1", "value4"]).await?;
    assert_eq!(2, result.len());
    assert!(result[0]);
    assert!(!result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn smove() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client.sadd("key1", ["value1", "value2", "value3"]).await?;
    client.sadd("key2", ["value4", "value5", "value6"]).await?;

    let result = client.smove("key1", "key2", "value3").await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn spop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let result: HashSet<String> = client.spop("key", 2).await?;
    assert_eq!(2, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn srandmember() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let result: HashSet<String> = client.srandmember("key", 2).await?;
    assert_eq!(2, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn srem() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let result = client.srem("key", ["value1", "value2", "value4"]).await?;
    assert_eq!(2, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sscan() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.sadd("key", ["value1", "value2", "value3"]).await?;

    let result: (u64, Vec<String>) = client.sscan("key", 0, SScanOptions::default()).await?;
    assert_eq!(0, result.0);
    assert_eq!(3, result.1.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sunion() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let members: HashSet<String> = client.sunion(["key1", "key2", "key3"]).await?;
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
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).await?;

    client.sadd("key1", ["a", "b", "c", "d"]).await?;
    client.sadd("key2", "c").await?;
    client.sadd("key3", ["a", "c", "e"]).await?;

    let len = client.sunionstore("key4", ["key1", "key2", "key3"]).await?;
    assert_eq!(5, len);

    let members: HashSet<String> = client.smembers("key4").await?;
    assert_eq!(5, members.len());
    assert!(members.contains("a"));
    assert!(members.contains("b"));
    assert!(members.contains("c"));
    assert!(members.contains("d"));
    assert!(members.contains("e"));

    Ok(())
}
