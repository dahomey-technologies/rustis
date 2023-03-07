use crate::{
    client::Client,
    commands::{ClusterCommands, ClusterShardResult},
    tests::log_try_init,
    Result,
};
use log::debug;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn asking() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    client.asking().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_shards() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let shards: Vec<ClusterShardResult> = client.cluster_shards().await?;
    debug!("shards: {shards:?}");
    assert_eq!(3, shards.len());

    Ok(())
}
