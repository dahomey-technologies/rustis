use crate::{
    commands::{FlushingMode, HyperLogLogCommands, ServerCommands},
    tests::get_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfadd() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client.pfadd("key", "a").await?;
    assert!(result);

    let result = client.pfadd("key", ["b", "c", "d", "e", "f", "g"]).await?;
    assert!(result);

    let result = client.pfadd("key", "a").await?;
    assert!(!result);

    let result = client.pfadd("key", ["c", "d", "e"]).await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfcount() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .pfadd("key1", ["a", "b", "c", "d", "e", "f", "g"])
        .await?;

    let count = client.pfcount("key1").await?;
    assert_eq!(7, count);

    client.pfadd("key2", ["f", "g", "h", "i"]).await?;

    let count = client.pfcount(["key1", "key2"]).await?;
    assert_eq!(9, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfmerge() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.pfadd("key1", ["foo", "bar", "zap", "a"]).await?;

    client.pfadd("key2", ["a", "b", "c", "foo"]).await?;

    client.pfmerge("out", ["key1", "key2"]).await?;

    let count = client.pfcount("out").await?;
    assert_eq!(6, count);

    Ok(())
}
