use crate::{
    Result,
    client::{Client, IntoConfig},
    commands::{
        ClusterCommands, ClusterFailoverOption, ClusterLinkDirection, ClusterLinkInfo,
        ClusterMigrationTarget, ClusterResetType, ClusterShardResult, ClusterSlotStatMetric,
        ClusterSlotStatsFilter, ClusterState, GenericCommands, LegacyClusterShardResult, SortOrder,
        StringCommands,
    },
    resp::Value,
    tests::{TestClient, log_try_init},
};
use serial_test::serial;
use tracing::debug;

#[tokio::test]
#[serial]
async fn asking() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    client.asking().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_shards() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let shards: Vec<ClusterShardResult> = client.cluster_shards().await?;
    debug!("shards: {shards:?}");
    assert_eq!(3, shards.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_slots() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let shards: Vec<LegacyClusterShardResult> = client.cluster_slots().await?;
    debug!("shards: {shards:?}");
    assert_eq!(3, shards.len());

    Ok(())
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
#[serial]
async fn cluster_myid_and_nodes() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    let my_id: String = client.cluster_myid().await?;
    assert_eq!(40, my_id.len());
    assert!(my_id.chars().all(|c| c.is_ascii_hexdigit()));

    // The shard id groups a master with its replicas, so it differs from the
    // node id while sharing its shape.
    let my_shard_id: String = client.cluster_myshardid().await?;
    assert_eq!(40, my_shard_id.len());
    assert!(my_shard_id.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(my_id, my_shard_id);

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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
#[serial]
async fn cluster_saveconfig() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    client.cluster_saveconfig().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_bumpepoch() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    // The outcome depends on where this node's epoch sits among its peers, and
    // both outcomes carry the epoch it ends up with — which a cluster that has
    // ever elected anything has advanced past zero.
    let result = client.cluster_bumpepoch().await?;
    debug!("bumpepoch: {result:?}");
    assert!(result.epoch() > 0);

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_migration_cancel() -> Result<()> {
    log_try_init();
    let client = Client::connect("127.0.0.1:7000").await?;

    // Nothing is migrating in a healthy test cluster, so there is nothing to
    // cancel; what the call proves is that the command is accepted as written.
    let cancelled = client
        .cluster_migration_cancel(ClusterMigrationTarget::All)
        .await?;
    assert_eq!(0, cancelled);

    let cancelled = client
        .cluster_migration_cancel(ClusterMigrationTarget::Id("no-such-task"))
        .await?;
    assert_eq!(0, cancelled);

    Ok(())
}

// The commands below reconfigure the topology, and the test cluster is shared
// by every other cluster test, so none of them can be sent. What is still
// checkable is the wire form: each expectation below is the syntax the server
// prints for itself under `CLUSTER HELP`.

#[test]
fn cluster_addslots_command() {
    let cmd = TestClient.cluster_addslots([0u16, 1, 2]).command;
    assert_eq!("CLUSTER ADDSLOTS 0 1 2", cmd.to_string());
}

#[test]
fn cluster_addslotsrange_command() {
    let cmd = TestClient
        .cluster_addslotsrange([(0u16, 100u16), (200, 300)])
        .command;
    assert_eq!("CLUSTER ADDSLOTSRANGE 0 100 200 300", cmd.to_string());
}

#[test]
fn cluster_delslots_command() {
    let cmd = TestClient.cluster_delslots([0u16, 1, 2]).command;
    assert_eq!("CLUSTER DELSLOTS 0 1 2", cmd.to_string());
}

#[test]
fn cluster_delslotsrange_command() {
    let cmd = TestClient
        .cluster_delslotsrange([(0u16, 100u16), (200, 300)])
        .command;
    assert_eq!("CLUSTER DELSLOTSRANGE 0 100 200 300", cmd.to_string());
}

/// The option is what to watch: omitted, it must leave no trailing token.
#[test]
fn cluster_failover_command() {
    let cmd = TestClient.cluster_failover(None).command;
    assert_eq!("CLUSTER FAILOVER", cmd.to_string());

    let cmd = TestClient
        .cluster_failover(Some(ClusterFailoverOption::Force))
        .command;
    assert_eq!("CLUSTER FAILOVER FORCE", cmd.to_string());

    let cmd = TestClient
        .cluster_failover(Some(ClusterFailoverOption::Takeover))
        .command;
    assert_eq!("CLUSTER FAILOVER TAKEOVER", cmd.to_string());
}

#[test]
fn cluster_flushslots_command() {
    let cmd = TestClient.cluster_flushslots().command;
    assert_eq!("CLUSTER FLUSHSLOTS", cmd.to_string());
}

#[test]
fn cluster_forget_command() {
    let cmd = TestClient
        .cluster_forget("37618c7eec0dd58e946e1ef0df02d8c5a9a14235")
        .command;
    assert_eq!(
        "CLUSTER FORGET 37618c7eec0dd58e946e1ef0df02d8c5a9a14235",
        cmd.to_string()
    );
}

/// The cluster bus port is optional and comes last.
#[test]
fn cluster_meet_command() {
    let cmd = TestClient.cluster_meet("127.0.0.1", 7000, None).command;
    assert_eq!("CLUSTER MEET 127.0.0.1 7000", cmd.to_string());

    let cmd = TestClient
        .cluster_meet("127.0.0.1", 7000, Some(17000))
        .command;
    assert_eq!("CLUSTER MEET 127.0.0.1 7000 17000", cmd.to_string());
}

#[test]
fn cluster_replicate_command() {
    let cmd = TestClient
        .cluster_replicate("37618c7eec0dd58e946e1ef0df02d8c5a9a14235")
        .command;
    assert_eq!(
        "CLUSTER REPLICATE 37618c7eec0dd58e946e1ef0df02d8c5a9a14235",
        cmd.to_string()
    );
}

#[test]
fn cluster_reset_command() {
    let cmd = TestClient.cluster_reset(ClusterResetType::Hard).command;
    assert_eq!("CLUSTER RESET HARD", cmd.to_string());

    let cmd = TestClient.cluster_reset(ClusterResetType::Soft).command;
    assert_eq!("CLUSTER RESET SOFT", cmd.to_string());
}

#[test]
fn cluster_set_config_epoch_command() {
    let cmd = TestClient.cluster_set_config_epoch(12).command;
    assert_eq!("CLUSTER SET-CONFIG-EPOCH 12", cmd.to_string());
}

/// `CLUSTER MIGRATION IMPORT` takes slot ranges, two tokens per range, like
/// `ADDSLOTSRANGE` and unlike the single slots of `ADDSLOTS`.
#[test]
fn cluster_migration_import_command() {
    let cmd = TestClient
        .cluster_migration_import::<()>([(0u16, 100u16), (200, 300)])
        .command;
    assert_eq!("CLUSTER MIGRATION IMPORT 0 100 200 300", cmd.to_string());
}
