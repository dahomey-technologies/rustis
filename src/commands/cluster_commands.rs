use std::collections::HashMap;

use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CommandArgs, FromSingleValueArray, FromValue, HashMapExt, IntoArgs,
        KeyValueArgOrCollection, SingleArg, SingleArgOrCollection, Value,
    },
    Error, Result,
};

/// A group of Redis commands related to [`Cluster Management`](https://redis.io/docs/management/scaling/)
/// # See Also
/// [Redis Cluster Management commands](https://redis.io/commands/?group=cluster)
/// [Redis cluster specification](https://redis.io/docs/reference/cluster-spec/)
pub trait ClusterCommands {
    /// When a cluster client receives an -ASK redirect,
    /// the ASKING command is sent to the target node followed by the command which was redirected.
    /// This is normally done automatically by cluster clients.
    ///
    /// # See Also
    /// [<https://redis.io/commands/asking/>](https://redis.io/commands/asking/)
    #[must_use]
    fn asking(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
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
    fn cluster_addslots<S>(&mut self, slots: S) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        S: SingleArgOrCollection<u16>,
    {
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
    fn cluster_addslotsrange<S>(&mut self, slots: S) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        S: KeyValueArgOrCollection<u16, u16>,
    {
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
    fn cluster_bumpepoch(&mut self) -> PreparedCommand<Self, ClusterBumpEpochResult>
    where
        Self: Sized,
    {
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
    fn cluster_count_failure_reports<I>(&mut self, node_id: I) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        I: SingleArg,
    {
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
    fn cluster_countkeysinslot(&mut self, slot: usize) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLUSTER").arg("COUNTKEYSINSLOT").arg(slot))
    }

    /// In Redis Cluster, each node keeps track of which master is serving a particular hash slot.
    /// This command asks a particular Redis Cluster node to forget which master
    ///  is serving the hash slots specified as arguments.

    /// # See Also
    /// [<https://redis.io/commands/cluster-delslots/>](https://redis.io/commands/cluster-delslots/)
    #[must_use]
    fn cluster_delslots<S>(&mut self, slots: S) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        S: SingleArgOrCollection<u16>,
    {
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
    fn cluster_delslotsrange<S>(&mut self, slots: S) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        S: KeyValueArgOrCollection<u16, u16>,
    {
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
    fn cluster_failover(&mut self, option: ClusterFailoverOption) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLUSTER").arg("FAILOVER").arg(option))
    }

    /// Deletes all slots from a node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-flushslots/>](https://redis.io/commands/cluster-flushslots/)
    #[must_use]
    fn cluster_flushslots(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLUSTER").arg("FLUSHSLOTS"))
    }

    /// The command is used in order to remove a node, specified via its node ID,
    /// from the set of known nodes of the Redis Cluster node receiving the command.
    /// In other words the specified node is removed from the nodes table of the node receiving the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-forget/>](https://redis.io/commands/cluster-forget/)
    #[must_use]
    fn cluster_forget<I>(&mut self, node_id: I) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        I: SingleArg,
    {
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
    fn cluster_getkeysinslot(&mut self, slot: u16, count: usize) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
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
    fn cluster_info(&mut self, slot: u16, count: usize) -> PreparedCommand<Self, ClusterInfo>
    where
        Self: Sized,
    {
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
    fn cluster_keyslot<K>(&mut self, key: K) -> PreparedCommand<Self, u16>
    where
        Self: Sized,
        K: SingleArg,
    {
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
    fn cluster_links<I>(&mut self) -> PreparedCommand<Self, Vec<I>>
    where
        Self: Sized,
        I: FromSingleValueArray<ClusterLinkInfo>,
    {
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
    fn cluster_meet<IP>(
        &mut self,
        ip: IP,
        port: u16,
        cluster_bus_port: Option<u16>,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        IP: SingleArg,
    {
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
    fn cluster_myid<N>(&mut self) -> PreparedCommand<Self, N>
    where
        Self: Sized,
        N: FromValue,
    {
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
    fn cluster_nodes<R>(&mut self) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        R: FromValue,
    {
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
    fn cluster_replicas<I, R>(&mut self, node_id: I) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        I: SingleArg,
        R: FromValue,
    {
        prepare_command(self, cmd("CLUSTER").arg("REPLICAS").arg(node_id))
    }

    /// The command reconfigures a node as a replica of the specified master.
    /// If the node receiving the command is an empty master, as a side effect of the command, the node role is changed from master to replica.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-replicate/>](https://redis.io/commands/cluster-replicate/)
    #[must_use]
    fn cluster_replicate<I>(&mut self, node_id: I) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        I: SingleArg,
    {
        prepare_command(self, cmd("CLUSTER").arg("REPLICATE").arg(node_id))
    }

    /// Reset a Redis Cluster node, in a more or less drastic way depending on the reset type, that can be hard or soft.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-reset/>](https://redis.io/commands/cluster-reset/)
    #[must_use]
    fn cluster_reset(&mut self, reset_type: ClusterResetType) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLUSTER").arg("RESET").arg(reset_type))
    }

    /// Forces a node to save the nodes.conf configuration on disk.
    /// Before to return the command calls `fsync(2)` in order to make sure the configuration is flushed on the computer disk.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-saveconfig/>](https://redis.io/commands/cluster-saveconfig/)
    #[must_use]
    fn cluster_saveconfig(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLUSTER").arg("SAVECONFIG"))
    }

    /// This command sets a specific config epoch in a fresh node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-set-config-epoch/>](https://redis.io/commands/cluster-set-config-epoch/)
    #[must_use]
    fn cluster_set_config_epoch(&mut self, config_epoch: u64) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
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
        &mut self,
        slot: u16,
        subcommand: ClusterSetSlotSubCommand,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
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
    fn cluster_shards<S>(&mut self) -> PreparedCommand<Self, S>
    where
        Self: Sized,
        S: FromSingleValueArray<ClusterShardResult>,
    {
        prepare_command(self, cmd("CLUSTER").arg("SHARDS"))
    }

    /// Enables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readonly/>](https://redis.io/commands/readonly/)
    #[must_use]
    fn readonly(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("READONLY"))
    }

    /// Disables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readwrite/>](https://redis.io/commands/readwrite/)
    #[must_use]
    fn readwrite(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("READWRITE"))
    }
}

/// Result for the [`cluster_bumpepoch`](ClusterCommands::cluster_bumpepoch) command
pub enum ClusterBumpEpochResult {
    /// if the epoch was incremented
    Bumped,
    /// if the node already has the greatest config epoch in the cluster.
    Still,
}

impl FromValue for ClusterBumpEpochResult {
    fn from_value(value: Value) -> Result<Self> {
        let result: String = value.into()?;
        match result.as_str() {
            "BUMPED" => Ok(Self::Bumped),
            "STILL" => Ok(Self::Still),
            _ => Err(Error::Client(
                "Unexpected result for command 'CLUSTER BUMPEPOCH'".to_owned(),
            )),
        }
    }
}

/// Options for the [`cluster_failover`](ClusterCommands::cluster_failover) command
pub enum ClusterFailoverOption {
    /// No option
    Default,
    /// FORCE option: manual failover when the master is down
    Force,
    /// TAKEOVER option: manual failover without cluster consensus
    Takeover,
}

impl Default for ClusterFailoverOption {
    fn default() -> Self {
        Self::Default
    }
}

impl IntoArgs for ClusterFailoverOption {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ClusterFailoverOption::Default => args,
            ClusterFailoverOption::Force => args.arg("FORCE"),
            ClusterFailoverOption::Takeover => args.arg("TAKEOVER"),
        }
    }
}

/// Cluster state used in the `cluster_state` field of [`ClusterInfo`](ClusterInfo)
pub enum ClusterState {
    /// State is `ok` if the node is able to receive queries.
    Ok,
    /// `fail` if there is at least one hash slot which is unbound (no node associated),
    /// in error state (node serving it is flagged with FAIL flag),
    /// or if the majority of masters can't be reached by this node.
    Fail,
}

impl FromValue for ClusterState {
    fn from_value(value: Value) -> Result<Self> {
        match value.into::<String>()?.as_str() {
            "ok" => Ok(ClusterState::Ok),
            "fail" => Ok(ClusterState::Fail),
            _ => Err(Error::Client("Unexpected ClusterState result".to_owned())),
        }
    }
}

/// Result for the [`cluster_info`](ClusterCommands::cluster_info) command
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

impl FromValue for ClusterInfo {
    fn from_value(value: Value) -> Result<Self> {
        let lines: String = value.into()?;
        let mut values = lines
            .split("\r\n")
            .filter(|line| line.is_empty() || line.starts_with('#'))
            .map(|line| {
                let mut parts = line.split(':');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some(key), Some(value), None) => {
                        Ok((key.to_owned(), Value::BulkString(value.as_bytes().to_vec())))
                    }
                    _ => Err(Error::Client(
                        "Unexpected result for cluster_info".to_owned(),
                    )),
                }
            })
            .collect::<Result<HashMap<String, Value>>>()?;

        Ok(Self {
            cluster_state: values.remove_or_default("cluster_state").into()?,
            cluster_slots_assigned: values.remove_or_default("cluster_slots_assigned").into()?,
            cluster_slots_ok: values.remove_or_default("cluster_slots_ok").into()?,
            cluster_slots_pfail: values.remove_or_default("cluster_slots_pfail").into()?,
            cluster_slots_fail: values.remove_or_default("cluster_slots_fail").into()?,
            cluster_known_nodes: values.remove_or_default("cluster_known_nodes").into()?,
            cluster_size: values.remove_or_default("cluster_size").into()?,
            cluster_current_epoch: values.remove_or_default("cluster_current_epoch").into()?,
            cluster_my_epoch: values.remove_or_default("cluster_my_epoch").into()?,
            cluster_stats_messages_sent: values
                .remove_or_default("cluster_stats_messages_sent")
                .into()?,
            cluster_stats_messages_received: values
                .remove_or_default("cluster_stats_messages_received")
                .into()?,
            total_cluster_links_buffer_limit_exceeded: values
                .remove_or_default("total_cluster_links_buffer_limit_exceeded")
                .into()?,
            cluster_stats_messages_ping_sent: values
                .remove_or_default("cluster_stats_messages_ping_sent")
                .into()?,
            cluster_stats_messages_ping_received: values
                .remove_or_default("cluster_stats_messages_ping_received")
                .into()?,
            cluster_stats_messages_pong_sent: values
                .remove_or_default("cluster_stats_messages_pong_sent")
                .into()?,
            cluster_stats_messages_pong_received: values
                .remove_or_default("cluster_stats_messages_pong_received")
                .into()?,
            cluster_stats_messages_meet_sent: values
                .remove_or_default("cluster_stats_messages_meet_sent")
                .into()?,
            cluster_stats_messages_meet_received: values
                .remove_or_default("cluster_stats_messages_meet_received")
                .into()?,
            cluster_stats_messages_fail_sent: values
                .remove_or_default("cluster_stats_messages_fail_sent")
                .into()?,
            cluster_stats_messages_fail_received: values
                .remove_or_default("cluster_stats_messages_fail_received")
                .into()?,
            cluster_stats_messages_publish_sent: values
                .remove_or_default("cluster_stats_messages_publish_sent")
                .into()?,
            cluster_stats_messages_publish_received: values
                .remove_or_default("cluster_stats_messages_publish_received")
                .into()?,
            cluster_stats_messages_auth_req_sent: values
                .remove_or_default("cluster_stats_messages_auth-req_sent")
                .into()?,
            cluster_stats_messages_auth_req_received: values
                .remove_or_default("cluster_stats_messages_auth-req_received")
                .into()?,
            cluster_stats_messages_auth_ack_sent: values
                .remove_or_default("cluster_stats_messages_auth-ack_sent")
                .into()?,
            cluster_stats_messages_auth_ack_received: values
                .remove_or_default("cluster_stats_messages_auth-ack_received")
                .into()?,
            cluster_stats_messages_update_sent: values
                .remove_or_default("cluster_stats_messages_update_sent")
                .into()?,
            cluster_stats_messages_update_received: values
                .remove_or_default("cluster_stats_messages_update_received")
                .into()?,
            cluster_stats_messages_mfstart_sent: values
                .remove_or_default("cluster_stats_messages_mfstart_sent")
                .into()?,
            cluster_stats_messages_mfstart_received: values
                .remove_or_default("cluster_stats_messages_mfstart_received")
                .into()?,
            cluster_stats_messages_module_sent: values
                .remove_or_default("cluster_stats_messages_mfstart_received")
                .into()?,
            cluster_stats_messages_module_received: values
                .remove_or_default("cluster_stats_messages_module_received")
                .into()?,
            cluster_stats_messages_publishshard_sent: values
                .remove_or_default("cluster_stats_messages_publishshard_sent")
                .into()?,
            cluster_stats_messages_publishshard_received: values
                .remove_or_default("cluster_stats_messages_publishshard_received")
                .into()?,
        })
    }
}

/// This link is established by the local node to the peer, or accepted by the local node from the peer.
pub enum ClusterLinkDirection {
    To,
    From,
}

impl FromValue for ClusterLinkDirection {
    fn from_value(value: Value) -> Result<Self> {
        match value.into::<String>()?.as_str() {
            "to" => Ok(ClusterLinkDirection::To),
            "from" => Ok(ClusterLinkDirection::From),
            _ => Err(Error::Client(
                "Unexpected ClusterLinkDirection result".to_owned(),
            )),
        }
    }
}

/// Result for the [`cluster_links`](ClusterCommands::cluster_links) command
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

impl FromValue for ClusterLinkInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            direction: values.remove_or_default("direction").into()?,
            node: values.remove_or_default("node").into()?,
            create_time: values.remove_or_default("create-time").into()?,
            events: values.remove_or_default("events").into()?,
            send_buffer_allocated: values.remove_or_default("send-buffer-allocated").into()?,
            send_buffer_used: values.remove_or_default("send-buffer-used").into()?,
        })
    }
}

/// Type of [`cluster reset`](ClusterCommands::cluster_reset)
pub enum ClusterResetType {
    Hard,
    Soft,
}

impl IntoArgs for ClusterResetType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ClusterResetType::Hard => args.arg("HARD"),
            ClusterResetType::Soft => args.arg("SOFT"),
        }
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

impl IntoArgs for ClusterSetSlotSubCommand {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ClusterSetSlotSubCommand::Importing { node_id } => args.arg("IMPORTING").arg(node_id),
            ClusterSetSlotSubCommand::Migrating { node_id } => args.arg("MIGRATING").arg(node_id),
            ClusterSetSlotSubCommand::Node { node_id } => args.arg("NODE").arg(node_id),
            ClusterSetSlotSubCommand::Stable => args.arg("STABLE"),
        }
    }
}

