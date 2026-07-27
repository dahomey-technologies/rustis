use crate::{
    Result,
    client::{Client, IntoConfig},
    commands::{
        ClusterCommands, ClusterLinkDirection, ClusterLinkInfo, ClusterMigrationTarget,
        ClusterShardResult, ClusterSlotStatMetric, ClusterSlotStatsFilter, ClusterState,
        GenericCommands, LegacyClusterShardResult, SortOrder, StringCommands,
    },
    resp::Value,
    tests::log_try_init,
};
use serial_test::serial;
use tracing::debug;

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
async fn cluster_info() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let info = client.cluster_info().await?;
    debug!("info: {info:?}");

    assert_eq!(ClusterState::Ok, info.cluster_state);
    assert_eq!(16384, info.cluster_slots_assigned);
    assert_eq!(3, info.cluster_size);
    assert_eq!(6, info.cluster_known_nodes);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_getkeysinslot() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    // `key` hashes to slot 12539, which belongs to the third master's range
    // (10923-16383), so that node both accepts the write and reports the key.
    let slot = client.cluster_keyslot("key").await?;
    assert_eq!(12539, slot);

    let owner = Client::connect("127.0.0.1:7002").await?;
    owner.del("key").await?;
    assert_eq!(0, owner.cluster_countkeysinslot(slot as usize).await?);

    owner.set("key", "value").await?;
    assert_eq!(1, owner.cluster_countkeysinslot(slot as usize).await?);

    let keys: Vec<String> = owner.cluster_getkeysinslot(slot, 10).await?;
    assert_eq!(vec!["key".to_owned()], keys);

    owner.del("key").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_myid_and_nodes() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let my_id: String = client.cluster_myid().await?;
    assert_eq!(40, my_id.len());
    assert!(my_id.chars().all(|c| c.is_ascii_hexdigit()));

    // CLUSTER NODES is the same text format as the nodes.conf file: one line per
    // node, the contacted one flagged `myself`.
    let nodes: String = client.cluster_nodes().await?;
    let lines: Vec<&str> = nodes.lines().collect();
    assert_eq!(6, lines.len());

    let myself = lines
        .iter()
        .find(|l| l.split(' ').nth(2).is_some_and(|f| f.contains("myself")))
        .unwrap();
    assert!(myself.starts_with(&my_id));
    assert_eq!(3, lines.iter().filter(|l| l.contains("master")).count());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_links() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    // Each peer is reached by one link the node opened and one it accepted.
    let links: Vec<ClusterLinkInfo> = client.cluster_links().await?;
    debug!("links: {links:?}");
    assert_eq!(10, links.len());

    let my_id: String = client.cluster_myid().await?;
    assert!(links.iter().all(|l| l.node != my_id));
    assert_eq!(
        5,
        links
            .iter()
            .filter(|l| l.direction == ClusterLinkDirection::To)
            .count()
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_replicas_and_failure_reports() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let my_id: String = client.cluster_myid().await?;

    // Every master of the test cluster is given exactly one replica.
    let replicas: Vec<String> = client.cluster_replicas(my_id.clone()).await?;
    assert_eq!(1, replicas.len());
    assert!(replicas[0].contains("slave"));

    // A healthy node is reported by nobody.
    let reports = client.cluster_count_failure_reports(my_id).await?;
    assert_eq!(0, reports);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn readonly_and_readwrite() -> Result<()> {
    log_try_init();
    let seed = Client::connect("127.0.0.1:7000").await?;

    // Find the replica of the shard that serves `key`, since a replica only
    // answers for the slots its own master owns.
    let slot = seed.cluster_keyslot("key").await?;
    let shards: Vec<ClusterShardResult> = seed.cluster_shards().await?;
    let shard = shards
        .iter()
        .find(|s| {
            s.slots
                .iter()
                .any(|(start, end)| *start <= slot && slot <= *end)
        })
        .unwrap();
    let replica = shard.nodes.iter().find(|n| n.role == "replica").unwrap();

    // Both commands act on the connection they are sent on, so they are tested
    // against that single node rather than through a routed cluster client.
    let client =
        Client::connect((replica.ip.clone(), replica.port.unwrap()).into_config()?).await?;

    // A replica redirects reads until the connection opts into stale reads.
    let result: Result<String> = client.get("key").await;
    assert!(result.is_err());

    client.readonly().await?;
    let _: Value = client.get("key").await?;

    client.readwrite().await?;
    let result: Result<String> = client.get("key").await;
    assert!(result.is_err());

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
