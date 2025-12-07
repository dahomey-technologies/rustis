use std::{collections::HashMap, fmt};

use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{CommandArgs, Response, Args, cmd, deserialize_vec_of_pairs},
};
use serde::{
    Deserialize,
    de::{self},
};

/// A group of Redis commands related to [`Cluster Management`](https://redis.io/docs/management/scaling/)
/// # See Also
/// [Redis Cluster Management commands](https://redis.io/commands/?group=cluster)
/// [Redis cluster specification](https://redis.io/docs/reference/cluster-spec/)
pub trait ClusterCommands<'a>: Sized {
    /// When a cluster client receives an -ASK redirect,
    /// the ASKING command is sent to the target node followed by the command which was redirected.
    /// This is normally done automatically by cluster clients.
    ///
    /// # See Also
    /// [<https://redis.io/commands/asking/>](https://redis.io/commands/asking/)
    #[must_use]
    fn asking(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("ASKING"))
    }

    /// This command is useful in order to modify a node's view of the cluster configuration.
    ///
    /// Specifically it assigns a set of hash slots to the node receiving the command.
    /// If the command is successful, the node will map the specified hash slots to itself,
    /// and will start broadcasting the new configuration.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-addslots/>](https://redis.io/commands/cluster-addslots/)
    #[must_use]
    fn cluster_addslots<S>(self, slots: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("ADDSLOTS").arg(slots))
    }

    /// This command is similar to the [`cluster_addslots`](ClusterCommands::cluster_addslots)
    /// command in that they both assign hash slots to nodes.
    ///
    /// The difference between the two commands is that [`cluster_addslots`](ClusterCommands::cluster_addslots)
    /// takes a list of slots to assign to the node, while this command takes a list of slot ranges
    /// (specified by a tuple containing start and end slots) to assign to the node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-addslotsrange/>](https://redis.io/commands/cluster-addslotsrange/)
    #[must_use]
    fn cluster_addslotsrange<S>(self, slots: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("ADDSLOTSRANGE").arg(slots))
    }

    /// Advances the cluster config epoch.
    ///
    /// # Return
    /// * `Bumped` if the epoch was incremented, or
    /// * `Still` if the node already has the greatest config epoch in the cluster.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-bumpepoch/>](https://redis.io/commands/cluster-bumpepoch/)
    #[must_use]
    fn cluster_bumpepoch(self) -> PreparedCommand<'a, Self, ClusterBumpEpochResult> {
        prepare_command(self, cmd("CLUSTER").arg("BUMPEPOCH"))
    }

    /// The command returns the number of failure reports for the specified node.
    ///
    /// # Return
    /// The number of active failure reports for the node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-count-failure-reports/>](https://redis.io/commands/cluster-count-failure-reports/)
    #[must_use]
    fn cluster_count_failure_reports<I>(
        self,
        node_id: impl Args,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("CLUSTER").arg("COUNT-FAILURE-REPORTS").arg(node_id),
        )
    }

    /// Returns the number of keys in the specified Redis Cluster hash slot.
    ///
    /// # Return
    /// The number of keys in the specified hash slot, or an error if the hash slot is invalid.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-countkeysinslot/>](https://redis.io/commands/cluster-countkeysinslot/)
    #[must_use]
    fn cluster_countkeysinslot(self, slot: usize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("CLUSTER").arg("COUNTKEYSINSLOT").arg(slot))
    }

    /// In Redis Cluster, each node keeps track of which master is serving a particular hash slot.
    /// This command asks a particular Redis Cluster node to forget which master
    ///  is serving the hash slots specified as arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-delslots/>](https://redis.io/commands/cluster-delslots/)
    #[must_use]
    fn cluster_delslots<S>(self, slots: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("DELSLOTS").arg(slots))
    }

    /// This command is similar to the [`cluster_delslotsrange`](ClusterCommands::cluster_delslotsrange)
    ///  command in that they both remove hash slots from the node.
    ///
    /// The difference is that [`cluster_delslotsrange`](ClusterCommands::cluster_delslotsrange)
    ///  takes a list of hash slots to remove from the node,
    /// while this command takes a list of slot ranges (specified by a tuple containing start and end slots) to remove from the node.
    /// # See Also
    /// [<https://redis.io/commands/cluster-delslotsrange/>](https://redis.io/commands/cluster-delslotsrange/)
    #[must_use]
    fn cluster_delslotsrange<S>(self, slots: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("DELSLOTSRANGE").arg(slots))
    }

    /// This command, that can only be sent to a Redis Cluster replica node,
    /// forces the replica to start a manual failover of its master instance.
    ///
    /// # Errors
    /// An error cann occured if the operation cannot be executed,
    /// for example if we are talking with a node which is already a master.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-failover/>](https://redis.io/commands/cluster-failover/)
    #[must_use]
    fn cluster_failover(self, option: ClusterFailoverOption) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("FAILOVER").arg(option))
    }

    /// Deletes all slots from a node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-flushslots/>](https://redis.io/commands/cluster-flushslots/)
    #[must_use]
    fn cluster_flushslots(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("FLUSHSLOTS"))
    }

    /// The command is used in order to remove a node, specified via its node ID,
    /// from the set of known nodes of the Redis Cluster node receiving the command.
    /// In other words the specified node is removed from the nodes table of the node receiving the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-forget/>](https://redis.io/commands/cluster-forget/)
    #[must_use]
    fn cluster_forget<I>(self, node_id: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("FORGET").arg(node_id))
    }

    /// The command returns an array of keys names stored in
    /// the contacted node and hashing to the specified hash slot.
    ///
    /// The maximum number of keys to return is specified via the count argument,
    /// so that it is possible for the user of this API to batch-processing keys.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-getkeysinslot/>](https://redis.io/commands/cluster-getkeysinslot/)
    #[must_use]
    fn cluster_getkeysinslot(self, slot: u16, count: usize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLUSTER").arg("GETKEYSINSLOT").arg(slot).arg(count),
        )
    }

    /// This command provides [`info`](crate::commands::ServerCommands::info) style information about Redis Cluster vital parameters.
    ///
    /// # Return
    /// The Cluster information
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-info/>](https://redis.io/commands/cluster-info/)
    #[must_use]
    fn cluster_info(self, slot: u16, count: usize) -> PreparedCommand<'a, Self, ClusterInfo> {
        prepare_command(self, cmd("CLUSTER").arg("INFO").arg(slot).arg(count))
    }

    /// Returns an integer identifying the hash slot the specified key hashes to.
    ///
    /// # Return
    /// The hash slot number.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-keyslot/>](https://redis.io/commands/cluster-keyslot/)
    #[must_use]
    fn cluster_keyslot(self, key: impl Args) -> PreparedCommand<'a, Self, u16> {
        prepare_command(self, cmd("CLUSTER").arg("KEYSLOT").arg(key))
    }

    /// Each node in a Redis Cluster maintains a pair of long-lived TCP link with each peer in the cluster:
    /// - One for sending outbound messages towards the peer
    /// - and one for receiving inbound messages from the peer.
    ///
    /// This command outputs information of all such peer links as an array,
    /// where each array element is a struct that contains attributes and their values for an individual link.
    ///
    /// # Return
    /// An array of structs where each struct contains various attributes and their values of a cluster link.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-links/>](https://redis.io/commands/cluster-links/)
    #[must_use]
    fn cluster_links<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("LINKS"))
    }

    /// This command is used in order to connect different Redis nodes with cluster support enabled, into a working cluster.
    ///
    /// # Return
    /// An array of structs where each struct contains various attributes and their values of a cluster link.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-meet/>](https://redis.io/commands/cluster-meet/)
    #[must_use]
    fn cluster_meet(
        self,
        ip: impl Args,
        port: u16,
        cluster_bus_port: Option<u16>,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLUSTER")
                .arg("MEET")
                .arg(ip)
                .arg(port)
                .arg(cluster_bus_port),
        )
    }

    /// This command returns the unique, auto-generated identifier that is associated with the connected cluster node.
    ///
    /// # Return
    ///  The node id.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-myid/>](https://redis.io/commands/cluster-myid/)
    #[must_use]
    fn cluster_myid<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("MYID"))
    }

    /// Each node in a Redis Cluster has its view of the current cluster configuration,
    /// given by the set of known nodes, the state of the connection we have with such nodes,
    /// their flags, properties and assigned slots, and so forth.
    ///
    /// This command provides all this information, that is, the current cluster configuration of the node we are contacting,
    /// in a serialization format which happens to be exactly the same as the one used by Redis Cluster itself
    /// in order to store on disk the cluster state (however the on disk cluster state has a few additional info appended at the end).
    ///
    /// # Return
    /// The serialized cluster configuration.
    /// The output of the command is just a space-separated CSV string, where each line represents a node in the cluster.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-nodes/>](https://redis.io/commands/cluster-nodes/)
    #[must_use]
    fn cluster_nodes<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("NODES"))
    }

    /// The command provides a list of replica nodes replicating from the specified master node.
    ///
    /// # Return
    /// The command returns data in the same format as [`cluster_nodes`](ClusterCommands::cluster_nodes).
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-replicas/>](https://redis.io/commands/cluster-replicas/)
    #[must_use]
    fn cluster_replicas<R: Response>(self, node_id: impl Args) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("REPLICAS").arg(node_id))
    }

    /// The command reconfigures a node as a replica of the specified master.
    /// If the node receiving the command is an empty master, as a side effect of the command, the node role is changed from master to replica.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-replicate/>](https://redis.io/commands/cluster-replicate/)
    #[must_use]
    fn cluster_replicate(self, node_id: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("REPLICATE").arg(node_id))
    }

    /// Reset a Redis Cluster node, in a more or less drastic way depending on the reset type, that can be hard or soft.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-reset/>](https://redis.io/commands/cluster-reset/)
    #[must_use]
    fn cluster_reset(self, reset_type: ClusterResetType) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("RESET").arg(reset_type))
    }

    /// Forces a node to save the nodes.conf configuration on disk.
    /// Before to return the command calls `fsync(2)` in order to make sure the configuration is flushed on the computer disk.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-saveconfig/>](https://redis.io/commands/cluster-saveconfig/)
    #[must_use]
    fn cluster_saveconfig(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLUSTER").arg("SAVECONFIG"))
    }

    /// This command sets a specific config epoch in a fresh node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-set-config-epoch/>](https://redis.io/commands/cluster-set-config-epoch/)
    #[must_use]
    fn cluster_set_config_epoch(self, config_epoch: u64) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLUSTER").arg("SET-CONFIG-EPOCH").arg(config_epoch),
        )
    }

    /// This command is responsible of changing the state of a hash slot in the receiving node in different ways.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-setslot/>](https://redis.io/commands/cluster-setslot/)
    #[must_use]
    fn cluster_setslot(
        self,
        slot: u16,
        subcommand: ClusterSetSlotSubCommand,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLUSTER").arg("SETSLOT").arg(slot).arg(subcommand),
        )
    }

    /// This command returns details about the shards of the cluster.
    ///
    /// # Return
    /// A list of shard information for each shard (slot ranges & shard nodes)
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-shards/>](https://redis.io/commands/cluster-shards/)
    #[must_use]
    fn cluster_shards<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("SHARDS"))
    }

    /// This command returns details details about which cluster slots map to which Redis instances.
    ///
    /// # Return
    /// A nested list of slot ranges with networking information.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-slots/>](https://redis.io/commands/cluster-slots/)
    fn cluster_slots<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("SLOTS"))
    }

    /// Enables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readonly/>](https://redis.io/commands/readonly/)
    #[must_use]
    fn readonly(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("READONLY"))
    }

    /// Disables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readwrite/>](https://redis.io/commands/readwrite/)
    #[must_use]
    fn readwrite(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("READWRITE"))
    }
}

