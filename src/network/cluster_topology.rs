//! Who serves which slot, and how a command finds the node that owns it.
//!
//! The topology is two collections that must stay sorted: the nodes by id, the
//! slot ranges by their first slot. Every lookup below is a binary search over
//! one of them, so an insertion that appended instead of ordering would not
//! fail — it would make the node it added invisible, and route the slots it owns
//! to whoever the search landed on instead. That is why nothing here takes a
//! `&mut Vec`: the two collections are private, and the only ways in place a
//! node or a slot range where the search will find it.

use crate::{
    ClientError, ConnectionState, Error, StandaloneConnection,
    client::{ClusterConfig, Config, ReadPreference},
    commands::{
        ClusterCommands, ClusterHealthStatus, ClusterNodeResult, ClusterShardResult,
        InternalCommands, LegacyClusterShardResult,
    },
    network::Version,
    resp::{Command, RespResponse},
};
use futures_util::{FutureExt, future};
use rand::RngExt;
use smallvec::{SmallVec, smallvec};
use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    sync::Arc,
    task::Poll,
};
use tracing::{debug, warn};

/// Host and port, as the discovery reply names a node and as a redirection
/// points at one.
pub(super) type ClusterNodeAddress = (String, u16);

#[derive(Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[repr(transparent)]
pub(super) struct NodeId(Arc<str>);

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(Arc::from(value))
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub(super) struct Node {
    pub id: NodeId,
    pub is_master: bool,
    pub address: ClusterNodeAddress,
    pub connection: StandaloneConnection,
    pub is_dirty: bool,
}

impl Node {
    /// `reply_skip` is a held-back `CLIENT REPLY SKIP`, emitted on this node right
    /// before the command it silences.
    ///
    /// It has to travel with the command rather than being routed on its own: it
    /// suppresses the reply of whatever the node receives next, so sending it to a
    /// node the command never reaches would leave that node swallowing the reply of
    /// some unrelated later command.
    pub(super) async fn feed(
        &mut self,
        command: &Command,
        reply_skip: Option<&Command>,
    ) -> crate::Result<()> {
        if let Some(reply_skip) = reply_skip {
            self.connection.feed(reply_skip, &[]).await?;
        }
        self.connection.feed(command, &[]).await?;
        self.is_dirty = true;
        Ok(())
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("is_master", &self.is_master)
            .field("tag", &self.connection.tag())
            .finish()
    }
}

#[derive(Debug)]
pub(super) struct SlotRange {
    pub slot_range: (u16, u16),
    /// node ids of the shard that owns the slot range,
    /// the first node id being the master node id
    pub node_ids: SmallVec<[NodeId; 6]>,
    /// Round-robin cursor over the replicas of the shard, used when the read
    /// preference sends read-only commands to them. It only has to advance, so
    /// it wraps freely: the candidate is picked modulo the replica count.
    pub next_replica: usize,
}

/// Which nodes a command with no key of its own must reach.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum NodeReach {
    /// One node per shard: the command applies per shard, and sending it to a
    /// replica too would apply it twice.
    Masters,
    /// Every node, replicas included — a `CONFIG SET` has to reach them all.
    All,
}

/// The nodes of the cluster and the slots each shard owns.
#[derive(Debug, Default)]
pub(super) struct ClusterTopology {
    /// Ordered by node id: [`Self::node_index_by_id`] binary-searches it.
    nodes: Vec<Node>,
    /// Ordered by first slot: [`Self::slot_range_index`] binary-searches it.
    slot_ranges: Vec<SlotRange>,
}

impl ClusterTopology {
    /// Nodes are addressed by index everywhere the routing decides where a
    /// command goes, so an index only stays valid until the next insertion or
    /// removal. Every caller resolves one and uses it immediately.
    pub(super) fn node(&self, index: usize) -> Option<&Node> {
        self.nodes.get(index)
    }

    pub(super) fn node_mut(&mut self, index: usize) -> Option<&mut Node> {
        self.nodes.get_mut(index)
    }

    /// Whether every node held is a master, i.e. no replica has been connected
    /// yet. A command addressed to all nodes has to bring them in first, or it
    /// would reach a subset and report success.
    pub(super) fn holds_no_replica(&self) -> bool {
        self.nodes.iter().all(|n| n.is_master)
    }

