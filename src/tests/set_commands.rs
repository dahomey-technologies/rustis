use std::collections::HashSet;

use crate::{
    tests::get_test_client, ClientCommandResult, GenericCommands, Result, SScanOptions,
    SetCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sadd() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    let len = client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn scard() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;
    let len = client.scard("key").send().await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sdiff() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let members: HashSet<String> = client.sdiff(["key1", "key2", "key3"]).send().await?;
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
    client.del(["key1", "key2", "key3", "key4"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = client
        .sdiffstore("key4", ["key1", "key2", "key3"])
        .send()
        .await?;
    assert_eq!(2, len);

    let members: HashSet<String> = client.smembers("key4").send().await?;
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
    client.del(["key1", "key2", "key3"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let members: HashSet<String> = client.sinter(["key1", "key2", "key3"]).send().await?;
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
    client.del(["key1", "key2", "key3"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = client
        .sintercard(["key1", "key2", "key3"], 0)
        .send()
        .await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sinterstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = client
        .sinterstore("key4", ["key1", "key2", "key3"])
        .send()
        .await?;
    assert_eq!(1, len);

    let members: HashSet<String> = client.smembers("key4").send().await?;
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
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let result = client.sismember("key", "value1").send().await?;
    assert!(result);

    let result = client.sismember("key", "value4").send().await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn smembers() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let members: HashSet<String> = client.smembers("key").send().await?;
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
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let result = client
        .smismember("key", ["value1", "value4"])
        .send()
        .await?;
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
    client.del(["key1", "key2"]).send().await?;

    client
        .sadd("key1", ["value1", "value2", "value3"])
        .send()
        .await?;
    client
        .sadd("key2", ["value4", "value5", "value6"])
        .send()
        .await?;

    let result = client.smove("key1", "key2", "value3").send().await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn spop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let result: HashSet<String> = client.spop("key", 2).send().await?;
    assert_eq!(2, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn srandmember() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let result: HashSet<String> = client.srandmember("key", 2).send().await?;
    assert_eq!(2, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn srem() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let result = client
        .srem("key", ["value1", "value2", "value4"])
        .send()
        .await?;
    assert_eq!(2, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sscan() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    client
        .sadd("key", ["value1", "value2", "value3"])
        .send()
        .await?;

    let result: (u64, Vec<String>) = client
        .sscan("key", 0, SScanOptions::default())
        .send()
        .await?;
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
    client.del(["key1", "key2", "key3"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let members: HashSet<String> = client.sunion(["key1", "key2", "key3"]).send().await?;
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
    client.del(["key1", "key2", "key3", "key4"]).send().await?;

    client.sadd("key1", ["a", "b", "c", "d"]).send().await?;
    client.sadd("key2", "c").send().await?;
    client.sadd("key3", ["a", "c", "e"]).send().await?;

    let len = client
        .sunionstore("key4", ["key1", "key2", "key3"])
        .send()
        .await?;
    assert_eq!(5, len);

    let members: HashSet<String> = client.smembers("key4").send().await?;
    assert_eq!(5, members.len());
    assert!(members.contains("a"));
    assert!(members.contains("b"));
    assert!(members.contains("c"));
    assert!(members.contains("d"));
    assert!(members.contains("e"));

    Ok(())
}
