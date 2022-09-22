use crate::{
    tests::get_default_addr, Connection, ConnectionCommandResult, FlushingMode,
    HyperLogLogCommands, Result, ServerCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfadd() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;
    connection.flushdb(FlushingMode::Sync).send().await?;

    let result = connection.pfadd("key", "a").send().await?;
    assert!(result);

    let result = connection
        .pfadd("key", ["b", "c", "d", "e", "f", "g"])
        .send()
        .await?;
    assert!(result);

    let result = connection.pfadd("key", "a").send().await?;
    assert!(!result);

    let result = connection.pfadd("key", ["c", "d", "e"]).send().await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfcount() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;
    connection.flushdb(FlushingMode::Sync).send().await?;

    connection
        .pfadd("key1", ["a", "b", "c", "d", "e", "f", "g"])
        .send()
        .await?;

    let count = connection.pfcount("key1").send().await?;
    assert_eq!(7, count);

    connection
        .pfadd("key2", ["f", "g", "h", "i"])
        .send()
        .await?;

    let count = connection.pfcount(["key1", "key2"]).send().await?;
    assert_eq!(9, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfmerge() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;
    connection.flushdb(FlushingMode::Sync).send().await?;

    connection
        .pfadd("key1", ["foo", "bar", "zap", "a"])
        .send()
        .await?;

    connection
        .pfadd("key2", ["a", "b", "c", "foo"])
        .send()
        .await?;

    connection.pfmerge("out", ["key1", "key2"]).send().await?;

    let count = connection.pfcount("out").send().await?;
    assert_eq!(6, count);

    Ok(())
}
