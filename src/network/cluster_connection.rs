use super::pub_sub_message::PubSubMessage;
use crate::{
    ClientError, Error, RedisError, RedisErrorKind, Result, RetryReason, StandaloneConnection,
    client::{ClusterConfig, Config},
    commands::{
        ClusterCommands, ClusterHealthStatus, ClusterNodeResult, ClusterShardResult,
        LegacyClusterShardResult, RequestPolicy, ResponsePolicy,
    },
    network::Version,
    resp::{Command, CommandBuilder, RespResponse, RespView},
};
use bytes::Bytes;
use futures_util::{FutureExt, future};
use rand::Rng;
use smallvec::{SmallVec, smallvec};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    fmt::{Debug, Formatter},
    iter::zip,
    sync::Arc,
    task::Poll,
};
use tracing::{debug, error, info, trace, warn};

/// Test-only handle used to make the cluster topology-change failure path
/// observable. Shared (via `Arc`) between a test and the `ClusterConnection`
/// living inside the network task; like `SendBatchTestHook`, it exists only
/// when the crate itself is built as a test target.
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct ClusterTestHook {
    /// When armed, the node serving the oldest in-flight request is removed
    /// from the topology and its slot ranges are handed over to a surviving
    /// node, reproducing the state a topology refresh leaves behind when a node
    /// disappears while requests are in flight against it.
    drop_front_pending_node: Arc<std::sync::atomic::AtomicBool>,
    /// When armed, the next topology refresh discovers an empty cluster,
    /// reproducing what a buggy server, a proxy, or a corrupted discovery reply
    /// can return.
    empty_topology_on_refresh: Arc<std::sync::atomic::AtomicBool>,
    /// When set, the initial discovery ignores the shard holding this node,
    /// reproducing a local topology that does not know a node the cluster does.
    hidden_node_id: Arc<std::sync::Mutex<Option<String>>>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test-support code: a panic is how a test reports failure"
)]
impl ClusterTestHook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Arms a one-shot removal of the node serving the oldest in-flight request.
    /// It is consumed only once such a request actually exists.
    pub fn arm_drop_front_pending_node(&self) {
        self.drop_front_pending_node
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn take_drop_front_pending_node(&self) -> bool {
        self.drop_front_pending_node
            .swap(false, std::sync::atomic::Ordering::SeqCst)
    }

    /// Arms a one-shot empty topology discovery on the next refresh.
    pub fn arm_empty_topology_on_refresh(&self) {
        self.empty_topology_on_refresh
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn take_empty_topology_on_refresh(&self) -> bool {
        self.empty_topology_on_refresh
            .swap(false, std::sync::atomic::Ordering::SeqCst)
    }

    /// Hides the shard holding `node_id` from the initial discovery only, so
    /// that a later refresh sees the real topology again.
    pub fn hide_node_on_initial_discovery(&self, node_id: &str) {
        *self.hidden_node_id.lock().unwrap() = Some(node_id.to_owned());
    }

    fn take_hidden_node_id(&self) -> Option<String> {
        self.hidden_node_id.lock().unwrap().take()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[repr(transparent)]
struct NodeId(Arc<str>);

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

struct Node {
    pub id: NodeId,
    pub is_master: bool,
    pub address: (String, u16),
    pub connection: StandaloneConnection,
    pub is_dirty: bool,
}

impl Node {
    pub async fn feed(&mut self, command: &Command) -> Result<()> {
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
struct SlotRange {
    pub slot_range: (u16, u16),
    /// node ids of the shard that owns the slot range,
    /// the first node id being the master node id
    pub node_ids: SmallVec<[NodeId; 6]>,
}

#[derive(Debug)]
struct SubRequest {
    pub node_id: NodeId,
    pub keys: SmallVec<[Bytes; 10]>,
    pub result: Option<Option<Result<RespResponse>>>,
}

#[derive(Debug)]
struct RequestInfo {
    pub response_policy: Option<ResponsePolicy>,
    pub keys: SmallVec<[Bytes; 10]>,
    pub sub_requests: SmallVec<[SubRequest; 10]>,
    /// The command the sub-requests were derived from, kept only when a single
    /// sub-request can be re-sent on its own — i.e. when the command was split
    /// across several shards. Everything else is retried as a whole and does not
    /// pay for the clone.
    pub command: Option<Command>,
    /// Whether the command is a subscription one, whose answer is a push frame
    /// the network handler consumes on its own. See `retire_pub_sub_request`.
    pub is_pub_sub: bool,
    #[allow(unused)]
    #[cfg(test)]
    pub command_seq: usize,
}

/// A subscription command is acknowledged by a push frame, not by an ordinary
/// reply: `read` hands it to the network handler, which matches it against the
/// caller itself. Nothing therefore ever fills the sub-request the connection
/// filed for it.
fn is_pub_sub_command(command: &Command) -> bool {
    matches!(
        command.name(),
        b"SUBSCRIBE"
            | b"PSUBSCRIBE"
            | b"SSUBSCRIBE"
            | b"UNSUBSCRIBE"
            | b"PUNSUBSCRIBE"
            | b"SUNSUBSCRIBE"
    )
}

/// A sub-request that must be re-sent to another node before its request can be
/// completed. Held aside because deciding this happens in `read`/`try_read`,
/// and `try_read` cannot await the send.
struct PendingRedirection {
    node_id: NodeId,
    command: Command,
    should_ask: bool,
}

/// What `internal_read` concluded about a fulfilled request.
enum ReadOutcome {
    /// The request is over: this is its answer, or `None` for a disconnection.
    Ready(Option<Result<RespResponse>>),
    /// Part of the request was redirected and has been re-armed against the
    /// right node. There is nothing to report yet.
    Deferred,
}

/// Stores the state related to the current transaction (MULTI/EXEC block).
#[derive(Debug, Default)]
struct TransactionState {
    /// Holds the MULTI command temporarily until we know which shard to send it to.
    pending_multi: Option<Command>,
    /// The index of the node currently locked for the transaction.
    node_index: Option<usize>,
}

impl ClusterNodeResult {
    pub(crate) fn get_port(&self) -> Result<u16> {
        match (self.port, self.tls_port) {
            (None, Some(port)) => Ok(port),
            (Some(port), None) => Ok(port),
            _ => Err(Error::Client(ClientError::ClusterConfig)),
        }
    }
}

/// Cluster connection
/// read & write_batch functions are implemented following Redis Command Tips
/// See <https://redis.io/docs/reference/command-tips/>
pub struct ClusterConnection {
    cluster_config: ClusterConfig,
    config: Config,
    nodes: Vec<Node>,
    slot_ranges: Vec<SlotRange>,
    pending_requests: VecDeque<RequestInfo>,
    /// Sub-requests re-armed by a partial redirection, awaiting the next `read`
    /// to be sent.
    pending_redirections: Vec<PendingRedirection>,
    tag: Arc<str>,
    /// State to manage the "Lazy MULTI" logic
    transaction_state: TransactionState,
    /// Whether the topology has already been refreshed during the send batch
    /// currently being fed. Reset by `flush`, which ends that batch.
    refreshed_in_current_batch: bool,
    #[cfg(test)]
    test_hook: Option<ClusterTestHook>,
}

impl ClusterConnection {
    pub async fn connect(
        cluster_config: &ClusterConfig,
        config: &Config,
    ) -> Result<ClusterConnection> {
        let (mut nodes, slot_ranges) = Self::connect_to_cluster(cluster_config, config).await?;
        let first_node = nodes
            .get_mut(0)
            .ok_or_else(|| Error::Client(ClientError::ClusterConfig))?;

        let tag = first_node.connection.tag();

        Ok(ClusterConnection {
            cluster_config: cluster_config.clone(),
            config: config.clone(),
            nodes,
            slot_ranges,
            pending_requests: VecDeque::new(),
            pending_redirections: Vec::new(),
            tag,
            transaction_state: TransactionState::default(),
            refreshed_in_current_batch: false,
            #[cfg(test)]
            test_hook: config.cluster_test_hook.clone(),
        })
    }

    #[inline]
    pub async fn feed(&mut self, command: &Command, retry_reasons: &[RetryReason]) -> Result<()> {
        if retry_reasons.iter().any(|r| {
            matches!(
                r,
                RetryReason::Moved {
                    hash_slot: _,
                    address: _
                }
            )
        }) {
            // The retry reasons are carried by the message, so every command of
            // a retried batch is fed with them. One refresh per send batch is
            // enough: it reloads the whole topology, which covers them all.
            if !self.refreshed_in_current_batch {
                self.refreshed_in_current_batch = true;
                self.refresh_nodes_and_slot_ranges().await?;
            }
        }

        let ask_reasons = retry_reasons
            .iter()
            .filter_map(|r| {
                if let RetryReason::Ask { hash_slot, address } = r {
                    Some((*hash_slot, address.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // An ASK points at the node importing the slot, which the local topology
        // may not know: it may have joined, or only been learned about, after
        // the last discovery. Unlike a MOVED, an ASK invalidates nothing, so
        // nothing else would ever bring that node in and the command would fail
        // outright, where the cluster spec requires the redirection to be
        // followed. Reload the topology so the target becomes reachable.
        if !self.refreshed_in_current_batch
            && ask_reasons.iter().any(|(_hash_slot, address)| {
                !self.nodes.iter().any(|node| node.address == *address)
            })
        {
            self.refreshed_in_current_batch = true;
            self.refresh_nodes_and_slot_ranges().await?;
        }

        if let Some(multi_cmd) = self.transaction_state.pending_multi.take() {
            let (node_idx, _) = self.get_no_request_policy_node(command, &ask_reasons)?;
            self.feed_no_request_policy(&multi_cmd, node_idx, false)
                .await?;
            self.transaction_state.node_index = Some(node_idx);
        }

        match command.name() {
            b"MULTI" => {
                // We do not send it to the network yet. We wait for the first key-based command
                // to decide which shard owns this transaction.
                self.transaction_state.pending_multi = Some(command.clone());
            }
            b"EXEC" => {
                if let Some(node_idx) = self.transaction_state.node_index {
                    self.feed_no_request_policy(command, node_idx, false)
                        .await?;
                    self.transaction_state = TransactionState::default();
                } else {
                    return Err(Error::Client(ClientError::ExecCalledWithoutMulti));
                }
            }
            _ => self.internal_feed(command, &ask_reasons).await?,
        }

        Ok(())
    }

    async fn internal_feed(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        trace!("Analyzing command {command:?}");
        let request_policy = command.request_policy();

        if let Some(request_policy) = request_policy {
            match request_policy {
                RequestPolicy::AllNodes => {
                    self.request_policy_all_nodes(command).await?;
                }
                RequestPolicy::AllShards => {
                    self.request_policy_all_shards(command).await?;
                }
                RequestPolicy::MultiShard => {
                    self.request_policy_multi_shard(command, ask_reasons)
                        .await?;
                }
                RequestPolicy::Special => {
                    self.request_policy_special(command)?;
                }
            }
        } else {
            self.no_request_policy(command, ask_reasons).await?;
        }

        Ok(())
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        // End of the send batch: allow the next one to refresh again if needed.
        self.refreshed_in_current_batch = false;

        let mut flush_futures = SmallVec::<[_; 16]>::new();

        for node in self.nodes.iter_mut() {
            if node.is_dirty {
                node.is_dirty = false;
                flush_futures.push(node.connection.flush());
            }
        }

        let results = future::join_all(flush_futures).await;

        for res in results {
            res?;
        }

        Ok(())
    }

    /// The client should execute the command on all master shards (e.g., the DBSIZE command).
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_shards(&mut self, command: &Command) -> Result<()> {
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for node in self.nodes.iter_mut().filter(|n| n.is_master) {
            node.feed(command).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests,
            keys: command.keys().collect(),
            command: None,
            is_pub_sub: is_pub_sub_command(command),
            #[cfg(test)]
            command_seq: command.command_seq,
        };

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    /// The client should execute the command on all nodes - masters and replicas alike.
    /// An example is the CONFIG SET command.
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_nodes(&mut self, command: &Command) -> Result<()> {
        if self.nodes.iter().all(|n| n.is_master) {
            self.connect_replicas().await?;
        }
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for node in self.nodes.iter_mut() {
            node.feed(command).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests,
            keys: command.keys().collect(),
            command: None,
            is_pub_sub: is_pub_sub_command(command),
            #[cfg(test)]
            command_seq: command.command_seq,
        };

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    /// The client should execute the command on multiple shards.
    /// The shards that execute the command are determined by the hash slots of its input key name arguments.
    /// Examples for such commands include MSET, MGET and DEL.
    /// However, note that SUNIONSTORE isn't considered as multi_shard because all of its keys must belong to the same hash slot.
    async fn request_policy_multi_shard(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        let mut node_slot_keys_ask = command
            .args_for_cluster()
            .filter_map(|(arg, is_key, slot)| {
                is_key.then(|| {
                    let (node_index, should_ask) = self
                        .get_master_node_index_by_slot(slot, ask_reasons)
                        .ok_or_else(|| Error::Client(ClientError::ClusterConfig))?;
                    Ok((node_index, slot, arg, should_ask))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        if node_slot_keys_ask.is_empty() {
            return Ok(());
        }

        node_slot_keys_ask.sort();
        trace!("node_slot_keys_ask: {node_slot_keys_ask:?}");

        let mut current_slot_keys = SmallVec::<[Bytes; 10]>::new();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();
        let mut last_slot = u16::MAX;
        let mut last_node_index: usize = usize::MAX;
        let mut last_should_ask = false;

        // Placeholder, overwritten on the first iteration: `last_node_index`
        // starts at a value no real index can equal. A node-less connection
        // cannot serve the non-empty work list above.
        let mut node = self
            .nodes
            .first_mut()
            .ok_or_else(|| Error::Client(ClientError::InconsistentRoutingState))?;

        for (node_index, slot, key, should_ask) in node_slot_keys_ask {
            if slot != last_slot {
                if !current_slot_keys.is_empty() {
                    if last_should_ask {
                        node.connection.asking().await?;
                    }

                    let shard_command = prepare_command_for_shard(command, &current_slot_keys);
                    node.feed(&shard_command).await?;
                    sub_requests.push(SubRequest {
                        node_id: node.id.clone(),
                        keys: std::mem::take(&mut current_slot_keys),
                        result: None,
                    });
                }

                last_slot = slot;
                last_should_ask = should_ask;
            }

            current_slot_keys.push(key);

            if node_index != last_node_index {
                node = self
                    .nodes
                    .get_mut(node_index)
                    .ok_or_else(|| Error::Client(ClientError::InconsistentRoutingState))?;
                last_node_index = node_index;
            }
        }

        if last_should_ask {
            node.connection.asking().await?;
        }

        let shard_command = prepare_command_for_shard(command, &current_slot_keys);
        node.feed(&shard_command).await?;
        sub_requests.push(SubRequest {
            node_id: node.id.clone(),
            keys: std::mem::take(&mut current_slot_keys),
            result: None,
        });

        let sub_requests_len = sub_requests.len();
        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            keys: command.keys().collect(),
            sub_requests,
            command: (sub_requests_len > 1).then(|| command.clone()),
            is_pub_sub: is_pub_sub_command(command),
            #[cfg(test)]
            command_seq: command.command_seq,
        };

        trace!("{request_info:?}");

        self.pending_requests.push_back(request_info);

        Ok(())
    }

    async fn no_request_policy(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<usize> {
        let (node_idx, should_ask) = self.get_no_request_policy_node(command, ask_reasons)?;
        self.feed_no_request_policy(command, node_idx, should_ask)
            .await?;
        Ok(node_idx)
    }

    fn get_no_request_policy_node(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<(usize, bool)> {
        let mut slots = command.slots();

        if let Some(first_slot) = slots.next() {
            if !slots.all(|s| s == first_slot) {
                return Err(Error::Client(ClientError::MismatchedKeySlots));
            }

            self.get_master_node_index_by_slot(first_slot, ask_reasons)
                .ok_or_else(|| Error::Client(ClientError::ClusterConfig))
        } else {
            self.get_random_node_index()
                .map(|node_idx| (node_idx, false))
                .ok_or_else(|| Error::Client(ClientError::ClusterConfig))
        }
    }

    async fn feed_no_request_policy(
        &mut self,
        command: &Command,
        node_idx: usize,
        should_ask: bool,
    ) -> Result<()> {
        let node = self
            .nodes
            .get_mut(node_idx)
            .ok_or_else(|| Error::Client(ClientError::InconsistentRoutingState))?;
        if should_ask {
            node.connection.asking().await?;
        }
        node.feed(command).await?;
        let keys: SmallVec<[Bytes; 10]> = command.keys().collect();
        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests: smallvec![SubRequest {
                node_id: node.id.clone(),
                keys: keys.clone(),
                result: None,
            }],
            keys,
            command: None,
            is_pub_sub: is_pub_sub_command(command),
            #[cfg(test)]
            command_seq: command.command_seq,
        };
        self.pending_requests.push_back(request_info);
        Ok(())
    }

    fn request_policy_special(&mut self, _command: &Command) -> Result<()> {
        Err(Error::Client(ClientError::CommandNotSupportedInCluster))
    }

    /// A pending request is orphaned once one of its still-unresolved
    /// sub-requests targets a node that is no longer part of the cluster: a
    /// topology refresh removed that node, and its connection died with it, so
    /// the response can never arrive. Since `read()` pops the front request only
    /// once **all** its sub-requests resolve, an orphaned request left at the
    /// front would block every subsequent reply and hang all callers.
    /// Test-only: reproduce the state a topology refresh leaves behind when the
    /// node serving the oldest in-flight request disappears from the cluster.
    /// Consumed only once such a request exists, so a test needs no timing
    /// assumption about when its command reaches the wire.
    #[cfg(test)]
    fn apply_test_node_drop(&mut self) {
        let Some(hook) = self.test_hook.clone() else {
            return;
        };

        let Some(victim) = self
            .pending_requests
            .front()
            .and_then(|ri| ri.sub_requests.iter().find(|sr| sr.result.is_none()))
            .map(|sr| sr.node_id.clone())
        else {
            return;
        };

        // Keep at least one node so the cluster stays usable.
        if self.nodes.len() < 2 || !hook.take_drop_front_pending_node() {
            return;
        }

        self.nodes.retain(|node| node.id != victim);
        debug!("test hook removed node {victim:?}");
    }

    /// Drops the pending request a subscription command left behind, now that
    /// the server has acknowledged it with a push frame. Without this the
    /// request waits for a reply that never comes, and since `read` reports the
    /// queue in order, it blocks every later reply from any other node — the
    /// whole connection deadlocks. Only a subscription acknowledgement retires
    /// one: an error reply such as `MOVED` is filed as a result like any other,
    /// so the redirection path keeps working.
    fn retire_pub_sub_request(&mut self, node_id: &NodeId, response: &RespResponse) {
        if !matches!(
            PubSubMessage::try_from(response),
            Ok(PubSubMessage::Subscribe(_)
                | PubSubMessage::PSubscribe(_)
                | PubSubMessage::SSubscribe(_)
                | PubSubMessage::Unsubscribe(_)
                | PubSubMessage::PUnsubscribe(_)
                | PubSubMessage::SUnsubscribe(_))
        ) {
            return;
        }

        let Some(index) = self.pending_requests.iter().position(|request| {
            request.is_pub_sub
                && request.sub_requests.iter().any(|sub_request| {
                    sub_request.node_id == *node_id && sub_request.result.is_none()
                })
        }) else {
            return;
        };

        self.pending_requests.remove(index);
    }

    fn front_request_references_missing_node(&self) -> bool {
        let Some(request_info) = self.pending_requests.front() else {
            return false;
        };

        request_info
            .sub_requests
            .iter()
            .any(|sr| sr.result.is_none() && self.get_node_index_by_id(&sr.node_id).is_none())
    }

    pub async fn read(&mut self) -> Option<Result<RespResponse>> {
        loop {
            #[cfg(test)]
            self.apply_test_node_drop();

            // Sub-requests re-armed by a partial redirection, possibly by a
            // `try_read` that could not await their send.
            if !self.pending_redirections.is_empty()
                && let Err(e) = self.flush_pending_redirections().await
            {
                return Some(Err(e));
            }

            // Fail an orphaned front request instead of waiting forever for a
            // reply that will never come. It is reported as a lost connection,
            // not as a redirection: replaying it unconditionally would
            // re-execute a command whose caller may have opted out of retries,
            // and which the vanished node may well have already run.
            if self.front_request_references_missing_node() {
                self.pending_requests.pop_front();
                return Some(Err(Error::DisconnectedByPeer));
            }

            if let Some(ri) = self.pending_requests.front()
                && ri.sub_requests.iter().all(|sr| sr.result.is_some())
            {
                trace!("fulfilled request_info: {ri:?}");
                if let Some(ri) = self.pending_requests.pop_front() {
                    match self.internal_read(ri) {
                        ReadOutcome::Ready(result) => return result,
                        ReadOutcome::Deferred => continue,
                    }
                }
            }

            // `select_all` panics on an empty set of futures. A node-less
            // cluster connection cannot serve anything: report it as a
            // disconnection so the handler reconnects and rediscovers the
            // topology, rather than taking the whole network task down.
            if self.nodes.is_empty() {
                warn!("No cluster node available to read from");
                return None;
            }

            let read_futures = self.nodes.iter_mut().map(|n| n.connection.read().boxed());
            let (result, node_idx, _) = future::select_all(read_futures).await;

            result.as_ref()?;

            if let Some(Ok(response)) = &result
                && response.is_push()
            {
                if let Some(node_id) = self.nodes.get(node_idx).map(|node| node.id.clone()) {
                    self.retire_pub_sub_request(&node_id, response);
                }
                return result;
            }

            // `select_all` reports the index of the future it resolved, so this
            // always addresses a node we are holding.
            let Some(node) = self.nodes.get(node_idx) else {
                return Some(Err(Error::Client(ClientError::InconsistentRoutingState)));
            };
            let node_id = &node.id;

            let Some((req_idx, sub_req_idx)) =
                self.pending_requests
                    .iter()
                    .enumerate()
                    .find_map(|(req_idx, req)| {
                        let sub_req_idx = req
                            .sub_requests
                            .iter()
                            .position(|sr| sr.node_id == *node_id && sr.result.is_none())?;
                        Some((req_idx, sub_req_idx))
                    })
            else {
                error!(
                    "Received unexpected message: {result:?} from {}",
                    node.connection.tag()
                );
                return Some(Err(Error::Client(ClientError::UnexpectedMessageReceived)));
            };

            if !self.store_sub_request_result(req_idx, sub_req_idx, result) {
                return Some(Err(Error::Client(ClientError::InconsistentRoutingState)));
            }
        }
    }

    pub fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        loop {
            #[cfg(test)]
            self.apply_test_node_drop();

            // Re-armed sub-requests can only be sent from `read`, which can
            // await. Yield so the network loop goes back to it.
            if !self.pending_redirections.is_empty() {
                return Poll::Pending;
            }

            // See `read()`: an orphaned front request must not block the queue.
            if self.front_request_references_missing_node() {
                self.pending_requests.pop_front();
                return Poll::Ready(Some(Err(Error::DisconnectedByPeer)));
            }

            if let Some(ri) = self.pending_requests.front()
                && ri.sub_requests.iter().all(|sr| sr.result.is_some())
            {
                trace!("fulfilled request_info: {ri:?}");
                if let Some(ri) = self.pending_requests.pop_front() {
                    match self.internal_read(ri) {
                        ReadOutcome::Ready(result) => return Poll::Ready(result),
                        ReadOutcome::Deferred => return Poll::Pending,
                    }
                }
            }

            // See `read()`: a node-less connection cannot serve anything.
            if self.nodes.is_empty() {
                warn!("No cluster node available to read from");
                return Poll::Ready(None);
            }

            let Some((node_idx, result)) =
                self.nodes.iter_mut().enumerate().find_map(|(node_idx, n)| {
                    match n.connection.try_read() {
                        Poll::Ready(result) => Some((node_idx, result)),
                        Poll::Pending => None,
                    }
                })
            else {
                return Poll::Pending;
            };

            if let Some(Ok(response)) = &result
                && response.is_push()
            {
                if let Some(node_id) = self.nodes.get(node_idx).map(|node| node.id.clone()) {
                    self.retire_pub_sub_request(&node_id, response);
                }
                return Poll::Ready(result);
            }

            // The index comes from the `enumerate` over `self.nodes` just above.
            let Some(node) = self.nodes.get(node_idx) else {
                return Poll::Ready(Some(Err(Error::Client(
                    ClientError::InconsistentRoutingState,
                ))));
            };
            let node_id = &node.id;

            let Some((req_idx, sub_req_idx)) =
                self.pending_requests
                    .iter()
                    .enumerate()
                    .find_map(|(req_idx, req)| {
                        let sub_req_idx = req
                            .sub_requests
                            .iter()
                            .position(|sr| sr.node_id == *node_id && sr.result.is_none())?;
                        Some((req_idx, sub_req_idx))
                    })
            else {
                error!(
                    node = %node.connection.tag(),
                    "Received unexpected message: {result:?}"
                );
                return Poll::Ready(Some(Err(Error::Client(
                    ClientError::UnexpectedMessageReceived,
                ))));
            };

            if !self.store_sub_request_result(req_idx, sub_req_idx, result) {
                return Poll::Ready(Some(Err(Error::Client(
                    ClientError::InconsistentRoutingState,
                ))));
            }
        }
    }

    /// Files a sub-request result at the indices the caller just located by
    /// scanning `pending_requests`, returning `false` if either index no longer
    /// addresses anything.
    ///
    /// The scan and the store see the same queue with no mutation in between, so
    /// `false` is unreachable; the caller turns it into an error for that one
    /// command rather than letting it panic the network task.
    fn store_sub_request_result(
        &mut self,
        req_idx: usize,
        sub_req_idx: usize,
        result: Option<Result<RespResponse>>,
    ) -> bool {
        let Some(request) = self.pending_requests.get_mut(req_idx) else {
            return false;
        };
        let Some(sub_request) = request.sub_requests.get_mut(sub_req_idx) else {
            return false;
        };
        sub_request.result = Some(result);
        trace!(
            "Did store sub-request result into {:?}",
            self.pending_requests.get(req_idx)
        );
        true
    }

    /// Collects the ASK/MOVED redirections carried by a fulfilled request,
    /// paired with the sub-request that received them.
    fn collect_redirections(request_info: &RequestInfo) -> SmallVec<[(usize, RetryReason); 1]> {
        let mut redirections = SmallVec::<[(usize, RetryReason); 1]>::new();

        for (idx, sub_request) in request_info.sub_requests.iter().enumerate() {
            let Some(Some(Ok(result))) = &sub_request.result else {
                continue;
            };

            let Ok(RespView::Error(error)) = result.view() else {
                continue;
            };

            match RedisError::try_from(error) {
                Ok(RedisError {
                    kind: RedisErrorKind::Ask { hash_slot, address },
                    ..
                }) => redirections.push((idx, RetryReason::Ask { hash_slot, address })),
                Ok(RedisError {
                    kind: RedisErrorKind::Moved { hash_slot, address },
                    ..
                }) => redirections.push((idx, RetryReason::Moved { hash_slot, address })),
                _ => (),
            }
        }

        redirections
    }

    /// Re-arms the redirected sub-requests of a partially redirected command
    /// against the nodes the server pointed at, leaving the sub-results already
    /// obtained untouched.
    ///
    /// Returns `false` — changing nothing — when a target is not a node we hold a
    /// connection to. The caller then falls back to retrying the whole command,
    /// which goes through a topology refresh.
    fn rearm_redirected_sub_requests(
        &mut self,
        request_info: &mut RequestInfo,
        redirections: &[(usize, RetryReason)],
    ) -> bool {
        let Some(command) = request_info.command.clone() else {
            return false;
        };

        // Resolve every target first: re-arming half of the sub-requests and
        // then giving up would leave the request unable to ever complete.
        let mut targets = SmallVec::<[(usize, NodeId, bool); 1]>::new();

        for (idx, reason) in redirections {
            let (address, should_ask) = match reason {
                RetryReason::Ask { address, .. } => (address, true),
                RetryReason::Moved { address, .. } => (address, false),
            };

            let Some(node) = self.nodes.iter().find(|n| n.address == *address) else {
                return false;
            };

            // Resolve the sub-request index here too, for the same reason: the
            // loop below must not be able to skip one half-way through.
            if request_info.sub_requests.get(*idx).is_none() {
                return false;
            }

            targets.push((*idx, node.id.clone(), should_ask));
        }

        for (idx, node_id, should_ask) in targets {
            // Bounds-checked in the resolve loop above.
            let Some(sub_request) = request_info.sub_requests.get_mut(idx) else {
                continue;
            };
            let shard_command = prepare_command_for_shard(&command, &sub_request.keys);

            sub_request.node_id = node_id.clone();
            sub_request.result = None;

            self.pending_redirections.push(PendingRedirection {
                node_id,
                command: shard_command,
                should_ask,
            });
        }

        true
    }

    /// Sends the sub-requests re-armed by a partial redirection.
    async fn flush_pending_redirections(&mut self) -> Result<()> {
        let redirections = std::mem::take(&mut self.pending_redirections);

        // A MOVED means the slot map is stale, exactly as on the whole-command
        // retry path. Without this every later command on that slot would be
        // redirected again. The re-send itself does not depend on it — the
        // target is already known by node id — so a failed refresh only costs
        // freshness and must not fail the request.
        if redirections.iter().any(|r| !r.should_ask)
            && let Err(e) = self.refresh_nodes_and_slot_ranges().await
        {
            warn!("Cannot refresh the topology after a redirection: {e}");
        }

        for redirection in redirections {
            // A node that vanished in the meantime leaves the sub-request
            // unfulfilled; the orphan check at the top of `read` turns that into
            // a reported failure rather than an endless wait.
            let Some(node_index) = self.get_node_index_by_id(&redirection.node_id) else {
                warn!("Redirection target {:?} is gone", redirection.node_id);
                continue;
            };

            let node = self
                .nodes
                .get_mut(node_index)
                .ok_or_else(|| Error::Client(ClientError::InconsistentRoutingState))?;
            if redirection.should_ask {
                node.connection.asking().await?;
            }
            node.feed(&redirection.command).await?;
        }

        self.flush().await
    }

    fn internal_read(&mut self, mut request_info: RequestInfo) -> ReadOutcome {
        // A command split across shards whose sub-requests did not all fail must
        // not be replayed as a whole: the shards that answered already applied
        // it, and a second run reports different numbers — a replayed `DEL`
        // answers 0 for the keys it deleted the first time. Re-send only what
        // was redirected and keep the rest.
        let redirections = Self::collect_redirections(&request_info);
        if !redirections.is_empty()
            && redirections.len() < request_info.sub_requests.len()
            && self.rearm_redirected_sub_requests(&mut request_info, &redirections)
        {
            debug!(
                "partially redirected request, re-sending {} of {} sub-requests. reasons: {:?}",
                redirections.len(),
                request_info.sub_requests.len(),
                redirections.iter().map(|(_, r)| r).collect::<Vec<_>>()
            );
            self.pending_requests.push_front(request_info);
            return ReadOutcome::Deferred;
        }

        ReadOutcome::Ready(self.aggregate_sub_results(request_info))
    }

    fn aggregate_sub_results(
        &mut self,
        mut request_info: RequestInfo,
    ) -> Option<std::result::Result<RespResponse, Error>> {
        let mut sub_results =
            Vec::<Result<RespResponse>>::with_capacity(request_info.sub_requests.len());
        let mut retry_reasons = SmallVec::<[RetryReason; 1]>::new();

        for sub_request in request_info.sub_requests.iter_mut() {
            // A sub-request still waiting for its result, or one whose node stream
            // ended, leaves nothing to aggregate.
            let result = sub_request.result.take()??;

            if let Ok(result) = result {
                match result.view() {
                    Ok(RespView::Error(error)) => match RedisError::try_from(error) {
                        Ok(RedisError {
                            kind: RedisErrorKind::Ask { hash_slot, address },
                            description: _,
                        }) => retry_reasons.push(RetryReason::Ask {
                            hash_slot,
                            address: address.clone(),
                        }),
                        Ok(RedisError {
                            kind: RedisErrorKind::Moved { hash_slot, address },
                            description: _,
                        }) => retry_reasons.push(RetryReason::Moved {
                            hash_slot,
                            address: address.clone(),
                        }),
                        _ => sub_results.push(Ok(result)),
                    },
                    _ => sub_results.push(Ok(result)),
                }
            } else {
                sub_results.push(result);
            }
        }

        if !retry_reasons.is_empty() {
            debug!(
                "read failed and will be retried. reasons: {:?}",
                retry_reasons
            );
            return Some(Err(Error::Retry(retry_reasons)));
        }

        // The response_policy tip is set for commands that reply with scalar data types,
        // or when it's expected that clients implement a non-default aggregate.
        if let Some(response_policy) = &request_info.response_policy {
            match response_policy {
                ResponsePolicy::OneSucceeded => self.response_policy_one_succeeded(sub_results),
                ResponsePolicy::AllSucceeded => self.response_policy_all_succeeded(sub_results),
                ResponsePolicy::AggLogicalAnd => {
                    self.response_policy_agg(sub_results, |a, b| i64::from(a == 1 && b == 1))
                }
                ResponsePolicy::AggLogicalOr => self
                    .response_policy_agg(sub_results, |a, b| if a == 0 && b == 0 { 0 } else { 1 }),
                ResponsePolicy::AggMin => self.response_policy_agg(sub_results, i64::min),
                ResponsePolicy::AggMax => self.response_policy_agg(sub_results, i64::max),
                ResponsePolicy::AggSum => self.response_policy_agg(sub_results, |a, b| a + b),
                ResponsePolicy::Special => self.response_policy_special(sub_results),
            }
        } else {
            self.no_response_policy(sub_results, &request_info)
        }
    }

    fn response_policy_one_succeeded(
        &mut self,
        sub_results: Vec<Result<RespResponse>>,
    ) -> Option<Result<RespResponse>> {
        let mut result: Result<RespResponse> = Ok(RespResponse::null());

        for sub_result in sub_results {
            match &sub_result {
                Err(_) => result = sub_result,
                Ok(resp_buf) if resp_buf.is_error() => result = sub_result,
                _ => return Some(sub_result),
            }
        }

        Some(result)
    }

    fn response_policy_all_succeeded(
        &mut self,
        sub_results: Vec<Result<RespResponse>>,
    ) -> Option<Result<RespResponse>> {
        let mut result: Result<RespResponse> = Ok(RespResponse::null());

        for sub_result in sub_results {
            match &sub_result {
                Err(_) => return Some(sub_result),
                Ok(resp_buf) if resp_buf.is_error() => return Some(sub_result),
                _ => result = sub_result,
            }
        }

        Some(result)
    }

    fn response_policy_agg<F>(
        &mut self,
        sub_results: Vec<Result<RespResponse>>,
        f: F,
    ) -> Option<Result<RespResponse>>
    where
        F: Fn(i64, i64) -> i64,
    {
        let mut integer = Integer::Null;

        for sub_result in sub_results {
            let Ok(sub_result) = sub_result else {
                return Some(sub_result);
            };

            let view = match sub_result.view() {
                Ok(view) => view,
                Err(e) => return Some(Err(e)),
            };
            match view {
                RespView::Integer(i, _) => match &mut integer {
                    Integer::Single(current) => *current = f(*current, i),
                    Integer::Null => integer = Integer::Single(i),
                    Integer::Array(_) => return Some(Err(Error::Client(ClientError::Unexpected))),
                },
                RespView::Array(resp_array)
                | RespView::Set(resp_array)
                | RespView::Push(resp_array) => match &mut integer {
                    Integer::Single(_) => {
                        return Some(Err(Error::Client(ClientError::Unexpected)));
                    }
                    Integer::Array(items) => {
                        // Unequal per-shard array lengths must not be silently
                        // truncated by `zip`: an uncombined tail would be a wrong
                        // aggregate reported as success.
                        if items.len() != resp_array.len() {
                            return Some(Err(Error::Client(ClientError::Unexpected)));
                        }
                        for (item, view) in items.iter_mut().zip(resp_array) {
                            match view {
                                Ok(RespView::Integer(i, _)) => *item = f(*item, i),
                                Ok(_) => {
                                    return Some(Err(Error::Client(ClientError::Unexpected)));
                                }
                                Err(e) => return Some(Err(e)),
                            }
                        }
                    }
                    Integer::Null => {
                        let mut int_array = Vec::with_capacity(resp_array.len());

                        for view in resp_array {
                            match view {
                                Ok(RespView::Integer(i, _)) => int_array.push(i),
                                Ok(_) => {
                                    return Some(Err(Error::Client(ClientError::Unexpected)));
                                }
                                Err(e) => return Some(Err(e)),
                            }
                        }

                        integer = Integer::Array(int_array)
                    }
                },
                _ => return Some(Err(Error::Client(ClientError::Unexpected))),
            }
        }

        match integer {
            Integer::Single(i) => Some(Ok(RespResponse::integer(i))),
            Integer::Array(v) => Some(Ok(RespResponse::integer_array(v))),
            Integer::Null => Some(Ok(RespResponse::null())),
        }
    }

    fn response_policy_special(
        &mut self,
        _sub_results: Vec<Result<RespResponse>>,
    ) -> Option<Result<RespResponse>> {
        Some(Err(Error::Client(
            ClientError::CommandNotSupportedInCluster,
        )))
    }

    fn no_response_policy(
        &mut self,
        sub_results: Vec<Result<RespResponse>>,
        request_info: &RequestInfo,
    ) -> Option<Result<RespResponse>> {
        trace!("no_response_policy");

        if sub_results.len() == 1 {
            // when there is a single sub request, we just read the response
            // on the right connection. For example, GET's reply
            sub_results.into_iter().next()
        } else if request_info.keys.is_empty() {
            // The command doesn't accept key name arguments:
            // the client can aggregate all replies within a single nested data structure.
            // For example, the array replies we get from calling KEYS against all shards.
            // These should be packed in a single array in no particular order.
            let mut results = Vec::<RespResponse>::new();
            for sub_result in sub_results {
                // Propagate the shard's failure as a failure. Returning `None` here
                // would mean "disconnected" to the network handler, which would
                // reconnect the whole cluster over what is merely one shard
                // answering an error.
                let iter = match sub_result.and_then(RespResponse::into_collection_iter) {
                    Ok(iter) => iter,
                    Err(e) => return Some(Err(e)),
                };
                for item in iter {
                    match item {
                        Ok(item) => results.push(item),
                        Err(e) => return Some(Err(e)),
                    }
                }
            }

            Some(Ok(RespResponse::owned_array(results)))
        } else {
            // For commands that accept one or more key name arguments:
            // the client needs to retain the same order of replies as the input key names.
            // For example, MGET's aggregated reply.
            let mut results = SmallVec::<[(&Bytes, RespResponse); 10]>::new();

            for (sub_result, sub_request) in zip(sub_results, &request_info.sub_requests) {
                // Same reasoning as above: one shard's error is an error for the
                // caller, not a lost connection.
                let iter = match sub_result.and_then(RespResponse::into_collection_iter) {
                    Ok(iter) => iter,
                    Err(e) => return Some(Err(e)),
                };
                for (key, item) in sub_request.keys.iter().zip(iter) {
                    match item {
                        Ok(item) => results.push((key, item)),
                        Err(e) => return Some(Err(e)),
                    }
                }
            }

            // Precompute each key's position in the request's key list once, so
            // the reorder is O(n log n) instead of O(n² log n): the previous
            // comparator ran two linear `position` scans per comparison, making a
            // 10k-key MGET ~10⁹ `Bytes` comparisons. Duplicate keys keep their
            // first position, matching the old `position` semantics.
            let mut key_order = HashMap::<&Bytes, usize>::with_capacity(request_info.keys.len());
            for (i, k) in request_info.keys.iter().enumerate() {
                key_order.entry(k).or_insert(i);
            }
            results.sort_by_key(|(k, _)| *key_order.get(k).unwrap_or(&usize::MAX));

            let results = results.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
            Some(Ok(RespResponse::owned_array(results)))
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        info!("Reconnecting to cluster...");
        let (nodes, slot_ranges) =
            Self::connect_to_cluster(&self.cluster_config, &self.config).await?;
        info!("Reconnected to cluster!");

        self.nodes = nodes;
        self.slot_ranges = slot_ranges;

        // Every in-flight request was fed to the previous per-node connections,
        // which are now gone; their responses can never arrive. Left in place,
        // the request stuck at the front of the queue would block every
        // subsequent reply from surfacing (`read()` pops the front only once
        // all its sub-requests resolve) and hang all callers. Drop them here:
        // the network handler owns caller delivery and has already failed the
        // non-retryable messages and re-queued the retryable ones for replay,
        // which will repopulate `pending_requests` consistently.
        self.pending_requests.clear();

        Ok(())

        // TODO improve reconnection strategy with multiple retries
    }

    /// Discover the cluster topology over a **dedicated, short-lived**
    /// connection, trying each address in turn.
    ///
    /// Discovery must never run on one of the multiplexed node connections.
    /// Those are driven by the network handler in feed/flush/read batches, so
    /// they can hold commands that have been fed but not yet flushed — and
    /// callers of this function run *inside* such a batch (`feed` triggers a
    /// refresh on a MOVED). An inline request/response on such a connection
    /// flushes the pending command too, then reads a single frame and
    /// attributes it to the discovery command, corrupting both.
    async fn discover_shards(
        addresses: &[(String, u16)],
        config: &Config,
    ) -> Option<Vec<ClusterShardResult>> {
        debug!("Discovering cluster shards and slots...");

        for (host, port) in addresses {
            let mut connection = match StandaloneConnection::connect(host, *port, config).await {
                Ok(connection) => connection,
                Err(e) => {
                    warn!("Cannot connect to node ({host}:{port}): {e}");
                    continue;
                }
            };

            let version: Result<Version> = connection.get_version().try_into();
            let Ok(version) = version else {
                warn!(node = %connection.tag(), "Cannot get Redis version");
                continue;
            };

            // From Redis 7.x CLUSTER SLOTS is deprecated in favor of CLUSTER SHARDS
            let shard_info_list = if version.major < 7 {
                connection
                    .cluster_slots()
                    .await
                    .map(Self::convert_from_legacy_shard_description)
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

    /// Addresses to try for topology discovery: the nodes currently known,
    /// then the configured seeds as a fallback.
    fn discovery_addresses(&self) -> Vec<(String, u16)> {
        let mut addresses: Vec<(String, u16)> =
            self.nodes.iter().map(|node| node.address.clone()).collect();
        addresses.extend(self.cluster_config.nodes.iter().cloned());
        addresses
    }

    async fn connect_to_cluster(
        cluster_config: &ClusterConfig,
        config: &Config,
    ) -> Result<(Vec<Node>, Vec<SlotRange>)> {
        #[cfg_attr(not(test), allow(unused_mut))]
        let Some(mut shard_info_list) = Self::discover_shards(&cluster_config.nodes, config).await
        else {
            return Err(Error::Client(ClientError::ClusterConfig));
        };

        // Test-only: build a topology that ignores a node the cluster does know.
        #[cfg(test)]
        if let Some(hook) = &config.cluster_test_hook
            && let Some(hidden_node_id) = hook.take_hidden_node_id()
        {
            shard_info_list.retain(|s| !s.nodes.iter().any(|n| n.id == hidden_node_id));
        }

        let mut nodes = Vec::<Node>::new();
        let mut slot_ranges = Vec::<SlotRange>::new();

        for shard_info in shard_info_list.into_iter() {
            let Some(master_info) = shard_info
                .nodes
                .into_iter()
                .find(|n| n.role == "master" && n.health == ClusterHealthStatus::Online)
            else {
                return Err(Error::Client(ClientError::ClusterConfig));
            };
            let master_id: NodeId = master_info.id.as_str().into();

            let port = master_info.get_port()?;

            let connection = StandaloneConnection::connect(&master_info.ip, port, config).await?;

            slot_ranges.extend(shard_info.slots.iter().map(|s| SlotRange {
                slot_range: *s,
                node_ids: smallvec![master_id.clone()],
            }));

            nodes.push(Node {
                id: master_id.clone(),
                is_master: true,
                address: (master_info.ip, port),
                connection,
                is_dirty: false,
            });
        }

        slot_ranges.sort_by_key(|s| s.slot_range.0);
        nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!("Cluster connected: nodes={nodes:?}, slot_ranges={slot_ranges:?}");

        Ok((nodes, slot_ranges))
    }

    async fn connect_replicas(&mut self) -> Result<()> {
        debug!("Connecting replicas...");

        let addresses = self.discovery_addresses();
        let Some(shard_info_list) = Self::discover_shards(&addresses, &self.config).await else {
            return Err(Error::Client(ClientError::ClusterConfig));
        };

        for shard_info in shard_info_list {
            for node_info in shard_info.nodes.into_iter().filter(|n| n.role == "replica") {
                let port = node_info.get_port()?;
                let node_id: NodeId = node_info.id.as_str().into();

                let connection =
                    StandaloneConnection::connect(&node_info.ip, port, &self.config).await?;

                for slot_range_info in &shard_info.slots {
                    if let Some(slot_range) = self.get_slot_range_by_slot_mut(slot_range_info.0)
                        && slot_range.slot_range.1 == slot_range_info.1
                    {
                        slot_range.node_ids.push(node_id.clone())
                    }
                }

                self.nodes.push(Node {
                    id: node_id,
                    is_master: false,
                    address: (node_info.ip.clone(), port),
                    connection,
                    is_dirty: false,
                });
            }
        }

        self.nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!(
            "Cluster replicas connected: nodes={:?}, slot_ranges={:?}",
            self.nodes, self.slot_ranges
        );

        Ok(())
    }

    /// Keep existing connection, connect new nodes, remove obsolte ones
    /// Rebuild slot_ranges from scratch
    async fn refresh_nodes_and_slot_ranges(&mut self) -> Result<()> {
        debug!("Reloading slot ranges");

        let addresses = self.discovery_addresses();
        #[cfg_attr(not(test), allow(unused_mut))]
        let Some(mut shard_info_list) = Self::discover_shards(&addresses, &self.config).await
        else {
            return Err(Error::Client(ClientError::ClusterConfig));
        };

        // Test-only: simulate a discovery reply that describes no node at all.
        #[cfg(test)]
        if let Some(hook) = &self.test_hook
            && hook.take_empty_topology_on_refresh()
        {
            shard_info_list.clear();
        }

        // Refuse an unusable topology rather than applying it. Applying it would
        // empty `nodes`, and every later node lookup — the `select_all` in
        // `read()`, the random-node pick — indexes that collection and would
        // panic the network task, which owns all routing state. Nothing has been
        // mutated at this point, so the previous topology stays in place.
        if shard_info_list.is_empty() {
            warn!("Ignoring a cluster topology describing no node");
            return Err(Error::Client(ClientError::ClusterConfig));
        }

        // filter out nodes that do not exist anymore
        let mut node_ids = shard_info_list
            .iter()
            .flat_map(|s| s.nodes.iter().map(|n| n.id.as_str()))
            .collect::<Vec<_>>();
        node_ids.sort();
        self.nodes.retain(|node| {
            node_ids
                .binary_search_by(|n| (*n).cmp(node.id.as_ref()))
                .is_ok()
        });

        // create slot_ranges from scratch
        self.slot_ranges.clear();

        // add missing nodes and connect them
        for mut shard_info in shard_info_list {
            // ensure that the first node is master. A shard the server describes
            // with no node at all is a malformed topology, not something to index.
            let first_is_master = match shard_info.nodes.first() {
                Some(first) => first.role == "master",
                None => return Err(Error::Client(ClientError::ClusterConfig)),
            };
            if !first_is_master {
                let Some(master_idx) = shard_info.nodes.iter().position(|n| n.role == "master")
                else {
                    return Err(Error::Client(ClientError::ClusterConfig));
                };

                // swap first node & master node
                shard_info.nodes.swap(0, master_idx);
            }

            // add slot_ranges
            for slot_range_info in &shard_info.slots {
                self.slot_ranges.push(SlotRange {
                    slot_range: *slot_range_info,
                    node_ids: shard_info
                        .nodes
                        .iter()
                        .map(|n| n.id.as_str().into())
                        .collect(),
                });
            }

            for node_info in shard_info.nodes {
                let node_id: NodeId = node_info.id.as_str().into();
                if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                    // refresh is_master flag in case a failover happened
                    node.is_master = node_info.role == "master";
                } else {
                    // add missing node
                    let port = node_info.get_port()?;

                    let connection =
                        StandaloneConnection::connect(&node_info.ip, port, &self.config).await?;

                    self.nodes.push(Node {
                        id: node_id,
                        is_master: node_info.role == "master",
                        address: (node_info.ip, port),
                        connection,
                        is_dirty: false,
                    });
                }
            }
        }

        self.slot_ranges.sort_by_key(|s| s.slot_range.0);
        self.nodes.sort_by(|n1, n2| n1.id.cmp(&n2.id));

        debug!(
            "Cluster new setup: nodes={:?}, slot_ranges={:?}",
            self.nodes, self.slot_ranges
        );

        Ok(())
    }

    #[inline]
    fn get_node_index_by_id(&self, id: &NodeId) -> Option<usize> {
        self.nodes.binary_search_by_key(&id, |n| &n.id).ok()
    }

    #[inline]
    fn get_random_node_index(&self) -> Option<usize> {
        if self.nodes.is_empty() {
            return None;
        }
        Some(rand::rng().random_range(0..self.nodes.len()))
    }

    #[inline]
    fn get_slot_range_index(&self, slot: u16) -> Option<usize> {
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
    fn get_slot_range_by_slot(&self, slot: u16) -> Option<&SlotRange> {
        self.get_slot_range_index(slot)
            .and_then(|idx| self.slot_ranges.get(idx))
    }

    #[inline]
    fn get_slot_range_by_slot_mut(&mut self, slot: u16) -> Option<&mut SlotRange> {
        self.get_slot_range_index(slot)
            .and_then(|idx| self.slot_ranges.get_mut(idx))
    }

    fn get_master_node_index_by_slot(
        &mut self,
        slot: u16,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Option<(usize, bool)> {
        let ask_reason = ask_reasons
            .iter()
            .find(|(hash_slot, (_ip, _port))| *hash_slot == slot);

        if let Some((_hash_slot, address)) = ask_reason {
            let node_index = self.nodes.iter().position(|n| n.address == *address)?;
            Some((node_index, true))
        } else {
            let slot_range = self.get_slot_range_by_slot(slot)?;
            // A slot range names its master first; one with no node routes nowhere.
            let master_node_id = slot_range.node_ids.first()?;
            let node_index = self.get_node_index_by_id(master_node_id)?;
            Some((node_index, false))
        }
    }

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

    pub(crate) fn tag(&self) -> Arc<str> {
        self.tag.clone()
    }
}

pub fn prepare_command_for_shard(command: &Command, shard_keys: &[Bytes]) -> Command {
    // Initialize a new command with the same base name
    let mut shard_command = CommandBuilder::new(command.name());

    // Tracks how many subsequent arguments to keep after a valid key
    let mut keep_next = 0;

    // The step defines how many arguments form a logical group (e.g., 2 for MSET)
    let step = command.key_step();

    // Index the shard's keys once so the per-key membership test below is O(1)
    // instead of a linear `contains` scan — the latter is O(K²) per shard on a
    // large multi-key command (e.g. a 10k-key MGET).
    let shard_key_set: HashSet<&[u8]> = shard_keys.iter().map(|k| k.as_ref()).collect();

    // Iterate through all arguments using the cluster helper
    for (arg, is_key, _) in command.args_for_cluster() {
        if is_key {
            // If the current argument is a key, check if it exists in our shard group
            if shard_key_set.contains(arg.as_ref()) {
                shard_command = shard_command.arg(arg);
                // Keep the next (step - 1) arguments associated with this key
                keep_next = step - 1;
            } else {
                // Key belongs to another shard
                keep_next = 0;
            }
        } else if keep_next > 0 {
            // This is a value/path associated with an accepted key
            shard_command = shard_command.arg(arg);
            keep_next -= 1;
        }
    }

    shard_command.into()
}

enum Integer {
    Single(i64),
    Array(Vec<i64>),
    Null,
}