/// Result for the [`cluster_shards`](ClusterCommands::cluster_shards) command.
#[derive(Debug)]
pub struct ClusterShardResult {
    pub slots: Vec<(u16, u16)>,
    pub nodes: Vec<ClusterNodeResult>,
}

impl FromValue for ClusterShardResult {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            slots: values.remove_with_result("slots")?.into()?,
            nodes: values.remove_with_result("nodes")?.into()?,
        })
    }
}

/// Cluster node result for the [`cluster_shards`](ClusterCommands::cluster_shards) command.
#[derive(Debug)]
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

impl FromValue for ClusterNodeResult {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            id: values.remove_with_result("id")?.into()?,
            endpoint: values.remove_with_result("endpoint")?.into()?,
            ip: values.remove_with_result("ip")?.into()?,
            port: values.remove_or_default("port").into()?,
            hostname: values.remove_or_default("hostname").into()?,
            tls_port: values.remove_or_default("tls-port").into()?,
            role: values.remove_with_result("role")?.into()?,
            replication_offset: values.remove_with_result("replication-offset")?.into()?,
            health: values.remove_with_result("health")?.into()?,
        })
    }
}

/// Cluster health status for the [`cluster_shards`](ClusterCommands::cluster_shards) command.
#[derive(Debug)]
pub enum ClusterHealthStatus {
    Online,
    Failed,
    Loading,
}

impl FromValue for ClusterHealthStatus {
    fn from_value(value: Value) -> Result<Self> {
        match value.into::<String>()?.as_str() {
            "online" => Ok(Self::Online),
            "failed" => Ok(Self::Failed),
            "loading" => Ok(Self::Loading),
            _ => Err(Error::Client(
                "Unexpected result for ClusterHealthStatus".to_owned(),
            )),
        }
    }
}
