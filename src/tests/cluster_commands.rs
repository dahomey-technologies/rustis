use crate::{
    Result,
    client::Client,
    commands::{
        ClusterCommands, ClusterMigrationTarget, ClusterShardResult, ClusterSlotStatMetric,
        ClusterSlotStatsFilter, LegacyClusterShardResult, SortOrder,
    },
    resp::Value,
    tests::log_try_init,
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_slots() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let shards: Vec<LegacyClusterShardResult> = client.cluster_slots().await?;
    debug!("shards: {shards:?}");
    assert_eq!(3, shards.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_slot_stats() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    // The node at 7000 owns the first slot range, so an inclusive 0..=3 range
    // reports four per-slot entries.
    let stats: Vec<Value> = client
        .cluster_slot_stats(ClusterSlotStatsFilter::SlotsRange { start: 0, end: 3 })
        .await?;
    debug!("slot stats: {stats:?}");
    assert_eq!(4, stats.len());

    // ORDERBY with a LIMIT caps the number of entries returned.
    let stats: Vec<Value> = client
        .cluster_slot_stats(ClusterSlotStatsFilter::OrderBy {
            metric: ClusterSlotStatMetric::KeyCount,
            limit: Some(2),
            order: Some(SortOrder::Desc),
        })
        .await?;
    assert!(stats.len() <= 2);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_migration_status() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    // A healthy test cluster has no migration in progress, so STATUS ALL returns
    // an empty task list.
    let tasks: Vec<Value> = client
        .cluster_migration_status(ClusterMigrationTarget::All)
        .await?;
    assert!(tasks.is_empty());

    Ok(())
}
