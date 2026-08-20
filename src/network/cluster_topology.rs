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
    StandaloneConnection,
    resp::{Command, RespResponse},
};
use futures_util::{FutureExt, future};
use rand::RngExt;
use smallvec::SmallVec;
use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    sync::Arc,
    task::Poll,
};

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

    /// The addresses of the nodes held, which a rediscovery starts from: they
    /// are the ones known to have answered, ahead of the configured seeds.
    pub(super) fn addresses(&self) -> Vec<ClusterNodeAddress> {
        self.nodes.iter().map(|n| n.address.clone()).collect()
    }

    #[cfg(test)]
    pub(super) fn node_count(&self) -> usize {
        self.nodes.len()
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
    pub(super) fn insert_node(&mut self, node: Node) {
        let position = self
            .nodes
            .binary_search_by(|n| n.id.cmp(&node.id))
            .unwrap_or_else(|position| position);
        self.nodes.insert(position, node);
    }

    /// Adds a slot range where the slot search will find it.
    pub(super) fn insert_slot_range(&mut self, slot_range: SlotRange) {
        let position = self
            .slot_ranges
            .binary_search_by_key(&slot_range.slot_range.0, |s| s.slot_range.0)
            .unwrap_or_else(|position| position);
        self.slot_ranges.insert(position, slot_range);
    }

    /// Drops the nodes a refreshed topology no longer describes. Order-preserving,
    /// so the id search stays valid.
    pub(super) fn retain_nodes(&mut self, keep: impl FnMut(&Node) -> bool) {
        self.nodes.retain(keep);
    }

    pub(super) fn clear_slot_ranges(&mut self) {
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

    pub(super) fn node_by_id_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
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
    pub(super) fn slot_range_by_slot_mut(&mut self, slot: u16) -> Option<&mut SlotRange> {
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