/// Result for the [`cluster_bumpepoch`](ClusterCommands::cluster_bumpepoch) command
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterBumpEpochResult {
    /// if the epoch was incremented
    Bumped,
    /// if the node already has the greatest config epoch in the cluster.
    Still,
}

/// Options for the [`cluster_failover`](ClusterCommands::cluster_failover) command
#[derive(Default)]
pub enum ClusterFailoverOption {
    /// No option
    #[default]
    Default,
    /// FORCE option: manual failover when the master is down
    Force,
    /// TAKEOVER option: manual failover without cluster consensus
    Takeover,
}

impl Args for ClusterFailoverOption {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ClusterFailoverOption::Default => {}
            ClusterFailoverOption::Force => {
                args.arg("FORCE");
            }
            ClusterFailoverOption::Takeover => {
                args.arg("TAKEOVER");
            }
        }
    }
}

/// Cluster state used in the `cluster_state` field of [`ClusterInfo`](ClusterInfo)
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterState {
    /// State is `ok` if the node is able to receive queries.
    Ok,
    /// `fail` if there is at least one hash slot which is unbound (no node associated),
    /// in error state (node serving it is flagged with FAIL flag),
    /// or if the majority of masters can't be reached by this node.
    Fail,
}

/// Result for the [`cluster_info`](ClusterCommands::cluster_info) command
#[derive(Deserialize)]
pub struct ClusterInfo {
    /// State is ok if the node is able to receive queries.
    /// fail if there is at least one hash slot which is unbound (no node associated),
    /// in error state (node serving it is flagged with FAIL flag),
    /// or if the majority of masters can't be reached by this node.
    pub cluster_state: ClusterState,

