use crate::{
    tests::get_default_addr, ConnectionMultiplexer, DatabaseCommandResult, FlushingMode,
    HyperLogLogCommands, Result, ServerCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfadd() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();
    database.flushdb(FlushingMode::Sync).send().await?;

    let result = database.pfadd("key", "a").send().await?;
    assert!(result);

    let result = database
        .pfadd("key", ["b", "c", "d", "e", "f", "g"])
        .send()
        .await?;
    assert!(result);

    let result = database.pfadd("key", "a").send().await?;
    assert!(!result);

    let result = database.pfadd("key", ["c", "d", "e"]).send().await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfcount() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();
    database.flushdb(FlushingMode::Sync).send().await?;

    database
        .pfadd("key1", ["a", "b", "c", "d", "e", "f", "g"])
        .send()
        .await?;

    let count = database.pfcount("key1").send().await?;
    assert_eq!(7, count);

    database.pfadd("key2", ["f", "g", "h", "i"]).send().await?;

    let count = database.pfcount(["key1", "key2"]).send().await?;
    assert_eq!(9, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pfmerge() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();
    database.flushdb(FlushingMode::Sync).send().await?;

    database
        .pfadd("key1", ["foo", "bar", "zap", "a"])
        .send()
        .await?;

    database
        .pfadd("key2", ["a", "b", "c", "foo"])
        .send()
        .await?;

    database.pfmerge("out", ["key1", "key2"]).send().await?;

    let count = database.pfcount("out").send().await?;
    assert_eq!(6, count);

    Ok(())
}
