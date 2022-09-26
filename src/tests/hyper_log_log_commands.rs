use crate::{
    tests::get_test_client, ClientCommandResult, FlushingMode, HyperLogLogCommands, Result,
    ServerCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfadd() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    let result = client.pfadd("key", "a").send().await?;
    assert!(result);

    let result = client
        .pfadd("key", ["b", "c", "d", "e", "f", "g"])
        .send()
        .await?;
    assert!(result);

    let result = client.pfadd("key", "a").send().await?;
    assert!(!result);

    let result = client.pfadd("key", ["c", "d", "e"]).send().await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfcount() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    client
        .pfadd("key1", ["a", "b", "c", "d", "e", "f", "g"])
        .send()
        .await?;

    let count = client.pfcount("key1").send().await?;
    assert_eq!(7, count);

    client.pfadd("key2", ["f", "g", "h", "i"]).send().await?;

    let count = client.pfcount(["key1", "key2"]).send().await?;
    assert_eq!(9, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfmerge() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    client
        .pfadd("key1", ["foo", "bar", "zap", "a"])
        .send()
        .await?;

    client.pfadd("key2", ["a", "b", "c", "foo"]).send().await?;

    client.pfmerge("out", ["key1", "key2"]).send().await?;

    let count = client.pfcount("out").send().await?;
    assert_eq!(6, count);

    Ok(())
}