    /// Number of slots which are associated to some node (not unbound).
    /// This number should be 16384 for the node to work properly,
    /// which means that each hash slot should be mapped to a node.
    pub cluster_slots_assigned: usize,

    /// Number of hash slots mapping to a node not in FAIL or PFAIL state.
    pub cluster_slots_ok: usize,

    /// Number of hash slots mapping to a node in PFAIL state.
    /// Note that those hash slots still work correctly,
    /// as long as the PFAIL state is not promoted to FAIL by the failure detection algorithm.
    /// PFAIL only means that we are currently not able to talk with the node,
    /// but may be just a transient error.
    pub cluster_slots_pfail: usize,

    /// Number of hash slots mapping to a node in FAIL state.
    /// If this number is not zero the node is not able to serve queries
    /// unless cluster-require-full-coverage is set to no in the configuration.
    pub cluster_slots_fail: usize,

    /// The total number of known nodes in the cluster,
    /// including nodes in HANDSHAKE state that may not currently be proper members of the cluster.
    pub cluster_known_nodes: usize,

    /// The number of master nodes serving at least one hash slot in the cluster.
    pub cluster_size: usize,

    /// The local Current Epoch variable.
    /// This is used in order to create unique increasing version numbers during fail overs.
    pub cluster_current_epoch: usize,

