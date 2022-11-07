use crate::{tests::log_try_init, Client, ClusterCommands, ClusterShardResult, Result};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn asking() -> Result<()> {
    log_try_init();
    let mut client = Client::connect("127.0.0.1:7000").await?;

    client.asking().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_shards() -> Result<()> {
    log_try_init();
    let mut client = Client::connect("127.0.0.1:7000").await?;

    let shards: Vec<ClusterShardResult> = client.cluster_shards().await?;
    assert_eq!(3, shards.len());

    Ok(())
}