    #[cfg(test)]
    pub(super) fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Test-only: removes one node, reproducing what a refresh leaves behind
    /// when a node disappears while requests are in flight against it.
    #[cfg(test)]
    pub(super) fn drop_node(&mut self, id: &NodeId) {
        self.retain_nodes(|node| node.id != *id);
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[cfg(test)]
    pub(super) fn slot_ranges(&self) -> &[SlotRange] {
        &self.slot_ranges
    }

    /// Adds a node where the id search will find it.
    fn insert_node(&mut self, node: Node) {
        let position = self
            .nodes
            .binary_search_by(|n| n.id.cmp(&node.id))
            .unwrap_or_else(|position| position);
        self.nodes.insert(position, node);
    }

    /// Adds a slot range where the slot search will find it.
    fn insert_slot_range(&mut self, slot_range: SlotRange) {
        let position = self
            .slot_ranges
            .binary_search_by_key(&slot_range.slot_range.0, |s| s.slot_range.0)
            .unwrap_or_else(|position| position);
        self.slot_ranges.insert(position, slot_range);
    }

    /// Drops the nodes a refreshed topology no longer describes. Order-preserving,
    /// so the id search stays valid.
    fn retain_nodes(&mut self, keep: impl FnMut(&Node) -> bool) {
        self.nodes.retain(keep);
    }

    fn clear_slot_ranges(&mut self) {
        self.slot_ranges.clear();
    }

    /// Flushes every node a command was fed to since the last flush, all at
    /// once: the sub-requests of one command sit on different sockets, and
    /// flushing them in turn would make each shard wait for the ones before it.
    ///
    /// A node is marked clean as its future is built, not once that future
    /// resolves. A failed flush is a lost connection, which the whole cluster
    /// connection is torn down over, so there is no half-flushed node to
    /// remember.
    pub(super) async fn flush_fed_nodes(&mut self) -> crate::Result<()> {
        let mut flush_futures = SmallVec::<[_; 16]>::new();

        for node in self.nodes.iter_mut() {
            if node.is_dirty {
                node.is_dirty = false;
                flush_futures.push(node.connection.flush());
            }
        }

        for result in future::join_all(flush_futures).await {
            result?;
        }

        Ok(())
    }

    /// Feeds `command` to each node `reach` names, and reports the ids of the
    /// nodes it reached — one sub-request is owed per id.
    ///
    /// A failure part-way leaves the nodes already fed holding a command whose
    /// reply nobody will file. That is a broken connection either way: the
    /// caller propagates it and the handler tears the cluster connection down.
    pub(super) async fn feed_each(
        &mut self,
        command: &Command,
        reply_skip: Option<&Command>,
        reach: NodeReach,
    ) -> crate::Result<SmallVec<[NodeId; 10]>> {
        let mut fed = SmallVec::<[NodeId; 10]>::new();

        for node in self.nodes.iter_mut() {
            if reach == NodeReach::Masters && !node.is_master {
                continue;
            }
            node.feed(command, reply_skip).await?;
            fed.push(node.id.clone());
        }

        Ok(fed)
    }

    /// Asks each address in turn for the cluster's shape, and returns the first
    /// answer. A node that cannot be reached, cannot state its version, or
    /// refuses the query is skipped: discovery only needs one node to answer.
    pub(super) async fn discover_shards(
        addresses: &[ClusterNodeAddress],
        config: &Config,
    ) -> Option<Vec<ClusterShardResult>> {
        debug!("Discovering cluster shards and slots...");

        for (host, port) in addresses {
            // A dedicated, short-lived discovery connection is not the caller's:
            // it must not replay their database, name or tracking mode.
            let mut connection =
                match StandaloneConnection::connect_control(host, *port, config).await {
                    Ok(connection) => connection,
                    Err(e) => {
                        warn!("Cannot connect to node ({host}:{port}): {e}");
                        continue;
                    }
                };

            let version: crate::Result<Version> = connection.get_version().try_into();
            let Ok(version) = version else {
                warn!(node = %connection.tag(), "Cannot get Redis version");
                continue;
            };

            // From Redis 7.x CLUSTER SLOTS is deprecated in favor of CLUSTER SHARDS
            let shard_info_list = if version.major < 7 {
                connection
                    .cluster_slots()
                    .await
                    .map(convert_from_legacy_shard_description)
            } else {
                connection.cluster_shards().await
            };

            match shard_info_list {
                Ok(shard_info_list) => return Some(shard_info_list),
                Err(e) => warn!(
                    node = %connection.tag(),
                    "Cannot discover cluster shards on node ({host}:{port}): {e}"
                ),
            }
        }

        None
    }

    /// Addresses to try for topology discovery: the nodes currently held —
    /// known to have answered — then the configured seeds as a fallback.
    pub(super) fn discovery_addresses(
        &self,
        cluster_config: &ClusterConfig,
    ) -> Vec<ClusterNodeAddress> {
        let mut addresses: Vec<ClusterNodeAddress> =
            self.nodes.iter().map(|n| n.address.clone()).collect();
        addresses.extend(cluster_config.nodes.iter().cloned());
        addresses
    }

    /// Discovers the cluster from the configured seeds and connects one master
    /// per shard. Replicas are brought in later, by `connect_replicas`.
    pub(super) async fn discover(
        cluster_config: &ClusterConfig,
        config: &Config,
        connection_state: &mut ConnectionState,
    ) -> crate::Result<ClusterTopology> {
        #[cfg_attr(not(test), allow(unused_mut))]
        let Some(mut shard_info_list) = Self::discover_shards(&cluster_config.nodes, config).await
        else {
            return Err(Error::from(ClientError::ClusterConfig));
        };

        // Test-only: build a topology that ignores a node the cluster does know.
        #[cfg(test)]
        if let Some(hook) = &config.cluster_test_hook
            && let Some(hidden_node_id) = hook.take_hidden_node_id()
        {
            shard_info_list.retain(|s| !s.nodes.iter().any(|n| n.id == hidden_node_id));
        }

        let mut topology = ClusterTopology::default();

        for shard_info in shard_info_list.into_iter() {
            let Some(master_info) = shard_info
                .nodes
                .into_iter()
                .find(|n| n.role == "master" && n.health == ClusterHealthStatus::Online)
            else {
                return Err(Error::from(ClientError::ClusterConfig));
            };
            let master_id: NodeId = master_info.id.as_str().into();

            let port = master_info.get_port()?;

            let connection =
                StandaloneConnection::connect(&master_info.ip, port, config, connection_state)
                    .await?;

            for slot_range in shard_info.slots.iter().map(|s| SlotRange {
                slot_range: *s,
                node_ids: smallvec![master_id.clone()],
                next_replica: 0,
            }) {
                topology.insert_slot_range(slot_range);
            }

            topology.insert_node(Node {
                id: master_id.clone(),
                is_master: true,
                address: (master_info.ip, port),
                connection,
                is_dirty: false,
            });
        }

        debug!("Cluster connected: {topology:?}");

        Ok(topology)
    }

    /// Puts a replica connection in `READONLY` mode, which is what makes the node
    /// serve a read instead of answering it with a `MOVED` to its master.
    ///
    /// Nothing is sent when reads stay on the masters: the mode would advertise a
    /// capability the routing never uses. A refusal is logged rather than
    /// propagated — the node then answers reads with a `MOVED`, which the client
    /// follows, so the cluster keeps working.
    async fn set_replica_read_mode(
        connection: &mut StandaloneConnection,
        read_preference: ReadPreference,
    ) {
        if read_preference == ReadPreference::Master {
            return;
        }

        if let Err(e) = connection.readonly().await {
            warn!(node = %connection.tag(), "Cannot enter readonly mode: {e}");
        }
    }

    /// Same, for a node whose role has just changed: a master must be back in
    /// read-write mode.
    async fn set_read_mode_for_role(
        connection: &mut StandaloneConnection,
        is_master: bool,
        read_preference: ReadPreference,
    ) {
        if read_preference == ReadPreference::Master {
            return;
        }

        if is_master {
            if let Err(e) = connection.readwrite().await {
                warn!(node = %connection.tag(), "Cannot leave readonly mode: {e}");
            }
        } else {
            Self::set_replica_read_mode(connection, read_preference).await;
        }
    }

    /// Connects the replicas the cluster describes and files them against the
    /// slot ranges their shard owns, so reads can be routed to them.
    pub(super) async fn connect_replicas(
        &mut self,
        cluster_config: &ClusterConfig,
        config: &Config,
        state_snapshot: &ConnectionState,
    ) -> crate::Result<()> {
        debug!("Connecting replicas...");

        let addresses = self.discovery_addresses(cluster_config);
        let Some(shard_info_list) = Self::discover_shards(&addresses, config).await else {
            return Err(Error::from(ClientError::ClusterConfig));
        };

        for shard_info in shard_info_list {
            for node_info in shard_info.nodes.into_iter().filter(|n| n.role == "replica") {
                let port = node_info.get_port()?;
                let node_id: NodeId = node_info.id.as_str().into();

                // Opened without state, then brought up to the state its siblings
                // are in: `connect` would need the handler's registry, which this
                // path does not have.
                let mut connection =
                    StandaloneConnection::connect_control(&node_info.ip, port, config).await?;
                connection.restore_from_snapshot(state_snapshot).await;

                for slot_range_info in &shard_info.slots {
                    if let Some(slot_range) = self.slot_range_by_slot_mut(slot_range_info.0)
                        && slot_range.slot_range.1 == slot_range_info.1
                    {
                        slot_range.node_ids.push(node_id.clone())
                    }
                }

                Self::set_replica_read_mode(&mut connection, cluster_config.read_preference).await;

                self.insert_node(Node {
                    id: node_id,
                    is_master: false,
                    address: (node_info.ip.clone(), port),
                    connection,
                    is_dirty: false,
                });
            }
        }

        debug!("Cluster replicas connected: {self:?}");

        Ok(())
    }

    /// Applies a discovered topology: keeps the connections still described,
    /// drops the nodes that vanished, connects the ones that joined and rebuilds
    /// the slot map from scratch.
    ///
    /// `shard_info_list` must not be empty — the caller refuses an empty
    /// discovery before reaching here, since applying it would leave the routing
    /// with no node to resolve to.
    pub(super) async fn apply(
        &mut self,
        shard_info_list: Vec<ClusterShardResult>,
        cluster_config: &ClusterConfig,
        config: &Config,
        state_snapshot: &ConnectionState,
    ) -> crate::Result<()> {
        let known = known_node_ids(&shard_info_list);
        self.retain_nodes(|node| {
            known
                .binary_search_by(|id| id.as_str().cmp(node.id.as_ref()))
                .is_ok()
        });

        // create slot_ranges from scratch
        self.clear_slot_ranges();

        // add missing nodes and connect them
        for shard_info in shard_info_list {
            let shard_info = with_master_first(shard_info)?;

            for slot_range in slot_ranges_of(&shard_info) {
                self.insert_slot_range(slot_range);
            }

            for node_info in shard_info.nodes {
                let node_id: NodeId = node_info.id.as_str().into();
                if let Some(node) = self.node_by_id_mut(&node_id) {
                    // refresh is_master flag in case a failover happened
                    let is_master = node_info.role == "master";
                    if is_master != node.is_master {
                        // The connection carries the read mode of the role the node
                        // has just left: a promoted replica would keep advertising a
                        // capability it no longer has, a demoted master would refuse
                        // the reads now routed to it.
                        Self::set_read_mode_for_role(
                            &mut node.connection,
                            is_master,
                            cluster_config.read_preference,
                        )
                        .await;
                    }
                    node.is_master = is_master;
                } else {
                    // add missing node
                    let port = node_info.get_port()?;

                    // A node joining the topology must reach the state its siblings
                    // are in before anything is sent on it, or the caller's tracking,
                    // name and exemptions would silently not apply to its shard.
                    let mut connection =
                        StandaloneConnection::connect_control(&node_info.ip, port, config).await?;
                    connection.restore_from_snapshot(state_snapshot).await;

                    if node_info.role != "master" {
                        Self::set_replica_read_mode(
                            &mut connection,
                            cluster_config.read_preference,
                        )
                        .await;
                    }

                    self.insert_node(Node {
                        id: node_id,
                        is_master: node_info.role == "master",
                        address: (node_info.ip, port),
                        connection,
                        is_dirty: false,
                    });
                }
            }
        }

        debug!("Cluster new setup: {self:?}");

        Ok(())
    }

    /// Awaits the first reply any node has, with the index of the node that sent
    /// it. `None` when the topology holds no node — `select_all` panics on an
    /// empty set, and the network task owns all routing state, so an empty
    /// topology has to be reported rather than polled.
    ///
    /// The inner `Option` is the node's own: `None` there is that node's stream
    /// ending.
    pub(super) async fn read_any(
        &mut self,
    ) -> Option<(usize, Option<crate::Result<RespResponse>>)> {
        if self.nodes.is_empty() {
            return None;
        }

        let read_futures = self.nodes.iter_mut().map(|n| n.connection.read().boxed());
        let (result, node_index, _) = future::select_all(read_futures).await;
        Some((node_index, result))
    }

    /// Same, without awaiting: `Pending` means no node has a reply ready,
    /// `Ready(None)` that there is no node to read from.
    pub(super) fn try_read_any(
        &mut self,
    ) -> Poll<Option<(usize, Option<crate::Result<RespResponse>>)>> {
        if self.nodes.is_empty() {
            return Poll::Ready(None);
        }

        self.nodes
            .iter_mut()
            .enumerate()
            .find_map(|(node_index, node)| match node.connection.try_read() {
                Poll::Ready(result) => Some(Poll::Ready(Some((node_index, result)))),
                Poll::Pending => None,
            })
            .unwrap_or(Poll::Pending)
    }

    pub(super) fn node_index_by_id(&self, id: &NodeId) -> Option<usize> {
        self.nodes.binary_search_by_key(&id, |n| &n.id).ok()
    }

    fn node_by_id_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        let index = self.node_index_by_id(id)?;
        self.nodes.get_mut(index)
    }