    /// The Config Epoch of the node we are talking with.
    /// This is the current configuration version assigned to this node.
    pub cluster_my_epoch: u64,

    /// Number of messages sent via the cluster node-to-node binary bus.
    pub cluster_stats_messages_sent: usize,

    /// Number of messages received via the cluster node-to-node binary bus.
    pub cluster_stats_messages_received: usize,

    /// Accumulated count of cluster links freed due to exceeding the `cluster-link-sendbuf-limit` configuration.
    pub total_cluster_links_buffer_limit_exceeded: usize,

    /// Cluster bus PING sent (not to be confused with the client command [`ping`](crate::commands::ConnectionCommands::ping)).
    pub cluster_stats_messages_ping_sent: usize,

    /// Cluster bus PING received (not to be confused with the client command [`ping`](crate::commands::ConnectionCommands::ping)).
    pub cluster_stats_messages_ping_received: usize,

    /// PONG sent (reply to PING).
    pub cluster_stats_messages_pong_sent: usize,

    /// PONG received (reply to PING).
    pub cluster_stats_messages_pong_received: usize,

    /// Handshake message sent to a new node, either through gossip or [`cluster_meet`](crate::commands::ClusterCommands::cluster_meet).
    pub cluster_stats_messages_meet_sent: usize,

    /// Handshake message sent to a new node, either through gossip or [`cluster_meet`](crate::commands::ClusterCommands::cluster_meet).
    pub cluster_stats_messages_meet_received: usize,

