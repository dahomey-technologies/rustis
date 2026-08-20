use super::cluster_reply_mode::ClusterReplyMode;
use super::cluster_request::{
    RequestInfo, RequestQueue, SubRequest, collect_redirections, is_pub_sub_command,
};
use super::cluster_send_batch::SendBatch;
use super::cluster_topology::{ClusterNodeAddress, ClusterTopology, NodeId, NodeReach};
use super::pub_sub_push::PubSubPush;
use crate::{
    ClientError, ConnectionState, Error, ErrorKind, Result, RetryReason,
    client::{ClusterConfig, Config, ReadPreference},
    commands::{ClusterNodeResult, InternalCommands, RequestPolicy},
    network::sleep,
    resp::{Command, CommandBuilder, RespResponse, hash_slot},
};
use bytes::Bytes;
use smallvec::{SmallVec, smallvec};
use std::{collections::HashSet, fmt::Debug, sync::Arc, task::Poll, time::Duration};
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

    pub(super) fn take_hidden_node_id(&self) -> Option<String> {
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

/// One shard's slice of a multi-key command: the keys of a single slot, the node
/// that serves them, and whether the send must be prefixed with an `ASKING`.
#[derive(Debug, PartialEq, Eq)]
struct ShardSlice {
    node_index: usize,
    keys: SmallVec<[Bytes; 10]>,
    should_ask: bool,
}

/// Cuts the routed keys into one slice per slot.
///
/// `routed_keys` must be sorted, which groups a slot's keys together. A slot is
/// served as a whole: the read preference resolves each key on its own and its
/// replica round-robin can name two different nodes for one slot, in which case
/// the last one takes all of its keys. Splitting them would file two
/// sub-requests for one slot and reassemble the reply against the wrong keys.
fn shard_slices(routed_keys: Vec<(usize, u16, Bytes, bool)>) -> Vec<ShardSlice> {
    let mut slices = Vec::<ShardSlice>::new();
    let mut current_slot = None;

    for (node_index, slot, key, should_ask) in routed_keys {
        match slices.last_mut() {
            Some(slice) if current_slot == Some(slot) => {
                slice.node_index = node_index;
                slice.keys.push(key);
            }
            _ => {
                current_slot = Some(slot);
                slices.push(ShardSlice {
                    node_index,
                    keys: smallvec![key],
                    should_ask,
                });
            }
        }
    }

    slices
}

/// The `ASK` redirections among the retry reasons, as the slot they name and the
/// node that is importing it.
fn ask_reasons(retry_reasons: &[RetryReason]) -> Vec<(u16, ClusterNodeAddress)> {
    retry_reasons
        .iter()
        .filter_map(|r| match r {
            RetryReason::Ask { hash_slot, address } => Some((*hash_slot, address.clone())),
            _ => None,
        })
        .collect()
}

/// Whether a push frame is a subscription command's acknowledgement, as opposed
/// to a published message.
///
/// Only an acknowledgement retires the request its command left behind. An error
/// reply such as `MOVED` is not a push frame at all and is filed like any other,
/// so the redirection path keeps working.
fn is_subscription_ack(response: &RespResponse) -> bool {
    matches!(
        PubSubPush::try_from(response),
        Ok(PubSubPush::Subscribe(_)
            | PubSubPush::PSubscribe(_)
            | PubSubPush::SSubscribe(_)
            | PubSubPush::Unsubscribe(_)
            | PubSubPush::PUnsubscribe(_)
            | PubSubPush::SUnsubscribe(_))
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

/// `interval` from now, capped rather than overflowing the monotonic clock.
pub(crate) fn deadline_after(interval: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(interval).unwrap_or(now)
}

/// Cluster connection.
///
/// `feed` and `read` route a command by the Redis command tips — a request
/// policy says which nodes it reaches, a response policy says how their replies
/// become one. See <https://redis.io/docs/reference/command-tips/>.
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
    topology: ClusterTopology,
    pending_requests: RequestQueue,
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
    /// A `CLIENT REPLY SKIP` held back until the command it silences is routed.
    ///
    /// It carries no routing policy of its own because it is only correct on the
    /// nodes that command reaches — one for a key-routed command, several for a
    /// multi-shard one. Same shape as the "Lazy MULTI" state below, and for the same
    /// reason: the target is known only once the next command arrives.
    reply_mode: ClusterReplyMode,
    /// State to manage the "Lazy MULTI" logic
    transaction_state: TransactionState,
    /// When the next proactive reload is due, `None` when there is none. The
    /// interval it is computed from lives on `cluster_config`.
    next_topology_refresh: Option<Instant>,
    /// Whether the topology has already been refreshed during the send batch
    /// currently being fed. Reset by `flush`, which ends that batch.
    /// Whether the transient-error delay has already been awaited during the
    /// send batch currently being fed. Reset by `flush`, like the flag above:
    /// every command of a retried batch carries the same reasons, and the delay
    /// is owed once, not once per command.
    send_batch: SendBatch,
    #[cfg(test)]
    test_hook: Option<ClusterTestHook>,
}

impl ClusterConnection {
    pub(crate) async fn connect(
        cluster_config: &ClusterConfig,
        config: &Config,
        connection_state: &mut ConnectionState,
    ) -> Result<ClusterConnection> {
        let topology = ClusterTopology::discover(cluster_config, config, connection_state).await?;
        let tag = topology
            .node(0)
            .ok_or_else(|| Error::from(ClientError::ClusterConfig))?
            .connection
            .tag();

        let mut cluster_connection = ClusterConnection {
            cluster_config: cluster_config.clone(),
            config: config.clone(),
            state_snapshot: connection_state.clone(),
            topology,
            pending_requests: RequestQueue::default(),
            pending_redirections: Vec::new(),
            tag,
            reply_mode: ClusterReplyMode::new(),
            transaction_state: TransactionState::default(),
            next_topology_refresh: cluster_config.topology_refresh_interval.map(deadline_after),
            send_batch: SendBatch::default(),
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

        if let Err(e) = self
            .topology
            .connect_replicas(&self.cluster_config, &self.config, &self.state_snapshot)
            .await
        {
            warn!("Cannot connect the cluster replicas to read from: {e}");
        }
    }

    #[inline]
    pub(crate) async fn feed(
        &mut self,
        command: &Command,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        if !self.reply_mode.admit(command) {
            return Ok(());
        }

        // The skip travels with the command it silences, on every node that command
        // reached. It applies to nothing further — including when the routing below
        // fails, where it never reached a node at all and the handler has already
        // spent its own one-shot on the command that errored.
        let result = self.feed_routed(command, retry_reasons).await;
        self.reply_mode.forget_held_skip();
        result
    }

    async fn feed_routed(
        &mut self,
        command: &Command,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        self.absorb_retry_reasons(retry_reasons).await?;

        let ask_reasons = ask_reasons(retry_reasons);
        self.reach_unknown_ask_targets(&ask_reasons).await?;
        self.release_pending_multi(command, &ask_reasons).await?;

        match command.name() {
            b"MULTI" => {
                // We do not send it to the network yet. We wait for the first key-based command
                // to decide which shard owns this transaction.
                self.transaction_state.pending_multi = Some(command.clone());
            }
            b"EXEC" => {
                let Some(node_idx) = self.transaction_state.node_index else {
                    return Err(Error::from(ClientError::ExecCalledWithoutMulti));
                };
                self.feed_no_request_policy(command, node_idx, false)
                    .await?;
                self.transaction_state = TransactionState::default();
            }
            _ => self.internal_feed(command, &ask_reasons).await?,
        }

        Ok(())
    }

    /// Pays what the retry reasons ask for before the command is routed again:
    /// a stale slot map is reloaded, a transient failure is waited out.
    ///
    /// Both are owed once per send batch, not once per command — see
    /// [`SendBatch`].
    async fn absorb_retry_reasons(&mut self, retry_reasons: &[RetryReason]) -> Result<()> {
        // A `MOVED` says the slot map is stale, so reload it before routing
        // anything else: without this every later command on that slot earns
        // its own redirection.
        if retry_reasons
            .iter()
            .any(|r| matches!(r, RetryReason::Moved { .. }))
            && self.send_batch.claim_topology_refresh()
        {
            self.refresh_nodes_and_slot_ranges().await?;
        }

        // A transient cluster error means the command never ran: the slot is
        // mid-migration (`TRYAGAIN`) or the shard is briefly unavailable
        // (`CLUSTERDOWN`). The cluster spec asks the client to replay it after a
        // short pause, which is what this awaits. It holds the whole send batch,
        // and that is the point: the cluster just said it cannot serve this
        // slot, so racing back at it would only burn the message's attempts.
        let Some(delay) = retry_reasons
            .iter()
            .filter_map(|r| match r {
                RetryReason::TryAgain { delay, .. } => Some(*delay),
                _ => None,
            })
            .max()
        else {
            return Ok(());
        };

        if !self.send_batch.claim_transient_delay() {
            return Ok(());
        }

        debug!("waiting {delay:?} before replaying a transient cluster error");
        sleep(delay).await;

        let asks_for_reload = retry_reasons.iter().any(|r| {
            matches!(
                r,
                RetryReason::TryAgain {
                    refresh_topology: true,
                    ..
                }
            )
        });

        if asks_for_reload
            && self.send_batch.claim_topology_refresh()
            // A cluster that is still down answers nothing usable; the replay
            // then goes to the topology already known and earns another
            // `CLUSTERDOWN`, which is a retry rather than a failure.
            && let Err(e) = self.refresh_nodes_and_slot_ranges().await
        {
            warn!("Cannot refresh the topology after a CLUSTERDOWN: {e}");
        }

        Ok(())
    }

    /// Reloads the topology when an `ASK` points at a node it does not know.
    ///
    /// An `ASK` names the node importing the slot, which may have joined, or
    /// only been learned about, after the last discovery. Unlike a `MOVED` it
    /// invalidates nothing, so nothing else would ever bring that node in and
    /// the command would fail outright, where the cluster spec requires the
    /// redirection to be followed.
    async fn reach_unknown_ask_targets(
        &mut self,
        ask_reasons: &[(u16, ClusterNodeAddress)],
    ) -> Result<()> {
        let unknown = ask_reasons
            .iter()
            .any(|(_hash_slot, address)| !self.topology.holds_address(address));

        if unknown && self.send_batch.claim_topology_refresh() {
            self.refresh_nodes_and_slot_ranges().await?;
        }

        Ok(())
    }

    /// Sends the `MULTI` held back until a command named the shard that owns the
    /// transaction, and locks that node for the rest of the block.
    ///
    /// The held skip belongs to the caller's command, not to the `MULTI` released
    /// here on its behalf, so it is set aside across that injection.
    async fn release_pending_multi(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, ClusterNodeAddress)],
    ) -> Result<()> {
        let Some(multi_cmd) = self.transaction_state.pending_multi.take() else {
            return Ok(());
        };

        let held_skip = self.reply_mode.lift_held_skip();
        let (node_idx, _) = self.get_no_request_policy_node(command, ask_reasons)?;
        let result = self
            .feed_no_request_policy(&multi_cmd, node_idx, false)
            .await;
        self.reply_mode.restore_held_skip(held_skip);

        result?;
        self.transaction_state.node_index = Some(node_idx);

        Ok(())
    }

    /// Records the in-flight bookkeeping for a request — unless the nodes are silent,
    /// in which case there is no reply to match it against and filing it would park
    /// an unresolvable entry at the head of the queue.
    ///
    /// The single funnel for all four routing policies, so the decision is made once.
    fn file_request(&mut self, request_info: RequestInfo) {
        if self.reply_mode.awaits_a_reply() {
            self.pending_requests.push(request_info);
        }
    }

    async fn internal_feed(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, (String, u16))],
    ) -> Result<()> {
        trace!("Analyzing command {command:?}");

        if is_broadcast_pub_sub_command(command) {
            return if command.num_args() > 0 {
                self.request_policy_pub_sub(command).await
            } else {
                // A channel-less UNSUBSCRIBE (or PUNSUBSCRIBE) names nothing to
                // hash, and cancels every subscription the *connection* holds —
                // which in a cluster is spread over the masters. Served by one
                // node it cancels that node's share and silently leaves the rest,
                // so every master hears it.
                self.request_policy_all_shards(command).await
            };
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
        // End of the send batch: the next one owes its refresh and its delay again.
        self.send_batch.end();

        self.topology.flush_fed_nodes().await
    }

    /// The client should execute the command on all master shards (e.g., the DBSIZE command).
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_shards(&mut self, command: &Command) -> Result<()> {
        let reply_skip = self.reply_mode.held_skip().cloned();
        let sub_requests = self
            .topology
            .feed_each(command, reply_skip.as_ref(), NodeReach::Masters)
            .await?
            .into_iter()
            .map(SubRequest::keyless)
            .collect();

        self.file_request(RequestInfo::new(command, sub_requests));

        Ok(())
    }

    /// The client should execute the command on all nodes - masters and replicas alike.
    /// An example is the CONFIG SET command.
    /// This tip is in-use by commands that don't accept key name arguments.
    /// The command operates atomically per shard.
    async fn request_policy_all_nodes(&mut self, command: &Command) -> Result<()> {
        if self.topology.holds_no_replica() {
            self.topology
                .connect_replicas(&self.cluster_config, &self.config, &self.state_snapshot)
                .await?;
        }
        let reply_skip = self.reply_mode.held_skip().cloned();
        let sub_requests = self
            .topology
            .feed_each(command, reply_skip.as_ref(), NodeReach::All)
            .await?
            .into_iter()
            .map(SubRequest::keyless)
            .collect();

        self.file_request(RequestInfo::new(command, sub_requests));

        Ok(())
    }

    /// The client should execute the command on multiple shards.
    /// The shards that execute the command are determined by the hash slots of its input key name arguments.
    /// Examples for such commands include MSET, MGET and DEL.
    /// However, note that SUNIONSTORE isn't considered as multi_shard because all of its keys must belong to the same hash slot.
    async fn request_policy_multi_shard(
        &mut self,
        command: &Command,
        ask_reasons: &[(u16, ClusterNodeAddress)],
    ) -> Result<()> {
        let for_read = self.may_read_from_replica(command);
        let mut routed_keys = command
            .args_for_cluster()
            .filter_map(|(arg, is_key, slot)| {
                is_key.then(|| {
                    let (node_index, should_ask) = self
                        .topology
                        .node_index_by_slot(slot, ask_reasons, for_read)
                        .ok_or_else(|| Error::from(ClientError::ClusterConfig))?;
                    Ok((node_index, slot, arg, should_ask))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        if routed_keys.is_empty() {
            return Ok(());
        }

        // Sorting brings a slot's keys together, which is what the grouping
        // below walks.
        routed_keys.sort();
        trace!("routed_keys: {routed_keys:?}");

        // Each shard receives the skip before its own slice of the command, so each
        // suppresses exactly one reply — its own.
        let reply_skip = self.reply_mode.held_skip().cloned();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for slice in shard_slices(routed_keys) {
            let node = self
                .topology
                .node_mut(slice.node_index)
                .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;

            if slice.should_ask {
                node.connection.asking().await?;
            }

            let shard_command = prepare_command_for_shard(command, &slice.keys);
            node.feed(&shard_command, reply_skip.as_ref()).await?;
            sub_requests.push(SubRequest {
                node_id: node.id.clone(),
                keys: slice.keys,
                result: None,
            });
        }

        let request_info = RequestInfo::new(command, sub_requests).replayable_per_shard(command);

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
                .topology
                .node_index_by_slot(hash_slot(&channel), &[], false)
                .ok_or_else(|| Error::from(ClientError::ClusterConfig))?;

            match node_channels.iter_mut().find(|(i, _)| *i == node_index) {
                Some((_, channels)) => channels.push(channel),
                None => node_channels.push((node_index, smallvec![channel])),
            }
        }

        // Each node receives the skip before its own slice of the command, so
        // each suppresses exactly one reply — its own.
        let reply_skip = self.reply_mode.held_skip().cloned();
        let mut sub_requests = SmallVec::<[SubRequest; 10]>::new();

        for (node_index, channels) in node_channels {
            let mut builder = CommandBuilder::new(command.name());
            for channel in channels {
                builder = builder.arg(channel);
            }
            let node_command: Command = builder.into();

            let node = self
                .topology
                .node_mut(node_index)
                .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;
            node.feed(&node_command, reply_skip.as_ref()).await?;
            sub_requests.push(SubRequest::keyless(node.id.clone()));
        }

        self.file_request(RequestInfo::new(command, sub_requests));

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

            self.topology
                .node_index_by_slot(first_slot, ask_reasons, for_read)
                .ok_or_else(|| Error::from(ClientError::ClusterConfig))
        } else {
            self.topology
                .random_node_index()
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
        let reply_skip = self.reply_mode.held_skip().cloned();
        let node = self
            .topology
            .node_mut(node_idx)
            .ok_or_else(|| Error::from(ClientError::InconsistentRoutingState))?;
        if should_ask {
            node.connection.asking().await?;
        }
        node.feed(command, reply_skip.as_ref()).await?;
        let sub_request = SubRequest {
            node_id: node.id.clone(),
            keys: command.keys().collect(),
            result: None,
        };
        self.file_request(RequestInfo::new(command, smallvec![sub_request]));
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

        let Some(victim) = self.pending_requests.front_awaited_node_id() else {
            return;
        };

        // Keep at least one node so the cluster stays usable.
        if self.topology.node_count() < 2 || !hook.take_drop_front_pending_node() {
            return;
        }

        self.topology.drop_node(&victim);
        debug!("test hook removed node {victim:?}");
    }

    /// Whether the oldest request waits on a node the topology no longer holds.
    fn front_awaits_a_missing_node(&self) -> bool {
        self.pending_requests
            .front_awaits_a_missing_node(|node_id| {
                self.topology.node_index_by_id(node_id).is_some()
            })
    }

    /// Files a node's reply against the sub-request awaiting it, reporting
    /// `false` when no request expected one.
    fn file_reply(
        &mut self,
        node_idx: usize,
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

        let Some(node) = self.topology.node(node_idx) else {
            return false;
        };
        let node_id = node.id.clone();

        if !self.pending_requests.file_reply(&node_id, result) {
            error!(node = %node_id.as_ref(), "Received a reply no request awaited");
            return false;
        }

        true
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
            if self.front_awaits_a_missing_node() {
                self.pending_requests.pop_front();
                return Some(Err(Error::from(ErrorKind::DisconnectedByPeer)));
            }

            if let Some(request_info) = self.pending_requests.take_fulfilled_front() {
                match self.internal_read(request_info) {
                    ReadOutcome::Ready(result) => return result,
                    ReadOutcome::Deferred => continue,
                }
            }

            // A node-less cluster connection cannot serve anything: report it as
            // a disconnection so the handler reconnects and rediscovers the
            // topology.
            let Some((node_idx, result)) = self.topology.read_any().await else {
                warn!("No cluster node available to read from");
                return None;
            };

            result.as_ref()?;

            if let Some(Ok(response)) = &result
                && response.is_push()
            {
                if is_subscription_ack(response)
                    && let Some(node_id) = self.topology.node(node_idx).map(|node| node.id.clone())
                {
                    self.pending_requests.retire_pub_sub(&node_id);
                }
                return result;
            }

            if !self.file_reply(node_idx, result) {
                return Some(Err(Error::from(ClientError::UnexpectedMessageReceived)));
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
            if self.front_awaits_a_missing_node() {
                self.pending_requests.pop_front();
                return Poll::Ready(Some(Err(Error::from(ErrorKind::DisconnectedByPeer))));
            }

            if let Some(request_info) = self.pending_requests.take_fulfilled_front() {
                match self.internal_read(request_info) {
                    ReadOutcome::Ready(result) => return Poll::Ready(result),
                    ReadOutcome::Deferred => return Poll::Pending,
                }
            }

            // See `read()`: a node-less connection cannot serve anything.
            let (node_idx, result) = match self.topology.try_read_any() {
                Poll::Ready(Some(read)) => read,
                Poll::Ready(None) => {
                    warn!("No cluster node available to read from");
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            };

            if let Some(Ok(response)) = &result
                && response.is_push()
            {
                if is_subscription_ack(response)
                    && let Some(node_id) = self.topology.node(node_idx).map(|node| node.id.clone())
                {
                    self.pending_requests.retire_pub_sub(&node_id);
                }
                return Poll::Ready(result);
            }

            if !self.file_reply(node_idx, result) {
                return Poll::Ready(Some(Err(Error::from(
                    ClientError::UnexpectedMessageReceived,
                ))));
            }
        }
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

            let Some(node) = self.topology.node_by_address(address) else {
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
            let Some(node_index) = self.topology.node_index_by_id(&redirection.node_id) else {
                warn!("Redirection target {:?} is gone", redirection.node_id);
                continue;
            };

            let node = self
                .topology
                .node_mut(node_index)
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
        let topology =
            ClusterTopology::discover(&self.cluster_config, &self.config, connection_state).await?;
        info!("Reconnected to cluster!");

        self.topology = topology;

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

        // A skip still held belonged to a command that never reached the wire on
        // the socket that just died. The handler resets its own one-shot.
        self.reply_mode.forget_held_skip();

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

    /// Reloads the topology from the cluster: existing connections are kept,
    /// joining nodes are connected, departed ones are dropped, and the slot map
    /// is rebuilt from scratch.
    ///
    /// A refresh runs inside `feed` / `read`, which the handler drives without
    /// lending its registry, so [`Self::state_snapshot`] is what makes the
    /// caller's state reach a joining shard.
    async fn refresh_nodes_and_slot_ranges(&mut self) -> Result<()> {
        debug!("Reloading slot ranges");

        #[cfg(test)]
        if let Some(hook) = &self.test_hook {
            hook.record_topology_refresh();
        }

        let addresses = self.topology.discovery_addresses(&self.cluster_config);
        #[cfg_attr(not(test), allow(unused_mut))]
        let Some(mut shard_info_list) =
            ClusterTopology::discover_shards(&addresses, &self.config).await
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

        self.topology
            .apply(
                shard_info_list,
                &self.cluster_config,
                &self.config,
                &self.state_snapshot,
            )
            .await
    }

    #[inline]
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

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::shard_slices;
    use bytes::Bytes;

    fn key(name: &str) -> Bytes {
        Bytes::from(name.to_owned())
    }

    fn slices(mut routed: Vec<(usize, u16, Bytes, bool)>) -> Vec<(usize, Vec<String>, bool)> {
        routed.sort();
        shard_slices(routed)
            .into_iter()
            .map(|slice| {
                (
                    slice.node_index,
                    slice
                        .keys
                        .iter()
                        .map(|k| String::from_utf8_lossy(k).into_owned())
                        .collect(),
                    slice.should_ask,
                )
            })
            .collect()
    }

    /// One slice per slot, not per node: a shard owning two slots is fed twice
    /// and owes two replies, which is what the reassembly lines up against the
    /// keys of each.
    #[test]
    fn a_node_serving_two_slots_is_cut_into_two_slices() {
        let cut = slices(vec![
            (0, 10, key("a"), false),
            (0, 20, key("b"), false),
            (0, 10, key("c"), false),
        ]);

        assert_eq!(
            vec![
                (0, vec!["a".to_owned(), "c".to_owned()], false),
                (0, vec!["b".to_owned()], false),
            ],
            cut
        );
    }

    /// A command spread over two shards is cut per shard, each slice naming its
    /// own node.
    #[test]
    fn keys_of_different_shards_are_cut_apart() {
        let cut = slices(vec![
            (1, 500, key("x"), false),
            (0, 10, key("a"), false),
            (1, 500, key("y"), false),
        ]);

        assert_eq!(
            vec![
                (0, vec!["a".to_owned()], false),
                (1, vec!["x".to_owned(), "y".to_owned()], false),
            ],
            cut
        );
    }

    /// An `ASK` applies to the whole slice: the redirection names the slot, and
    /// the `ASKING` prefixes the one send that carries its keys.
    #[test]
    fn a_redirected_slot_carries_its_ask_for_the_whole_slice() {
        let cut = slices(vec![
            (0, 10, key("a"), true),
            (0, 10, key("b"), true),
            (1, 20, key("c"), false),
        ]);

        assert_eq!(
            vec![
                (0, vec!["a".to_owned(), "b".to_owned()], true),
                (1, vec!["c".to_owned()], false),
            ],
            cut
        );
    }

    /// A slot is served as a whole even when the read preference resolved its
    /// keys to two different replicas. Two slices for one slot would file two
    /// sub-requests where the command was split once.
    #[test]
    fn a_slot_resolved_to_two_replicas_stays_one_slice() {
        let cut = slices(vec![(0, 10, key("a"), false), (1, 10, key("b"), false)]);

        assert_eq!(1, cut.len());
        let (node_index, keys, _) = cut.first().unwrap();
        assert_eq!(1, *node_index, "the last resolution takes the slot");
        assert_eq!(&vec!["a".to_owned(), "b".to_owned()], keys);
    }
}
