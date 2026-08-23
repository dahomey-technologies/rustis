use crate::{
    Result,
    client::{ClusterConfig, Config},
    commands::{
        ClusterCommands,
        ClusterSetSlotSubCommand::{self},
        LegacyClusterNodeResult, LegacyClusterShardResult,
    },
    network::{ClusterConnection, ConnectionState, convert_from_legacy_shard_description},
    tests::{
        TestClient,
        fake_server::{FakeServer, TcpNode},
    },
};

/// Builds a `CLUSTER SLOTS` node entry; only the id matters to the conversion.
fn legacy_node(id: &str, port: u16) -> LegacyClusterNodeResult {
    LegacyClusterNodeResult {
        id: id.to_owned(),
        preferred_endpoint: "127.0.0.1".to_owned(),
        ip: "127.0.0.1".to_owned(),
        hostname: None,
        port,
    }
}

#[test]
fn cluster_selslot_command() {
    let cmd = TestClient
        .cluster_setslot(
            12539,
            ClusterSetSlotSubCommand::Migrating("37618c7eec0dd58e946e1ef0df02d8c5a9a14235"),
        )
        .command;
    assert_eq!(
        "CLUSTER SETSLOT 12539 MIGRATING 37618c7eec0dd58e946e1ef0df02d8c5a9a14235",
        cmd.to_string()
    );
}

#[test]
fn a_legacy_shard_without_any_node_is_skipped_rather_than_indexed() {
    // A `CLUSTER SLOTS` entry that lists no node describes nothing routable. The
    // conversion reads each entry's first node to group slots by master, both
    // while sorting and while grouping — on the network task, where a panic
    // would take the whole client down with it.
    let converted = convert_from_legacy_shard_description(vec![
        LegacyClusterShardResult {
            slot: (0, 100),
            nodes: vec![],
        },
        LegacyClusterShardResult {
            slot: (101, 200),
            nodes: vec![legacy_node("node-a", 7000)],
        },
    ]);

    assert_eq!(1, converted.len());
    let shard = &converted[0];
    assert_eq!(vec![(101, 200)], shard.slots);
    assert_eq!("node-a", shard.nodes[0].id);
    assert_eq!("master", shard.nodes[0].role);
}

#[test]
fn legacy_shards_sharing_a_master_are_merged_into_one_shard() {
    // The grouping the skip above must not disturb: consecutive entries with the
    // same master accumulate their slot ranges, and the first node of each entry
    // is the master while the rest are replicas.
    let converted = convert_from_legacy_shard_description(vec![
        LegacyClusterShardResult {
            slot: (0, 100),
            nodes: vec![legacy_node("node-a", 7000), legacy_node("node-b", 7001)],
        },
        LegacyClusterShardResult {
            slot: (101, 200),
            nodes: vec![legacy_node("node-a", 7000)],
        },
        LegacyClusterShardResult {
            slot: (201, 300),
            nodes: vec![legacy_node("node-c", 7002)],
        },
    ]);

    assert_eq!(2, converted.len());
    assert_eq!(vec![(0, 100), (101, 200)], converted[0].slots);
    assert_eq!("node-a", converted[0].nodes[0].id);
    assert_eq!("master", converted[0].nodes[0].role);
    assert_eq!("replica", converted[0].nodes[1].role);
    assert_eq!(vec![(201, 300)], converted[1].slots);
    assert_eq!("node-c", converted[1].nodes[0].id);
}

/// A RESP3 bulk string.
fn bulk(value: &str) -> String {
    format!("${}\r\n{value}\r\n", value.len())
}

/// A RESP3 map, whose values are already encoded.
fn resp_map(entries: &[(&str, String)]) -> String {
    let mut map = format!("%{}\r\n", entries.len());
    for (key, value) in entries {
        map.push_str(&bulk(key));
        map.push_str(value);
    }
    map
}

/// A `CLUSTER SHARDS` reply describing one shard that owns every slot, served by
/// the master listening on `port`.
fn one_shard_owning_every_slot(node_id: &str, port: u16) -> Vec<u8> {
    let master = resp_map(&[
        ("id", bulk(node_id)),
        ("endpoint", bulk("127.0.0.1")),
        ("ip", bulk("127.0.0.1")),
        ("port", format!(":{port}\r\n")),
        ("hostname", bulk("")),
        ("role", bulk("master")),
        ("replication-offset", ":0\r\n".to_owned()),
        ("health", bulk("online")),
    ]);
    let shard = resp_map(&[
        ("slots", "*2\r\n:0\r\n:16383\r\n".to_owned()),
        ("nodes", format!("*1\r\n{master}")),
    ]);

    format!("*1\r\n{shard}").into_bytes()
}

/// A reconnection rediscovers the topology from the nodes the client holds,
/// falling back to the configured seeds rather than starting from them.
///
/// The held nodes answered a moment earlier, which is more than is known of any
/// seed; and the seeds are typically one control-plane endpoint, so they are what
/// a partial outage takes away. Dialling them alone fails the whole attempt with
/// `ClientError::ClusterConfig` while a working node sits untried in the
/// topology, and every attempt of the handler's budget repeats the same too-small
/// dial — so the client stays down for as long as the seed does, whatever
/// `max_attempts` says.
#[tokio::test]
async fn a_reconnection_rediscovers_from_the_nodes_it_holds() -> Result<()> {
    const MASTER_ID: &str = "0000000000000000000000000000000000000001";

    // Both nodes answer discovery, so which one the reconnection reaches is the
    // only thing the outcome can depend on.
    let mut master = TcpNode::bind().await?;
    let mut seed = TcpNode::bind().await?;
    let shards = one_shard_owning_every_slot(MASTER_ID, master.addr.port());
    master.serve(FakeServer::new().reply("CLUSTER", &shards));
    seed.serve(FakeServer::new().reply("CLUSTER", &shards));

    let cluster_config = ClusterConfig {
        nodes: vec![seed.address()],
        ..Default::default()
    };
    let config = Config::default();
    let mut connection_state = ConnectionState::default();

    let mut cluster =
        ClusterConnection::connect(&cluster_config, &config, &mut connection_state).await?;
    assert_eq!(
        1,
        seed.accepted(),
        "the seed serves the initial discovery, nothing being held yet"
    );

    // Both nodes up: discovery stops at the first address that answers, so the
    // seed being left alone is what says the held node was tried before it.
    let master_dials = master.accepted();
    cluster.reconnect(&mut connection_state).await?;
    assert!(
        master.accepted() > master_dials,
        "a rediscovery dials the node it holds"
    );
    assert_eq!(
        1,
        seed.accepted(),
        "a held node answered, so no seed is needed"
    );

    // The seed goes, the master stays: rediscovering from the seeds alone now has
    // nowhere to go, while the node held is still answering.
    drop(seed);

    let master_dials = master.accepted();
    cluster
        .reconnect(&mut connection_state)
        .await
        .expect("a held node answers, so the reconnection has somewhere to go");
    assert!(
        master.accepted() > master_dials,
        "the reconnection must fall back on the node it holds"
    );

    Ok(())
}
