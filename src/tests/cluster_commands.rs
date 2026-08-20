use crate::{
    commands::{ClusterCommands, ClusterFailoverOption, ClusterResetType},
    tests::TestClient,
};

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