    /// Mark node xxx as failing.
    pub cluster_stats_messages_fail_sent: usize,

    /// Mark node xxx as failing.    
    pub cluster_stats_messages_fail_received: usize,

    /// Pub/Sub Publish propagation, see [`Pubsub`](https://redis.io/topics/pubsub#pubsub).  
    pub cluster_stats_messages_publish_sent: usize,

    /// Pub/Sub Publish propagation, see [`Pubsub`](https://redis.io/topics/pubsub#pubsub).  
    pub cluster_stats_messages_publish_received: usize,

    /// Replica initiated leader election to replace its master.
    pub cluster_stats_messages_auth_req_sent: usize,

    /// Replica initiated leader election to replace its master.
    pub cluster_stats_messages_auth_req_received: usize,

    /// Message indicating a vote during leader election.
    pub cluster_stats_messages_auth_ack_sent: usize,

    /// Message indicating a vote during leader election.
    pub cluster_stats_messages_auth_ack_received: usize,

    /// Another node slots configuration.
    pub cluster_stats_messages_update_sent: usize,

    /// Another node slots configuration.
    pub cluster_stats_messages_update_received: usize,

    /// Pause clients for manual failover.
    pub cluster_stats_messages_mfstart_sent: usize,

    /// Pause clients for manual failover.
    pub cluster_stats_messages_mfstart_received: usize,

    /// Module cluster API message.
    pub cluster_stats_messages_module_sent: usize,

    /// Module cluster API message.
    pub cluster_stats_messages_module_received: usize,

    /// Pub/Sub Publish shard propagation, see [`Sharded Pubsub`](https://redis.io/topics/pubsub#sharded-pubsub).
    pub cluster_stats_messages_publishshard_sent: usize,

    /// Pub/Sub Publish shard propagation, see [`Sharded Pubsub`](https://redis.io/topics/pubsub#sharded-pubsub).
    pub cluster_stats_messages_publishshard_received: usize,
}

/// This link is established by the local node to the peer, or accepted by the local node from the peer.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterLinkDirection {
    To,
    From,
}

/// Result for the [`cluster_links`](ClusterCommands::cluster_links) command
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClusterLinkInfo {
    /// This link is established by the local node to the peer,
    /// or accepted by the local node from the peer.
    pub direction: ClusterLinkDirection,
    /// The node id of the peer.
    pub node: String,
    /// Creation time of the link. (In the case of a to link,
    /// this is the time when the TCP link is created by the local node,
    /// not the time when it is actually established.)
    pub create_time: u64,
    /// Events currently registered for the link. `r` means readable event, `w` means writable event.
    pub events: String,
    /// Allocated size of the link's send buffer,
    /// which is used to buffer outgoing messages toward the peer.
    pub send_buffer_allocated: usize,
    /// Size of the portion of the link's send buffer that is currently holding data(messages).
    pub send_buffer_used: usize,
}

/// Type of [`cluster reset`](ClusterCommands::cluster_reset)
pub enum ClusterResetType {
    Hard,
    Soft,
}

impl Args for ClusterResetType {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ClusterResetType::Hard => args.arg("HARD"),
            ClusterResetType::Soft => args.arg("SOFT"),
        };
    }
}

/// Subcommand for the [`cluster_setslot`](ClusterCommands::cluster_setslot) command.
pub enum ClusterSetSlotSubCommand {
    /// Set a hash slot in importing state.
    Importing { node_id: String },
    /// Set a hash slot in migrating state.
    Migrating { node_id: String },
    /// Bind the hash slot to a different node.
    Node { node_id: String },
    /// Clear any importing / migrating state from hash slot.
    Stable,
}

impl Args for ClusterSetSlotSubCommand {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ClusterSetSlotSubCommand::Importing { node_id } => args.arg("IMPORTING").arg(node_id),
            ClusterSetSlotSubCommand::Migrating { node_id } => args.arg("MIGRATING").arg(node_id),
            ClusterSetSlotSubCommand::Node { node_id } => args.arg("NODE").arg(node_id),
            ClusterSetSlotSubCommand::Stable => args.arg("STABLE"),
        };
    }
}

