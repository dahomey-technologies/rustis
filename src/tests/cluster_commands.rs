use crate::{
    Result,
    client::{Client, IntoConfig},
    commands::{
        ClusterCommands, ClusterFailoverOption, ClusterLinkDirection, ClusterLinkInfo,
        ClusterMigrationTarget, ClusterResetType, ClusterShardResult, ClusterSlotStatMetric,
        ClusterSlotStatsFilter, ClusterState, GenericCommands, InternalCommands,
        LegacyClusterShardResult, SortOrder, StringCommands,
    },
    resp::Value,
    tests::{TestClient, get_spare_cluster_node_client, log_try_init, reset_spare_cluster_node},
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

    // `key` hashes to slot 12539. The master owning that slot is the only node
    // that both accepts the write and reports the key, and which node that is
    // depends on the current topology, so it is looked up rather than assumed.
    let slot = client.cluster_keyslot("key").await?;
    assert_eq!(12539, slot);

    let shards: Vec<ClusterShardResult> = client.cluster_shards().await?;
    let shard = shards
        .iter()
        .find(|s| {
            s.slots
                .iter()
                .any(|(start, end)| *start <= slot && slot <= *end)
        })
        .unwrap();
    let master = shard.nodes.iter().find(|n| n.role == "master").unwrap();
    let owner = Client::connect((master.ip.clone(), master.port.unwrap()).into_config()?).await?;
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

// The commands below reconfigure the topology, so none of them can be sent
// against the cluster the other tests share. The wire-form tests here check
// their argument shape against the syntax the server prints under
// `CLUSTER HELP`; the live tests further down send them for real to the spare
// nodes on 7006/7007, which is the only thing that can check the declared
// response type — the wire form never disagrees with a `R` nobody read a reply
// into.

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

// The same twelve commands, sent for real to the spare nodes on 7006/7007.
// What the wire-form tests above cannot check is the declared response type:
// `R` is only ever wrong against a reply, and these commands had never received
// one. Each test resets the node it uses, so none inherits the topology the one
// before it built.

/// Slots are owned after ADDSLOTS and gone after DELSLOTS, which is what makes
/// this an assertion rather than a restatement of the command.
#[tokio::test]
#[serial]
async fn cluster_addslots_delslots() -> Result<()> {
    let client = get_spare_cluster_node_client(1).await?;
    reset_spare_cluster_node(&client).await?;

    client.cluster_addslots([0u16, 1, 2]).await?;
    let info = client.cluster_info().await?;
    assert_eq!(3, info.cluster_slots_assigned);

    client.cluster_delslots([0u16, 1]).await?;
    let info = client.cluster_info().await?;
    assert_eq!(1, info.cluster_slots_assigned);

    reset_spare_cluster_node(&client).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_addslotsrange_delslotsrange() -> Result<()> {
    let client = get_spare_cluster_node_client(1).await?;
    reset_spare_cluster_node(&client).await?;

    client
        .cluster_addslotsrange([(0u16, 100u16), (200, 300)])
        .await?;
    let info = client.cluster_info().await?;
    assert_eq!(202, info.cluster_slots_assigned);

    client.cluster_delslotsrange([(200u16, 300u16)]).await?;
    let info = client.cluster_info().await?;
    assert_eq!(101, info.cluster_slots_assigned);

    reset_spare_cluster_node(&client).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_flushslots() -> Result<()> {
    let client = get_spare_cluster_node_client(1).await?;
    reset_spare_cluster_node(&client).await?;

    client.cluster_addslotsrange([(0u16, 100u16)]).await?;
    assert_eq!(101, client.cluster_info().await?.cluster_slots_assigned);

    client.cluster_flushslots().await?;
    assert_eq!(0, client.cluster_info().await?.cluster_slots_assigned);

    Ok(())
}

#[tokio::test]
#[serial]
async fn cluster_set_config_epoch() -> Result<()> {
    let client = get_spare_cluster_node_client(1).await?;
    // The server only accepts a new epoch on a node whose epoch is still zero,
    // which a hard reset is what produces.
    reset_spare_cluster_node(&client).await?;

    client.cluster_set_config_epoch(12).await?;
    assert_eq!(12, client.cluster_info().await?.cluster_my_epoch);

    reset_spare_cluster_node(&client).await?;

    Ok(())
}

/// A hard reset drops the slots *and* the node id; a soft one keeps the id.
#[tokio::test]
#[serial]
async fn cluster_reset() -> Result<()> {
    let client = get_spare_cluster_node_client(1).await?;
    reset_spare_cluster_node(&client).await?;

    client.cluster_addslotsrange([(0u16, 100u16)]).await?;
    let id_before: String = client.cluster_myid().await?;

    client.cluster_reset(ClusterResetType::Soft).await?;
    assert_eq!(0, client.cluster_info().await?.cluster_slots_assigned);
    let id_after_soft: String = client.cluster_myid().await?;
    assert_eq!(id_before, id_after_soft);

    client.cluster_reset(ClusterResetType::Hard).await?;
    let id_after_hard: String = client.cluster_myid().await?;
    assert_ne!(id_before, id_after_hard);

    Ok(())
}

/// MEET, REPLICATE, FAILOVER and FORGET in one test because each needs the
/// state the one before it leaves: a node cannot replicate a master it has not
/// met, nor fail over a master it does not replicate.
#[tokio::test]
#[serial]
async fn cluster_meet_replicate_failover_forget() -> Result<()> {
    let node1 = get_spare_cluster_node_client(1).await?;
    let node2 = get_spare_cluster_node_client(2).await?;
    reset_spare_cluster_node(&node1).await?;
    reset_spare_cluster_node(&node2).await?;

    // A replica has to be a replica *of* something, and only a node owning every
    // slot makes the cluster reach `ok` — the state FAILOVER requires.
    node1.cluster_addslotsrange([(0u16, 16383u16)]).await?;
    let node1_id: String = node1.cluster_myid().await?;
    let node2_id: String = node2.cluster_myid().await?;

    node2
        .cluster_meet(announced_ip(&node1).await?, 7006, Some(17006))
        .await?;
    wait_for_cluster_ok(&node2).await?;

    node2.cluster_replicate(&node1_id).await?;
    wait_until("node2 becomes a working replica", || async {
        Ok(
            matches!(node2.cluster_info().await?.cluster_state, ClusterState::Ok)
                && node2.cluster_slots::<Value>().await.is_ok(),
        )
    })
    .await?;

    // The promotion is the observable effect: node2 stops being a replica and
    // takes the slots over.
    //
    // TAKEOVER rather than a plain failover: the coordinated form needs the
    // master's agreement over the cluster bus, so how long it takes — and
    // whether it happens at all — depends on gossip timing a test cannot pin
    // down. TAKEOVER promotes unilaterally, which is the same command and the
    // same reply, decided locally.
    node2
        .cluster_failover(Some(ClusterFailoverOption::Takeover))
        .await?;
    wait_until("the failover promotes node2", || async {
        let shards: Vec<ClusterShardResult> = node2.cluster_shards().await?;
        Ok(shards.iter().any(|shard| {
            shard
                .nodes
                .iter()
                .any(|node| node.id == node2_id && node.role == "master")
        }))
    })
    .await?;

    node2.cluster_forget(&node1_id).await?;
    wait_until("node2 forgets node1", || async {
        let nodes: String = node2.cluster_nodes().await?;
        Ok(!nodes.contains(&node1_id))
    })
    .await?;

    reset_spare_cluster_node(&node1).await?;
    reset_spare_cluster_node(&node2).await?;

    Ok(())
}

/// `CLUSTER MIGRATION IMPORT` answers the id of the migration task it started,
/// not an acknowledgement — and that id is what `CLUSTER MIGRATION CANCEL`
/// takes, so a caller reading the reply as `()` cannot cancel what it began.
#[tokio::test]
#[serial]
async fn cluster_migration_import() -> Result<()> {
    let node1 = get_spare_cluster_node_client(1).await?;
    let node2 = get_spare_cluster_node_client(2).await?;
    reset_spare_cluster_node(&node1).await?;
    reset_spare_cluster_node(&node2).await?;

    node1.cluster_addslotsrange([(0u16, 16383u16)]).await?;
    node2
        .cluster_meet(announced_ip(&node1).await?, 7006, Some(17006))
        .await?;
    wait_for_cluster_ok(&node2).await?;

    let task_id: String = node2.cluster_migration_import([(0u16, 100u16)]).await?;
    assert_eq!(40, task_id.len(), "expected a task id, got {task_id:?}");

    wait_until("the imported slots land on node2", || async {
        Ok(node2.cluster_info().await?.cluster_slots_assigned == 16384)
    })
    .await?;

    reset_spare_cluster_node(&node1).await?;
    reset_spare_cluster_node(&node2).await?;

    Ok(())
}

/// The cluster bus converges on its own schedule, so every assertion about the
/// state it produces has to be given time rather than a single shot.
async fn wait_until<F, Fut>(label: &str, mut condition: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    for _ in 0..150 {
        if condition().await? {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    panic!("{label}: condition still false after 30s");
}

/// The address a node announces on the cluster bus, which is the only one MEET
/// accepts: the host the test connects through may be a name, and the server
/// answers `Invalid node address specified` to anything it cannot parse as an
/// IP. The node has to own a slot for `CLUSTER SHARDS` to report it.
async fn announced_ip(client: &Client) -> Result<String> {
    let shards: Vec<ClusterShardResult> = client.cluster_shards().await?;
    let node = shards
        .first()
        .and_then(|shard| shard.nodes.first())
        .expect("the node owns no slot, so it announces no address");

    Ok(node.ip.clone())
}

async fn wait_for_cluster_ok(client: &Client) -> Result<()> {
    wait_until("the cluster reaches state ok", || async {
        Ok(matches!(
            client.cluster_info().await?.cluster_state,
            ClusterState::Ok
        ))
    })
    .await
}