    /// Addresses are not indexed: a redirection names one, and the scan is over
    /// as many nodes as the cluster has.
    fn node_index_by_address(&self, address: &ClusterNodeAddress) -> Option<usize> {
        self.nodes.iter().position(|n| n.address == *address)
    }

    pub(super) fn node_by_address(&self, address: &ClusterNodeAddress) -> Option<&Node> {
        self.nodes.iter().find(|n| n.address == *address)
    }

    pub(super) fn holds_address(&self, address: &ClusterNodeAddress) -> bool {
        self.node_by_address(address).is_some()
    }

    #[inline]
    pub(super) fn random_node_index(&self) -> Option<usize> {
        if self.nodes.is_empty() {
            return None;
        }
        Some(rand::rng().random_range(0..self.nodes.len()))
    }

    #[inline]
    fn slot_range_index(&self, slot: u16) -> Option<usize> {
        self.slot_ranges
            .binary_search_by(|s| {
                if s.slot_range.0 > slot {
                    Ordering::Greater
                } else if s.slot_range.1 < slot {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .ok()
    }

    #[inline]
    fn slot_range_by_slot(&self, slot: u16) -> Option<&SlotRange> {
        self.slot_range_index(slot)
            .and_then(|idx| self.slot_ranges.get(idx))
    }

    #[inline]
    fn slot_range_by_slot_mut(&mut self, slot: u16) -> Option<&mut SlotRange> {
        self.slot_range_index(slot)
            .and_then(|idx| self.slot_ranges.get_mut(idx))
    }

    /// The node a command addressing `slot` must be fed to, and whether it has to
    /// be prefixed with an `ASKING`.
    ///
    /// `for_read` asks for the configured read preference to be honoured. It is
    /// the caller's job to answer it only for a command that may legitimately
    /// leave the master.
    pub(super) fn node_index_by_slot(
        &mut self,
        slot: u16,
        ask_reasons: &[(u16, ClusterNodeAddress)],
        for_read: bool,
    ) -> Option<(usize, bool)> {
        let ask_reason = ask_reasons
            .iter()
            .find(|(hash_slot, (_ip, _port))| *hash_slot == slot);

        // An ASK names the node itself: the redirection is the routing decision,
        // and the read preference has nothing to say about it.
        if let Some((_hash_slot, address)) = ask_reason {
            return Some((self.node_index_by_address(address)?, true));
        }

        if for_read && let Some(node_index) = self.replica_node_index_by_slot(slot) {
            return Some((node_index, false));
        }

        let slot_range = self.slot_range_by_slot(slot)?;
        // A slot range names its master first; one with no node routes nowhere.
        let master_node_id = slot_range.node_ids.first()?;
        let node_index = self.node_index_by_id(master_node_id)?;
        Some((node_index, false))
    }

    /// The next replica of the shard owning `slot`, or `None` when the shard has
    /// no connected one — in which case the caller falls back to the master
    /// rather than failing the command.
    fn replica_node_index_by_slot(&mut self, slot: u16) -> Option<usize> {
        let slot_range_index = self.slot_range_index(slot)?;
        let slot_range = self.slot_ranges.get(slot_range_index)?;

        // The master heads the list; everything after it is a replica.
        let replica_ids: SmallVec<[NodeId; 6]> =
            slot_range.node_ids.iter().skip(1).cloned().collect();
        if replica_ids.is_empty() {
            return None;
        }

        let mut cursor = slot_range.next_replica;
        let node_index = select_replica(&replica_ids, &mut cursor, |id| self.node_index_by_id(id))?;

        if let Some(slot_range) = self.slot_ranges.get_mut(slot_range_index) {
            slot_range.next_replica = cursor;
        }

        Some(node_index)
    }
}

/// The ids a discovered topology describes, sorted so the retention filter can
/// binary-search them once per node held instead of scanning the whole reply.
fn known_node_ids(shard_info_list: &[ClusterShardResult]) -> Vec<String> {
    let mut ids = shard_info_list
        .iter()
        .flat_map(|s| s.nodes.iter().map(|n| n.id.clone()))
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

/// Puts the shard's master at index 0, which every later read of the shard
/// relies on: a slot range names its master first, and that is who a command
/// routed by slot is fed to. A shard the server describes with no node, or with
/// no master among them, is a malformed topology rather than something to index.
fn with_master_first(mut shard_info: ClusterShardResult) -> crate::Result<ClusterShardResult> {
    let first_is_master = match shard_info.nodes.first() {
        Some(first) => first.role == "master",
        None => return Err(Error::from(ClientError::ClusterConfig)),
    };

    if !first_is_master {
        let Some(master_idx) = shard_info.nodes.iter().position(|n| n.role == "master") else {
            return Err(Error::from(ClientError::ClusterConfig));
        };
        shard_info.nodes.swap(0, master_idx);
    }

    Ok(shard_info)
}

/// The slot ranges a shard owns, each naming the shard's nodes with the master
/// first. Requires [`with_master_first`] to have run.
fn slot_ranges_of(shard_info: &ClusterShardResult) -> impl Iterator<Item = SlotRange> {
    let node_ids: SmallVec<[NodeId; 6]> = shard_info
        .nodes
        .iter()
        .map(|n| n.id.as_str().into())
        .collect();

    shard_info.slots.iter().map(move |slot_range| SlotRange {
        slot_range: *slot_range,
        node_ids: node_ids.clone(),
        next_replica: 0,
    })
}

/// Rebuilds the modern `CLUSTER SHARDS` shape from a `CLUSTER SLOTS` reply,
/// which Redis before 7 is the only source for. The legacy reply lists one
/// entry per slot range, master first, so entries are grouped by master id.
pub(crate) fn convert_from_legacy_shard_description(
    mut legacy_shards: Vec<LegacyClusterShardResult>,
) -> Vec<ClusterShardResult> {
    // Group by master id, which the legacy reply lists first for each shard.
    // A shard the server sent with no node at all sorts to the front and is
    // skipped below rather than indexed into.
    legacy_shards.sort_by(|s1, s2| {
        s1.nodes
            .first()
            .map(|n| &n.id)
            .cmp(&s2.nodes.first().map(|n| &n.id))
    });

    let mut last_master_id = String::new();
    let mut shards = Vec::new();
    for legacy_shard in legacy_shards {
        let Some(master_id) = legacy_shard.nodes.first().map(|node| node.id.clone()) else {
            continue;
        };
        if master_id != last_master_id {
            last_master_id = master_id;
            shards.push(ClusterShardResult {
                slots: vec![legacy_shard.slot],
                nodes: legacy_shard
                    .nodes
                    .into_iter()
                    .enumerate()
                    .map(|(idx, node)| ClusterNodeResult {
                        id: node.id,
                        endpoint: node.preferred_endpoint.clone(),
                        ip: node.ip,
                        port: Some(node.port),
                        hostname: node.hostname,
                        tls_port: None,
                        role: if idx == 0 {
                            "master".to_owned()
                        } else {
                            "replica".to_owned()
                        },
                        replication_offset: 0,
                        health: ClusterHealthStatus::Online,
                    })
                    .collect(),
            });
        } else if let Some(shard) = shards.last_mut() {
            shard.slots.push(legacy_shard.slot);
        }
    }

    shards
}

/// Picks the next replica of a shard, starting at `cursor` and advancing it past
/// the one returned.
///
/// A replica the topology names may not be connected yet — `AllNodes` is what
/// brings them in when the read preference does not. The whole list is walked
/// from the cursor so a hole does not pin every read of the shard on one node,
/// and `None` — no replica reachable at all — sends the read back to the master.
fn select_replica(
    replica_ids: &[NodeId],
    cursor: &mut usize,
    resolve: impl Fn(&NodeId) -> Option<usize>,
) -> Option<usize> {
    if replica_ids.is_empty() {
        return None;
    }

    for offset in 0..replica_ids.len() {
        let position = cursor.wrapping_add(offset);
        let candidate = replica_ids.get(position.checked_rem(replica_ids.len())?)?;
        if let Some(node_index) = resolve(candidate) {
            *cursor = position.wrapping_add(1);
            return Some(node_index);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{ClusterTopology, NodeId, SlotRange, select_replica};
    use smallvec::smallvec;

    fn slot_range(from: u16, to: u16) -> SlotRange {
        SlotRange {
            slot_range: (from, to),
            node_ids: smallvec!["m".into()],
            next_replica: 0,
        }
    }

    /// The slot search is a binary search, so a range added out of order would
    /// not be reported missing — it would send the slots it owns to whichever
    /// range the search happened to land on. Insertion orders, so the order
    /// cannot be forgotten at a call site.
    #[test]
    fn a_slot_range_is_placed_where_the_search_will_find_it() {
        let mut topology = ClusterTopology::default();
        for (from, to) in [(10923, 16383), (0, 5460), (5461, 10922)] {
            topology.insert_slot_range(slot_range(from, to));
        }

        let starts = topology
            .slot_ranges()
            .iter()
            .map(|s| s.slot_range.0)
            .collect::<Vec<_>>();
        assert_eq!(vec![0, 5461, 10923], starts);

        for (slot, expected) in [
            (0, 0),
            (5460, 0),
            (5461, 5461),
            (10922, 5461),
            (16383, 10923),
        ] {
            assert_eq!(
                Some(expected),
                topology
                    .slot_range_by_slot_mut(slot)
                    .map(|s| s.slot_range.0),
                "slot {slot} must be served by the range starting at {expected}"
            );
        }
    }

    /// A slot no range covers routes nowhere. Answering the neighbouring range
    /// would send the command to a node that replies `MOVED` forever.
    #[test]
    fn a_slot_no_shard_owns_is_served_by_nothing() {
        let mut topology = ClusterTopology::default();
        topology.insert_slot_range(slot_range(0, 100));
        topology.insert_slot_range(slot_range(200, 300));

        assert!(topology.slot_range_by_slot_mut(150).is_none());
        assert!(topology.slot_range_by_slot_mut(301).is_none());
        assert!(topology.slot_range_by_slot_mut(100).is_some());
        assert!(topology.slot_range_by_slot_mut(200).is_some());
    }

    /// Reads of a shard must be spread over its replicas, not pinned on the
    /// first one: a preference that always answered the same node would move
    /// the load instead of sharing it.
    #[test]
    fn replicas_are_picked_in_round_robin() {
        let replicas: Vec<NodeId> = vec!["r1".into(), "r2".into(), "r3".into()];
        let mut cursor = 0;

        let picks = (0..6)
            .map(|_| {
                select_replica(&replicas, &mut cursor, |id| match id.as_ref() {
                    "r1" => Some(10),
                    "r2" => Some(20),
                    "r3" => Some(30),
                    _ => None,
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(
            vec![Some(10), Some(20), Some(30), Some(10), Some(20), Some(30)],
            picks
        );
    }

    /// A replica the topology names but nothing has connected yet must be
    /// stepped over, otherwise every read of the shard falls back to the master
    /// one time out of two.
    #[test]
    fn an_unconnected_replica_is_skipped() {
        let replicas: Vec<NodeId> = vec!["r1".into(), "r2".into()];
        let mut cursor = 0;

        let picks = (0..3)
            .map(|_| {
                select_replica(&replicas, &mut cursor, |id| {
                    (id.as_ref() == "r2").then_some(20)
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(vec![Some(20), Some(20), Some(20)], picks);
    }

    /// A shard with no reachable replica is not a routing failure: the read goes
    /// to the master, which is what the caller would have got anyway.
    #[test]
    fn a_shard_without_a_reachable_replica_selects_nothing() {
        let mut cursor = 0;
        assert_eq!(None, select_replica(&[], &mut cursor, |_| Some(0)));

        let replicas: Vec<NodeId> = vec!["r1".into()];
        assert_eq!(None, select_replica(&replicas, &mut cursor, |_| None));
    }

    /// An empty topology routes nothing rather than picking an index into a
    /// collection that has none: the node task owns all routing state, so a
    /// panic here takes every in-flight command with it.
    #[test]
    fn an_empty_topology_names_no_node() {
        let mut topology = ClusterTopology::default();
        assert!(topology.is_empty());
        assert_eq!(None, topology.random_node_index());
        assert_eq!(None, topology.node_index_by_slot(0, &[], false));
        assert_eq!(None, topology.node_index_by_id(&"m".into()));
    }
}

#[cfg(test)]
mod discovery_tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{
        convert_from_legacy_shard_description, known_node_ids, slot_ranges_of, with_master_first,
    };
    use crate::commands::{
        ClusterHealthStatus, ClusterNodeResult, ClusterShardResult, LegacyClusterNodeResult,
        LegacyClusterShardResult,
    };

    fn node(id: &str, role: &str) -> ClusterNodeResult {
        ClusterNodeResult {
            id: id.to_owned(),
            endpoint: "127.0.0.1".to_owned(),
            ip: "127.0.0.1".to_owned(),
            port: Some(6379),
            hostname: None,
            tls_port: None,
            role: role.to_owned(),
            replication_offset: 0,
            health: ClusterHealthStatus::Online,
        }
    }

    fn shard(slots: Vec<(u16, u16)>, nodes: Vec<ClusterNodeResult>) -> ClusterShardResult {
        ClusterShardResult { slots, nodes }
    }

    /// A slot range names its master first, and that is who a command routed by
    /// slot is fed to. A reply listing a replica first would silently send every
    /// write of the shard to a node that answers `MOVED`.
    #[test]
    fn a_shard_is_normalised_with_its_master_first() {
        let normalised = with_master_first(shard(
            vec![(0, 100)],
            vec![
                node("r1", "replica"),
                node("m", "master"),
                node("r2", "replica"),
            ],
        ))
        .expect("a shard with a master is usable");

        let ids = normalised
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!("m", ids.first().copied().unwrap());
        assert_eq!(3, ids.len(), "no node may be lost by the reordering");
    }

    /// A shard already in order is left alone rather than rotated.
    #[test]
    fn a_shard_already_in_order_is_untouched() {
        let normalised = with_master_first(shard(
            vec![(0, 100)],
            vec![node("m", "master"), node("r1", "replica")],
        ))
        .unwrap();

        let ids = normalised
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(vec!["m", "r1"], ids);
    }

    /// A shard with no master, or no node at all, is refused rather than
    /// indexed: the slot map would name a master that is not one, and every
    /// command for those slots would go to a replica forever.
    #[test]
    fn a_shard_without_a_master_is_refused() {
        assert!(with_master_first(shard(vec![(0, 100)], vec![])).is_err());
        assert!(
            with_master_first(shard(
                vec![(0, 100)],
                vec![node("r1", "replica"), node("r2", "replica")]
            ))
            .is_err()
        );
    }

    /// Every slot range of a shard is served by the same node list, master
    /// first — that is what the replica round-robin reads.
    #[test]
    fn a_shard_owns_each_of_its_slot_ranges_with_the_same_nodes() {
        let shard = with_master_first(shard(
            vec![(0, 100), (500, 600)],
            vec![node("m", "master"), node("r1", "replica")],
        ))
        .unwrap();

        let ranges = slot_ranges_of(&shard).collect::<Vec<_>>();
        assert_eq!(2, ranges.len());
        for range in &ranges {
            let ids = range
                .node_ids
                .iter()
                .map(|id| id.as_ref())
                .collect::<Vec<_>>();
            assert_eq!(vec!["m", "r1"], ids);
            assert_eq!(0, range.next_replica);
        }
        assert_eq!(
            vec![(0, 100), (500, 600)],
            ranges.iter().map(|r| r.slot_range).collect::<Vec<_>>()
        );
    }

    /// The retention filter binary-searches this list, so it has to be sorted.
    /// Unsorted, a node the cluster still holds would be reported gone and its
    /// connection dropped on every refresh.
    #[test]
    fn the_known_ids_are_sorted_for_the_search_that_reads_them() {
        let ids = known_node_ids(&[
            shard(
                vec![(0, 100)],
                vec![node("m2", "master"), node("r9", "replica")],
            ),
            shard(vec![(101, 200)], vec![node("m1", "master")]),
        ]);

        assert_eq!(vec!["m1", "m2", "r9"], ids);
    }

    /// `CLUSTER SLOTS` lists one entry per slot range, so a shard owning several
    /// appears several times. They must fold into one shard, or the slot map
    /// would hold a separate shard per range and lose the replica list of all
    /// but the first.
    #[test]
    fn a_legacy_reply_folds_a_shard_s_ranges_into_one_shard() {
        let legacy_node = |id: &str, port: u16| LegacyClusterNodeResult {
            ip: "127.0.0.1".to_owned(),
            port,
            id: id.to_owned(),
            preferred_endpoint: "127.0.0.1".to_owned(),
            hostname: None,
        };

        let shards = convert_from_legacy_shard_description(vec![
            LegacyClusterShardResult {
                slot: (0, 100),
                nodes: vec![legacy_node("m1", 7000), legacy_node("r1", 7001)],
            },
            LegacyClusterShardResult {
                slot: (500, 600),
                nodes: vec![legacy_node("m1", 7000), legacy_node("r1", 7001)],
            },
            LegacyClusterShardResult {
                slot: (101, 200),
                nodes: vec![legacy_node("m2", 7002)],
            },
        ]);

        assert_eq!(2, shards.len());

        let m1 = shards
            .iter()
            .find(|s| s.nodes.first().map(|n| n.id.as_str()) == Some("m1"))
            .expect("the shard whose master is m1");
        assert_eq!(vec![(0, 100), (500, 600)], m1.slots);
        assert_eq!(2, m1.nodes.len());
        // The legacy reply says nothing about roles: the first node listed is
        // the master, everything after it a replica.
        assert_eq!("master", m1.nodes.first().unwrap().role);
        assert_eq!("replica", m1.nodes.get(1).unwrap().role);
    }
}