/// Result for the [`cluster_shards`](ClusterCommands::cluster_shards) command.
#[derive(Debug, Deserialize)]
pub struct ClusterShardResult {
    #[serde(deserialize_with = "deserialize_vec_of_pairs")]
    pub slots: Vec<(u16, u16)>,
    pub nodes: Vec<ClusterNodeResult>,
}

/// Cluster node result for the [`cluster_shards`](ClusterCommands::cluster_shards) command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClusterNodeResult {
    /// The unique node id for this particular node.
    pub id: String,

    /// The preferred endpoint to reach the node
    pub endpoint: String,

    /// The IP address to send requests to for this node.
    pub ip: String,

    /// The TCP (non-TLS) port of the node. At least one of port or tls-port will be present.
    pub port: Option<u16>,

    /// The announced hostname to send requests to for this node.
    pub hostname: Option<String>,

    /// The TLS port of the node. At least one of port or tls-port will be present.
    pub tls_port: Option<u16>,

    /// The replication role of this node.
    pub role: String,

    /// The replication offset of this node.
    /// This information can be used to send commands to the most up to date replicas.
    pub replication_offset: usize,

    /// Either `online`, `failed`, or `loading`.
    /// This information should be used to determine which nodes should be sent traffic.
    /// The loading health state should be used to know that a node is not currently eligible to serve traffic,
    /// but may be eligible in the future.
    pub health: ClusterHealthStatus,
}

/// Cluster health status for the [`cluster_shards`](ClusterCommands::cluster_shards) command.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClusterHealthStatus {
    Online,
    Failed,
    Fail,
    Loading,
}

/// Result for the [`cluster_slots`](ClusterCommands::cluster_slots) command.
#[derive(Debug)]
pub struct LegacyClusterShardResult {
    pub slot: (u16, u16),
    pub nodes: Vec<LegacyClusterNodeResult>,
}

impl<'de> Deserialize<'de> for LegacyClusterShardResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = LegacyClusterShardResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("LegacyClusterShardResult")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let Some(start_slot_range) = seq.next_element::<u16>()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let Some(end_slot_range) = seq.next_element::<u16>()? else {
                    return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                };

                let mut nodes = Vec::new();
                while let Some(node) = seq.next_element::<LegacyClusterNodeResult>()? {
                    nodes.push(node);
                }

                Ok(LegacyClusterShardResult {
                    slot: (start_slot_range, end_slot_range),
                    nodes,
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

/// Cluster node result for the [`cluster_slots`](ClusterCommands::cluster_slots) command.
#[derive(Debug)]
pub struct LegacyClusterNodeResult {
    /// The node ID
    pub id: String,

    /// Preferred endpoint (Either an IP address, hostname, or NULL)
    pub preferred_endpoint: String,

    /// The IP address to send requests to for this node.
    pub ip: String,

    /// When a node has an announced hostname but the primary endpoint is not set to hostname.
    pub hostname: Option<String>,

    /// Port number
    pub port: u16,
}

impl<'de> Deserialize<'de> for LegacyClusterNodeResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = LegacyClusterNodeResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("LegacyClusterNodeResult")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let Some(preferred_endpoint) = seq.next_element::<String>()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let Some(port) = seq.next_element::<u16>()? else {
                    return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                };

                let Some(id) = seq.next_element::<String>()? else {
                    return Err(de::Error::invalid_length(2, &"more elements in sequence"));
                };

                let additional_data = seq.next_element::<HashMap<String, String>>()?;

                let (ip, hostname) = if let Some(mut additional_data) = additional_data {
                    (
                        additional_data
                            .remove("id")
                            .unwrap_or(preferred_endpoint.clone()),
                        additional_data.remove("hostname"),
                    )
                } else {
                    (preferred_endpoint.clone(), None)
                };

                Ok(LegacyClusterNodeResult {
                    id,
                    preferred_endpoint,
                    ip,
                    hostname,
                    port,
                })
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}
