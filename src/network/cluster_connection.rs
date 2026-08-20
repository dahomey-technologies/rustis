use super::cluster_request::{RequestInfo, SubRequest, collect_redirections};
use super::pub_sub_push::PubSubPush;
use crate::{
    ClientError, ConnectionState, Error, ErrorKind, Result, RetryReason, StandaloneConnection,
    client::{ClusterConfig, Config, ReadPreference},
    commands::{
        ClusterCommands, ClusterHealthStatus, ClusterNodeResult, ClusterShardResult,
        InternalCommands, LegacyClusterShardResult, RequestPolicy,
    },
    network::{Version, sleep},
    resp::{ClientReplyMode, Command, CommandBuilder, CommandKind, RespResponse, hash_slot},
};
use bytes::Bytes;
use futures_util::{FutureExt, future};
use rand::RngExt;
use smallvec::{SmallVec, smallvec};
use std::{
    cmp::Ordering,
    collections::{HashSet, VecDeque},
    fmt::{Debug, Formatter},
    sync::Arc,
    task::Poll,
    time::Duration,
};
use tokio::time::Instant;
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
    /// When set, the next sub-request result is replaced by this RESP error,
    /// reproducing a transient cluster reply (`TRYAGAIN`, `CLUSTERDOWN`) without
    /// having to catch a real resharding at the right microsecond.
    transient_error: Arc<std::sync::Mutex<Option<Bytes>>>,
    /// Counts every completed topology discovery, so a test can tell a refresh
    /// that happened on its own from one a redirection asked for.
    topology_refreshes: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "test-support code: a panic is how a test reports failure"
)]
impl ClusterTestHook {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Arms a one-shot removal of the node serving the oldest in-flight request.
    /// It is consumed only once such a request actually exists.
    pub(crate) fn arm_drop_front_pending_node(&self) {
        self.drop_front_pending_node
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn take_drop_front_pending_node(&self) -> bool {
        self.drop_front_pending_node
            .swap(false, std::sync::atomic::Ordering::SeqCst)
    }

    /// Arms a one-shot empty topology discovery on the next refresh.
    pub(crate) fn arm_empty_topology_on_refresh(&self) {
        self.empty_topology_on_refresh
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn take_empty_topology_on_refresh(&self) -> bool {
        self.empty_topology_on_refresh
            .swap(false, std::sync::atomic::Ordering::SeqCst)
    }

    /// Hides the shard holding `node_id` from the initial discovery only, so
    /// that a later refresh sees the real topology again.
    pub(crate) fn hide_node_on_initial_discovery(&self, node_id: &str) {
        *self.hidden_node_id.lock().unwrap() = Some(node_id.to_owned());
    }

    /// How many topology discoveries have completed on this connection.
    pub(crate) fn topology_refreshes(&self) -> usize {
        self.topology_refreshes
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    fn record_topology_refresh(&self) {
        self.topology_refreshes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn take_hidden_node_id(&self) -> Option<String> {
        self.hidden_node_id.lock().unwrap().take()
    }

    /// Arms a one-shot replacement of the next sub-request reply by the server
    /// error `error` (`"TRYAGAIN ..."`, `"CLUSTERDOWN ..."`).
    pub(crate) fn arm_transient_error_on_next_result(&self, error: &str) {
        *self.transient_error.lock().unwrap() = Some(Bytes::from(format!("-{error}\r\n")));
    }

    fn take_transient_error(&self) -> Option<Bytes> {
        self.transient_error.lock().unwrap().take()
    }
}

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

struct Node {
    pub id: NodeId,
    pub is_master: bool,
    pub address: (String, u16),
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
    pub(crate) async fn feed(
        &mut self,
        command: &Command,
        reply_skip: Option<&Command>,
    ) -> Result<()> {
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
struct SlotRange {
    pub slot_range: (u16, u16),
    /// node ids of the shard that owns the slot range,
    /// the first node id being the master node id
    pub node_ids: SmallVec<[NodeId; 6]>,
    /// Round-robin cursor over the replicas of the shard, used when the read
    /// preference sends read-only commands to them. It only has to advance, so
    /// it wraps freely: the candidate is picked modulo the replica count.
    pub next_replica: usize,
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

/// Whether the command subscribes to, or unsubscribes from, a plain channel or
/// a pattern. Those name no key, so nothing in the command itself says which
/// node must serve them: [`ClusterConnection::request_policy_pub_sub`] hashes
/// each channel name to pick one. Their shard counterparts — `SSUBSCRIBE` and
/// `SUNSUBSCRIBE` — name the shard channel as a key and route on its slot like
/// any other command.
fn is_broadcast_pub_sub_command(command: &Command) -> bool {
    matches!(
        command.name(),
        b"SUBSCRIBE" | b"PSUBSCRIBE" | b"UNSUBSCRIBE" | b"PUNSUBSCRIBE"
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
            _ => Err(Error::from(ClientError::ClusterConfig)),
        }
    }
}

/// Cluster connection
/// read & write_batch functions are implemented following Redis Command Tips
/// See <https://redis.io/docs/reference/command-tips/>
/// `interval` from now, capped rather than overflowing the monotonic clock.
fn deadline_after(interval: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(interval).unwrap_or(now)
}

pub(crate) struct ClusterConnection {
    cluster_config: ClusterConfig,
    config: Config,
    /// Read-only copy of the handler's connection-state registry, refreshed through
    /// `sync_connection_state`.
    ///
    /// A topology change creates node connections from inside `feed` / `read`, which
    /// the handler drives without lending its registry — `read` is polled in a
    /// `select!` over its other fields. Those nodes must still reach the state their
    /// siblings are in before anything is sent on them, and this is what lets them.
    state_snapshot: ConnectionState,
    nodes: Vec<Node>,
    slot_ranges: Vec<SlotRange>,
    pending_requests: VecDeque<RequestInfo>,
    /// Sub-requests re-armed by a partial redirection, awaiting the next `read`
    /// to be sent.
    pending_redirections: Vec<PendingRedirection>,
    tag: Arc<str>,
    /// Whether the nodes are answering, mirroring `CLIENT REPLY ON` / `OFF` — which
    /// is sent to all of them, so one flag describes the whole connection.
    ///
    /// While they are silent, no in-flight bookkeeping may be filed: a sub-request
    /// waiting for a reply that will never come sits at the head of
    /// `pending_requests` forever and stalls every caller behind it.
    is_reply_on: bool,
    /// A `CLIENT REPLY SKIP` held back until the command it silences is routed.
    ///
    /// It carries no routing policy of its own because it is only correct on the
    /// nodes that command reaches — one for a key-routed command, several for a
    /// multi-shard one. Same shape as the "Lazy MULTI" state below, and for the same
    /// reason: the target is known only once the next command arrives.
    pending_reply_skip: Option<Command>,
    /// State to manage the "Lazy MULTI" logic
    transaction_state: TransactionState,
    /// When the next proactive reload is due, `None` when there is none. The
    /// interval it is computed from lives on `cluster_config`.
    next_topology_refresh: Option<Instant>,
    /// Whether the topology has already been refreshed during the send batch
    /// currently being fed. Reset by `flush`, which ends that batch.
    refreshed_in_current_batch: bool,
    /// Whether the transient-error delay has already been awaited during the
    /// send batch currently being fed. Reset by `flush`, like the flag above:
    /// every command of a retried batch carries the same reasons, and the delay
    /// is owed once, not once per command.
    delayed_in_current_batch: bool,
    #[cfg(test)]
    test_hook: Option<ClusterTestHook>,
}

impl ClusterConnection {
    pub(crate) async fn connect(
        cluster_config: &ClusterConfig,
        config: &Config,
        connection_state: &mut ConnectionState,
    ) -> Result<ClusterConnection> {
        let (mut nodes, slot_ranges) =
            Self::connect_to_cluster(cluster_config, config, connection_state).await?;
        let first_node = nodes
            .get_mut(0)
            .ok_or_else(|| Error::from(ClientError::ClusterConfig))?;

        let tag = first_node.connection.tag();

        let mut cluster_connection = ClusterConnection {
            cluster_config: cluster_config.clone(),
            config: config.clone(),
            state_snapshot: connection_state.clone(),
            nodes,
            slot_ranges,
            pending_requests: VecDeque::new(),
            pending_redirections: Vec::new(),
            tag,
            is_reply_on: true,
            pending_reply_skip: None,
            transaction_state: TransactionState::default(),
            next_topology_refresh: cluster_config.topology_refresh_interval.map(deadline_after),
            refreshed_in_current_batch: false,
            delayed_in_current_batch: false,
            #[cfg(test)]
            test_hook: config.cluster_test_hook.clone(),
        };

        cluster_connection.connect_replicas_for_reads().await;

        Ok(cluster_connection)
    }

    /// Brings the replicas in when the read preference sends reads to them, so
    /// the first read is routed instead of waiting for an `AllNodes` command to
    /// discover them.
    ///
    /// A cluster whose replicas cannot be reached is still a working cluster:
    /// the failure is logged and every read falls back to its master.
    async fn connect_replicas_for_reads(&mut self) {
        if self.cluster_config.read_preference == ReadPreference::Master {
            return;
        }

        if let Err(e) = self.connect_replicas().await {
            warn!("Cannot connect the cluster replicas to read from: {e}");
        }
    }

    #[inline]
    pub(crate) async fn feed(
        &mut self,
        command: &Command,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        // The mode has to move before the command is routed, so that `file_request`
        // sees it: `CLIENT REPLY ON` is itself answered and must be filed, while
        // `OFF` is not answered and must not be.
        match command.kind() {
            CommandKind::ClientReply(ClientReplyMode::On) => self.is_reply_on = true,
            CommandKind::ClientReply(ClientReplyMode::Off) => self.is_reply_on = false,
            // Held back rather than sent: it belongs on the nodes the next command
            // reaches, which is only known once that command is routed.
            CommandKind::ClientReply(ClientReplyMode::Skip) => {
                self.pending_reply_skip = Some(command.clone());
                return Ok(());
            }
            _ => (),
        }

        // The skip travels with the command it silences, on every node that command
        // reached. It applies to nothing further — including when the routing below
        // fails, where it never reached a node at all and the handler has already
        // spent its own one-shot on the command that errored.
        let result = self.feed_routed(command, retry_reasons).await;
        self.pending_reply_skip = None;
        result
    }

    async fn feed_routed(
        &mut self,
        command: &Command,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
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

        // A transient cluster error means the command never ran: the slot is
        // mid-migration (`TRYAGAIN`) or the shard is briefly unavailable
        // (`CLUSTERDOWN`). The cluster spec asks the client to replay it after a
        // short pause, which is what this awaits. It holds the whole send batch,
        // and that is the point: the cluster just said it cannot serve this
        // slot, so racing back at it would only burn the message's attempts.
        if let Some(delay) = retry_reasons
            .iter()
            .filter_map(|r| match r {
                RetryReason::TryAgain { delay, .. } => Some(*delay),
                _ => None,
            })
            .max()
            && !self.delayed_in_current_batch
        {
            self.delayed_in_current_batch = true;
            debug!("waiting {delay:?} before replaying a transient cluster error");
            sleep(delay).await;

            if !self.refreshed_in_current_batch
                && retry_reasons.iter().any(|r| {
                    matches!(
                        r,
                        RetryReason::TryAgain {
                            refresh_topology: true,
                            ..
                        }
                    )
                })
            {
                self.refreshed_in_current_batch = true;
                // A cluster that is still down answers nothing usable; the
                // replay then goes to the topology already known and earns
                // another `CLUSTERDOWN`, which is a retry rather than a failure.
                if let Err(e) = self.refresh_nodes_and_slot_ranges().await {
                    warn!("Cannot refresh the topology after a CLUSTERDOWN: {e}");
                }
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

        // A held skip belongs to the caller's command, not to the `MULTI` released
        // here on its behalf, so it is set aside across that injection.
        let held_skip = self.pending_reply_skip.take();
        if let Some(multi_cmd) = self.transaction_state.pending_multi.take() {
            let (node_idx, _) = self.get_no_request_policy_node(command, &ask_reasons)?;
            self.feed_no_request_policy(&multi_cmd, node_idx, false)
                .await?;
            self.transaction_state.node_index = Some(node_idx);
        }
        self.pending_reply_skip = held_skip;

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
                    return Err(Error::from(ClientError::ExecCalledWithoutMulti));
                }
            }
            _ => self.internal_feed(command, &ask_reasons).await?,
        }

        Ok(())
    }

    /// Records the in-flight bookkeeping for a request — unless the nodes are silent,
    /// in which case there is no reply to match it against and filing it would park
    /// an unresolvable entry at the head of the queue.
    ///
    /// The single funnel for all four routing policies, so the decision is made once.
    fn file_request(&mut self, request_info: RequestInfo) {
        if self.is_reply_on && self.pending_reply_skip.is_none() {
            self.pending_requests.push_back(request_info);
        }
    }

    async fn internal_feed(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        trace!("Analyzing command {command:?}");

        // A channel-less UNSUBSCRIBE (or PUNSUBSCRIBE) names nothing to hash and
        // cancels every subscription the *connection* holds — which in a cluster
        // is spread over several nodes. It falls through to the ordinary routing
        // below, which serves it on a single node.
        if is_broadcast_pub_sub_command(command) && command.num_args() > 0 {
            return self.request_policy_pub_sub(command).await;
        }

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
    pub(crate) async fn flush(&mut self) -> Result<()> {
        // End of the send batch: allow the next one to refresh and delay again
        // if needed.
        self.refreshed_in_current_batch = false;
        self.delayed_in_current_batch = false;

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
        let reply_skip = self.pending_reply_skip.clone();

        for node in self.nodes.iter_mut().filter(|n| n.is_master) {
            node.feed(command, reply_skip.as_ref()).await?;
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

        self.file_request(request_info);

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
        let reply_skip = self.pending_reply_skip.clone();

        for node in self.nodes.iter_mut() {
            node.feed(command, reply_skip.as_ref()).await?;
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

        self.file_request(request_info);

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
        let for_read = self.may_read_from_replica(command);
        let mut node_slot_keys_ask = command
            .args_for_cluster()
            .filter_map(|(arg, is_key, slot)| {
                is_key.then(|| {
                    let (node_index, should_ask) = self
                        .get_node_index_by_slot(slot, ask_reasons, for_read)
                        .ok_or_else(|| Error::from(ClientError::ClusterConfig))?;
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
        // Each shard receives the skip before its own slice of the command, so each
        // suppresses exactly one reply — its own.
        let reply_skip = self.pending_reply_skip.clone();

        // Placeholder, overwritten on the first iteration: `last_node_index`
        // starts at a value no real index can equal. A node-less connection
        // cannot serve the non-empty work list above.
        let mut node = self
            .nodes
            .first_mut()
            .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;

        for (node_index, slot, key, should_ask) in node_slot_keys_ask {
            if slot != last_slot {
                if !current_slot_keys.is_empty() {
                    if last_should_ask {
                        node.connection.asking().await?;
                    }

                    let shard_command = prepare_command_for_shard(command, &current_slot_keys);
                    node.feed(&shard_command, reply_skip.as_ref()).await?;
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
                    .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;
                last_node_index = node_index;
            }
        }

        if last_should_ask {
            node.connection.asking().await?;
        }

        let shard_command = prepare_command_for_shard(command, &current_slot_keys);
        node.feed(&shard_command, reply_skip.as_ref()).await?;
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

        self.file_request(request_info);

        Ok(())
    }

    /// Routes a channel or pattern subscription command, which carries no key.
    ///
    /// A plain channel is not owned by any shard — the cluster broadcasts what
    /// is published on it — so any node may serve the subscription. What matters
    /// is that the *same* node serves the matching unsubscription: picked at
    /// random, the two land on different nodes as soon as the cluster has more
    /// than one, the node holding the subscription never hears about the
    /// cancellation and keeps the channel forever. Hashing the channel name like
    /// a key makes the choice deterministic, and spreads subscriptions over the
    /// shards instead of piling them on one node.
    ///
    /// The channels of a single command need not hash to the same node, so the
    /// command is split per node the way a multi-shard one is.
    async fn request_policy_pub_sub(&mut self, command: &Command) -> Result<()> {
        let mut node_channels: SmallVec<[(usize, SmallVec<[Bytes; 10]>); 10]> = smallvec![];

        for channel in command.args() {
            let (node_index, _should_ask) = self
                .get_node_index_by_slot(hash_slot(&channel), &[], false)
                .ok_or_else(|| Error::from(ClientError::ClusterConfig))?;

            match node_channels.iter_mut().find(|(i, _)| *i == node_index) {
                Some((_, channels)) => channels.push(channel),
                None => node_channels.push((node_index, smallvec![channel])),
            }
        }

        // Each node receives the skip before its own slice of the command, so
        // each suppresses exactly one reply — its own.
        let reply_skip = self.pending_reply_skip.clone();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for (node_index, channels) in node_channels {
            let mut builder = CommandBuilder::new(command.name());
            for channel in channels {
                builder = builder.arg(channel);
            }
            let node_command: Command = builder.into();

            let node = self
                .nodes
                .get_mut(node_index)
                .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;
            node.feed(&node_command, reply_skip.as_ref()).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: smallvec![],
                result: None,
            });
        }

        let request_info = RequestInfo {
            response_policy: command.response_policy(),
            sub_requests,
            keys: smallvec![],
            command: None,
            is_pub_sub: true,
            #[cfg(test)]
            command_seq: command.command_seq,
        };

        self.file_request(request_info);

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
        let for_read = self.may_read_from_replica(command);
        let mut slots = command.slots();

        if let Some(first_slot) = slots.next() {
            if !slots.all(|s| s == first_slot) {
                return Err(
                    Error::from(ClientError::MismatchedKeySlots).with_command(command.name_bytes())
                );
            }

            self.get_node_index_by_slot(first_slot, ask_reasons, for_read)
                .ok_or_else(|| Error::from(ClientError::ClusterConfig))
        } else {
            self.get_random_node_index()
                .map(|node_idx| (node_idx, false))
                .ok_or_else(|| Error::from(ClientError::ClusterConfig))
        }
    }

    async fn feed_no_request_policy(
        &mut self,
        command: &Command,
        node_idx: usize,
        should_ask: bool,
    ) -> Result<()> {
        let reply_skip = self.pending_reply_skip.clone();
        let node = self
            .nodes
            .get_mut(node_idx)
            .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;
        if should_ask {
            node.connection.asking().await?;
        }
        node.feed(command, reply_skip.as_ref()).await?;
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
        self.file_request(request_info);
        Ok(())
    }

    fn request_policy_special(&mut self, _command: &Command) -> Result<()> {
        Err(Error::from(ClientError::CommandNotSupportedInCluster))
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
            PubSubPush::try_from(response),
            Ok(PubSubPush::Subscribe(_)
                | PubSubPush::PSubscribe(_)
                | PubSubPush::SSubscribe(_)
                | PubSubPush::Unsubscribe(_)
                | PubSubPush::PUnsubscribe(_)
                | PubSubPush::SUnsubscribe(_))
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

    pub(crate) async fn read(&mut self) -> Option<Result<RespResponse>> {
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
                return Some(Err(Error::from(ErrorKind::DisconnectedByPeer)));
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
                return Some(Err(Error::from(ClientError::InconsistentRoutingState)));
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
                return Some(Err(Error::from(ClientError::UnexpectedMessageReceived)));
            };

            if !self.store_sub_request_result(req_idx, sub_req_idx, result) {
                return Some(Err(Error::from(ClientError::InconsistentRoutingState)));
            }
        }
    }

    pub(crate) fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
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
                return Poll::Ready(Some(Err(Error::from(ErrorKind::DisconnectedByPeer))));
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
                return Poll::Ready(Some(Err(Error::from(
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
                return Poll::Ready(Some(Err(Error::from(
                    ClientError::UnexpectedMessageReceived,
                ))));
            };

            if !self.store_sub_request_result(req_idx, sub_req_idx, result) {
                return Poll::Ready(Some(Err(Error::from(
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
        #[cfg_attr(not(test), allow(unused_mut))] mut result: Option<Result<RespResponse>>,
    ) -> bool {
        // Test-only: hand a transient cluster error to the next sub-request that
        // completes, in place of the reply the server actually sent.
        #[cfg(test)]
        if let Some(hook) = &self.test_hook
            && matches!(result, Some(Ok(_)))
            && let Some(error) = hook.take_transient_error()
        {
            let mut tape = crate::resp::RespTapeMut::default();
            let mut parser = crate::resp::RespFrameParser::new(&error, &mut tape);
            if let Ok((frame, _)) = parser.parse() {
                result = Some(Ok(RespResponse::new(error.into(), frame)));
            }
        }

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
                // Not a redirection: nothing to re-arm against, and the caller
                // falls back to replaying the whole command, which is where the
                // transient-error delay is awaited.
                RetryReason::TryAgain { .. } => return false,
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
                .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;
            if redirection.should_ask {
                node.connection.asking().await?;
            }
            // No skip here: this re-sends a sub-request of a request already filed,
            // whose reply is still expected.
            node.feed(&redirection.command, None).await?;
        }

        self.flush().await
    }

    fn internal_read(&mut self, mut request_info: RequestInfo) -> ReadOutcome {
        // A command split across shards whose sub-requests did not all fail must
        // not be replayed as a whole: the shards that answered already applied
        // it, and a second run reports different numbers — a replayed `DEL`
        // answers 0 for the keys it deleted the first time. Re-send only what
        // was redirected and keep the rest.
        let redirections = collect_redirections(&request_info);
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

        ReadOutcome::Ready(request_info.into_reply())
    }

    /// Refreshes the read-only copy the topology-change paths replay from.
    ///
    /// Called by the handler whenever it records connection state, which is the one
    /// place that state changes. Keeping the copy in step here is what lets
    /// `refresh_nodes_and_slot_ranges` restore a joining node without reaching back
    /// into the handler's registry.
    pub(crate) fn sync_connection_state(&mut self, connection_state: &ConnectionState) {
        self.state_snapshot = connection_state.clone();
    }

    pub(crate) async fn reconnect(&mut self, connection_state: &mut ConnectionState) -> Result<()> {
        info!("Reconnecting to cluster...");
        self.state_snapshot = connection_state.clone();
        let (nodes, slot_ranges) =
            Self::connect_to_cluster(&self.cluster_config, &self.config, connection_state).await?;
        info!("Reconnected to cluster!");

        self.nodes = nodes;
        self.slot_ranges = slot_ranges;

        self.connect_replicas_for_reads().await;

        // Every in-flight request was fed to the previous per-node connections,
        // which are now gone; their responses can never arrive. Left in place,
        // the request stuck at the front of the queue would block every
        // subsequent reply from surfacing (`read()` pops the front only once
        // all its sub-requests resolve) and hang all callers. Drop them here:
        // the network handler owns caller delivery and has already failed the
        // non-retryable messages and re-queued the retryable ones for replay,
        // which will repopulate `pending_requests` consistently.
        self.pending_requests.clear();

        // A skip still held belonged to a command that never reached the wire on the
        // socket that just died. The handler resets its own one-shot; keeping this one
        // would silence the first command of the new connection while that reply is
        // still expected, shifting every response after it.
        self.pending_reply_skip = None;

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
        connection_state: &mut ConnectionState,
    ) -> Result<(Vec<Node>, Vec<SlotRange>)> {
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

        let mut nodes = Vec::<Node>::new();
        let mut slot_ranges = Vec::<SlotRange>::new();

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

            slot_ranges.extend(shard_info.slots.iter().map(|s| SlotRange {
                slot_range: *s,
                node_ids: smallvec![master_id.clone()],
                next_replica: 0,
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

    async fn connect_replicas(&mut self) -> Result<()> {
        debug!("Connecting replicas...");

        let addresses = self.discovery_addresses();
        let Some(shard_info_list) = Self::discover_shards(&addresses, &self.config).await else {
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
                    StandaloneConnection::connect_control(&node_info.ip, port, &self.config)
                        .await?;
                connection.restore_from_snapshot(&self.state_snapshot).await;

                for slot_range_info in &shard_info.slots {
                    if let Some(slot_range) = self.get_slot_range_by_slot_mut(slot_range_info.0)
                        && slot_range.slot_range.1 == slot_range_info.1
                    {
                        slot_range.node_ids.push(node_id.clone())
                    }
                }

                Self::set_replica_read_mode(&mut connection, self.cluster_config.read_preference)
                    .await;

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
    ///
    /// Nodes appearing here are restored from [`Self::state_snapshot`]: a refresh runs
    /// inside `feed` / `read`, which the handler drives without lending its registry,
    /// so the snapshot is what makes the caller's state reach a joining shard.
    /// When the topology is next due to be reloaded on its own.
    ///
    /// A redirection is the only other thing that corrects the local slot map, so
    /// a healthy connection to a topology that has moved stays wrong until a
    /// command happens to be wrong — and a resharding that touches no slot this
    /// client uses is never noticed at all.
    pub(crate) fn next_maintenance(&self) -> Option<Instant> {
        self.next_topology_refresh
    }

    /// Reloads the topology and schedules the next reload.
    ///
    /// A failure is logged and not propagated: the previous topology is still in
    /// place and still serving, so giving up over a failed refresh would turn a
    /// stale map into no client at all. The next interval tries again.
    pub(crate) async fn run_maintenance(&mut self) {
        self.next_topology_refresh = self
            .cluster_config
            .topology_refresh_interval
            .map(deadline_after);

        if let Err(e) = self.refresh_nodes_and_slot_ranges().await {
            debug!("Cannot refresh the cluster topology: {e}");
        }
    }

    async fn refresh_nodes_and_slot_ranges(&mut self) -> Result<()> {
        debug!("Reloading slot ranges");

        #[cfg(test)]
        if let Some(hook) = &self.test_hook {
            hook.record_topology_refresh();
        }

        let addresses = self.discovery_addresses();
        #[cfg_attr(not(test), allow(unused_mut))]
        let Some(mut shard_info_list) = Self::discover_shards(&addresses, &self.config).await
        else {
            return Err(Error::from(ClientError::ClusterConfig));
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
            return Err(Error::from(ClientError::ClusterConfig));
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
                None => return Err(Error::from(ClientError::ClusterConfig)),
            };
            if !first_is_master {
                let Some(master_idx) = shard_info.nodes.iter().position(|n| n.role == "master")
                else {
                    return Err(Error::from(ClientError::ClusterConfig));
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
                    next_replica: 0,
                });
            }

            for node_info in shard_info.nodes {
                let node_id: NodeId = node_info.id.as_str().into();
                if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
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
                            self.cluster_config.read_preference,
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
                        StandaloneConnection::connect_control(&node_info.ip, port, &self.config)
                            .await?;
                    connection.restore_from_snapshot(&self.state_snapshot).await;

                    if node_info.role != "master" {
                        Self::set_replica_read_mode(
                            &mut connection,
                            self.cluster_config.read_preference,
                        )
                        .await;
                    }

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

    /// The node a command addressing `slot` must be fed to, and whether it has to
    /// be prefixed with an `ASKING`.
    ///
    /// `for_read` asks for the configured read preference to be honoured. It is
    /// the caller's job to answer it only for a command that may legitimately
    /// leave the master — see [`Self::may_read_from_replica`].
    fn get_node_index_by_slot(
        &mut self,
        slot: u16,
        ask_reasons: &[(u16, (String, u16))],
        for_read: bool,
    ) -> Option<(usize, bool)> {
        let ask_reason = ask_reasons
            .iter()
            .find(|(hash_slot, (_ip, _port))| *hash_slot == slot);

        // An ASK names the node itself: the redirection is the routing decision,
        // and the read preference has nothing to say about it.
        if let Some((_hash_slot, address)) = ask_reason {
            let node_index = self.nodes.iter().position(|n| n.address == *address)?;
            return Some((node_index, true));
        }

        if for_read && let Some(node_index) = self.get_replica_node_index_by_slot(slot) {
            return Some((node_index, false));
        }

        let slot_range = self.get_slot_range_by_slot(slot)?;
        // A slot range names its master first; one with no node routes nowhere.
        let master_node_id = slot_range.node_ids.first()?;
        let node_index = self.get_node_index_by_id(master_node_id)?;
        Some((node_index, false))
    }

    /// The next replica of the shard owning `slot`, or `None` when the shard has
    /// no connected one — in which case the caller falls back to the master
    /// rather than failing the command.
    fn get_replica_node_index_by_slot(&mut self, slot: u16) -> Option<usize> {
        let slot_range_index = self.get_slot_range_index(slot)?;
        let slot_range = self.slot_ranges.get(slot_range_index)?;

        // The master heads the list; everything after it is a replica.
        let replica_ids: SmallVec<[NodeId; 6]> =
            slot_range.node_ids.iter().skip(1).cloned().collect();
        if replica_ids.is_empty() {
            return None;
        }

        let mut cursor = slot_range.next_replica;
        let node_index = select_replica(&replica_ids, &mut cursor, |id| {
            self.get_node_index_by_id(id)
        })?;

        if let Some(slot_range) = self.slot_ranges.get_mut(slot_range_index) {
            slot_range.next_replica = cursor;
        }

        Some(node_index)
    }

    /// Whether `command` may be served by a replica: the preference asks for it,
    /// the command only reads, and it is not part of a block that belongs to a
    /// single node.
    fn may_read_from_replica(&self, command: &Command) -> bool {
        self.cluster_config.read_preference == ReadPreference::PreferReplica
            && command.is_readonly()
            && !is_pub_sub_command(command)
            // A MULTI locks one node for the whole transaction; a read of that
            // block sent elsewhere would leave the queue behind.
            && self.transaction_state.pending_multi.is_none()
            && self.transaction_state.node_index.is_none()
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

pub(crate) fn prepare_command_for_shard(command: &Command, shard_keys: &[Bytes]) -> Command {
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
                // Keep the next (step - 1) arguments associated with this key.
                // Every `MultiShard` command declares a step of at least 1 through
                // `cluster_info`, but the step is read from a public getter whose
                // default is 0, and this runs on the network task: a step of 0
                // keeps no trailing argument rather than underflowing.
                keep_next = step.saturating_sub(1);
            } else {
                // Key belongs to another shard
                keep_next = 0;
            }
        } else if let Some(remaining) = keep_next.checked_sub(1) {
            // This is a value/path associated with an accepted key
            shard_command = shard_command.arg(arg);
            keep_next = remaining;
        }
    }

    shard_command.into()
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
    use super::{NodeId, select_replica};

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
}
