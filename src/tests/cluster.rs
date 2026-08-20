use crate::{
    commands::{
        ClusterCommands,
        ClusterSetSlotSubCommand::{self},
        LegacyClusterNodeResult, LegacyClusterShardResult,
    },
    network::convert_from_legacy_shard_description,
    tests::TestClient,
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
