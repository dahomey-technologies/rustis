use super::pub_sub_push::PubSubPush;
use crate::{
    ClientError, Connection, ConnectionState, Error, ErrorKind, JoinHandle, ReconnectionState,
    RedisError, RedisErrorKind, Result, RetryReason,
    client::{Config, Message, MessageKind, PreparedCommand},
    commands::InternalPubSubCommands,
    resp::{
        ClientReplyMode, CommandKind, RespResponse, RespView, StateSlot, SubscriptionType, cmd,
    },
    spawn, timeout,
};
use bytes::Bytes;
use futures_util::{FutureExt, select};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::{
    collections::{HashMap, VecDeque},
    future::poll_fn,
    sync::Arc,
    task::Poll,
    time::Duration,
};
use tokio::{sync::broadcast, time::Instant};
use tracing::{Instrument, debug, error, info, info_span, trace, warn};

// Backpressure note. Nothing here ever blocks a sender: the network task owns
// the connection's whole routing state, so making it wait on a consumer would
// stall every other caller. Memory is bounded by shedding instead, and each
// place that can grow sheds in the way that suits what it carries.
//
// - `messages_to_send` grows while the connection is down and every reconnection
//   fails. It is capped by `BackpressureConfig::max_queued_bytes`, enforced on
//   *incoming* messages only: anything replayed or retried was already accepted
//   and must not be dropped. Left uncapped it was measured retaining 100 000
//   commands and 229 MiB.
// - Pub/sub streams and the push sinks are bounded channels that discard their
//   oldest messages, so a consumer that resumes sees current data. A paused
//   subscriber was measured absorbing 113 MiB/s and 221 MiB before this existed.
//
// The request channel (`MsgSender`) stays unbounded on purpose. The task drains
// it into `messages_to_send` continuously, even while disconnected, so it holds
// almost nothing: of the 100 000 commands accumulated in that measurement,
// essentially all sat in the queue downstream. Capping it would never fire.
// Its senders are synchronous anyway (`send_and_forget`, `forget`, the stream
// `Drop` impls), so a bound there would have to reject rather than wait.
pub(crate) type MsgSender = tokio::sync::mpsc::UnboundedSender<Message>;
pub(crate) type MsgReceiver = tokio::sync::mpsc::UnboundedReceiver<Message>;
/// Retry-only handle the network task keeps on the message channel. It is
/// [`Weak`](tokio::sync::mpsc::WeakUnboundedSender) on purpose: holding a strong
/// sender would keep the channel open forever, so dropping the last client
/// would never end the network loop. With a weak handle the channel closes
/// naturally when the last client is dropped, and the task upgrades it only to
/// requeue a message for retry.
type WeakMsgSender = tokio::sync::mpsc::WeakUnboundedSender<Message>;
pub(crate) type ResultSender = tokio::sync::oneshot::Sender<Result<RespResponse>>;
pub(crate) type ResultReceiver = tokio::sync::oneshot::Receiver<Result<RespResponse>>;
pub(crate) type ResultsSender = tokio::sync::oneshot::Sender<Result<Vec<RespResponse>>>;
pub(crate) type ResultsReceiver = tokio::sync::oneshot::Receiver<Result<Vec<RespResponse>>>;
/// Bounded, drop-oldest channels; see [`crate::client::bounded_channel`].
///
/// Pub/sub streams and the push sinks (client-side-caching invalidation,
/// `MONITOR`) share one implementation: all three deliver server-driven messages
/// to a consumer the network task must never wait on. What differs is what a
/// drop *means*, which is documented on the budgets in `BackpressureConfig`.
pub(crate) type PubSubSender = crate::client::BoundedSender;
pub(crate) type PubSubReceiver = crate::client::BoundedReceiver;
pub(crate) type PushSender = crate::client::BoundedSender;
pub(crate) type PushReceiver = crate::client::BoundedReceiver;
pub(crate) type ReconnectSender = broadcast::Sender<()>;
pub(crate) type ReconnectReceiver = broadcast::Receiver<()>;

/// Test-only observability and fault-injection hook for the send batch.
///
/// This is the queue-position primitive of the failure-path test
/// infrastructure: it lets a test deterministically force retry reasons onto a
/// message drained by [`NetworkHandler::send_messages`] and observe the retry
/// reasons every command is actually fed with. It carries no cost in shipped
/// builds because it is gated behind `cfg(test)`, so it is compiled only when
/// the crate itself is built as a test target, like the existing
/// `kill_connection_on_write` primitive.
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct SendBatchTestHook {
    /// Retry reasons to force onto the **first** message of each
    /// `send_messages` drain that contains at least one message. One entry is
    /// consumed per such drain; `None` leaves that drain untouched.
    inject_first_message_reasons: Arc<std::sync::Mutex<VecDeque<Option<Vec<RetryReason>>>>>,
    /// Records, in feed order, `(command name, number of retry reasons fed)`
    /// for every command actually fed to the connection.
    fed_retry_reasons: Arc<std::sync::Mutex<Vec<(String, usize)>>>,
    /// When set, the next fed command whose name matches is armed to kill the
    /// connection on its `usize`-th following read (see
    /// [`CommandBuilder::kill_connection_on_read`]). Consumed on first match,
    /// so it fires exactly once. Lets a test inject a send failure onto a
    /// command it does not build itself, such as the sink's internal
    /// `UNSUBSCRIBE`.
    kill_on_read_by_name: Arc<std::sync::Mutex<Option<(String, usize)>>>,
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-support code: a panic is how a test reports failure"
)]
impl SendBatchTestHook {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Queues retry reasons to be forced onto the first message of the next
    /// send drain (or `None` to skip that drain).
    pub(crate) fn push_injection(&self, reasons: Option<Vec<RetryReason>>) {
        self.inject_first_message_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .push_back(reasons);
    }

    /// Returns the recorded `(command name, number of retry reasons fed)`
    /// entries, in feed order.
    pub(crate) fn fed_retry_reasons(&self) -> Vec<(String, usize)> {
        self.fed_retry_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .clone()
    }

    fn take_injection(&self) -> Option<Vec<RetryReason>> {
        self.inject_first_message_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .pop_front()
            .flatten()
    }

    fn record_fed(&self, command_name: String, num_reasons: usize) {
        self.fed_retry_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .push((command_name, num_reasons));
    }

    /// Arms the connection to be killed on the `num_reads`-th read following the
    /// next fed command named `command_name`.
    pub(crate) fn arm_kill_on_read_for(&self, command_name: &str, num_reads: usize) {
        *self
            .kill_on_read_by_name
            .lock()
            .expect("send batch test hook mutex poisoned") =
            Some((command_name.to_owned(), num_reads));
    }

    /// If the next queued kill matches `command_name`, consumes it and returns
    /// the read count to arm.
    fn take_kill_on_read_for(&self, command_name: &str) -> Option<usize> {
        let mut guard = self
            .kill_on_read_by_name
            .lock()
            .expect("send batch test hook mutex poisoned");
        if guard.as_ref().is_some_and(|(name, _)| name == command_name) {
            return guard.take().map(|(_, num_reads)| num_reads);
        }
        None
    }
}

/// Test-only observability of how deep the network task's internal queues get
/// and of how much traffic the pub/sub and push sinks actually absorb.
///
/// Every counter is an `Arc<AtomicUsize>`, so reading one from a test never
/// contends with the network task and the send path never takes a lock. The
/// queue depths are recorded with `fetch_max`, which makes them **high-water
/// marks**: monotone, so a drain cannot lower them and a test can observe the
/// peak after the queue is empty again. A purge therefore shows up as the mark
/// *stopping*, never as the mark falling.
///
/// `futures_channel::mpsc::UnboundedReceiver` exposes no `len()`, so the
/// delivered/failed counters are the only way to observe how much a pub/sub
/// channel holds: when the consumer never polls its stream, `delivered` *is* the
/// channel depth.
///
/// Like [`SendBatchTestHook`], it is gated behind `cfg(test)` and carries no
/// cost in shipped builds.
#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct QueueMetricsTestHook {
    messages_to_send_high_water: Arc<std::sync::atomic::AtomicUsize>,
    messages_to_receive_high_water: Arc<std::sync::atomic::AtomicUsize>,
    pub_sub_delivered: Arc<std::sync::atomic::AtomicUsize>,
    pub_sub_delivery_failed: Arc<std::sync::atomic::AtomicUsize>,
    pub_sub_delivered_bytes: Arc<std::sync::atomic::AtomicUsize>,
    push_delivered: Arc<std::sync::atomic::AtomicUsize>,
    push_delivery_failed: Arc<std::sync::atomic::AtomicUsize>,
    push_delivered_bytes: Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl QueueMetricsTestHook {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Deepest `messages_to_send` ever observed, in messages.
    pub(crate) fn messages_to_send_high_water(&self) -> usize {
        self.messages_to_send_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Deepest `messages_to_receive` ever observed, in messages.
    pub(crate) fn messages_to_receive_high_water(&self) -> usize {
        self.messages_to_receive_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Pub/sub messages handed to a subscriber's channel without error.
    pub(crate) fn pub_sub_delivered(&self) -> usize {
        self.pub_sub_delivered
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Pub/sub messages the subscriber's channel refused, its receiver being gone.
    pub(crate) fn pub_sub_delivery_failed(&self) -> usize {
        self.pub_sub_delivery_failed
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Wire bytes of the pub/sub messages counted by [`Self::pub_sub_delivered`].
    /// This is the payload a paused subscriber's channel retains.
    pub(crate) fn pub_sub_delivered_bytes(&self) -> usize {
        self.pub_sub_delivered_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Push messages — monitor lines and tracking invalidations — handed to their
    /// sink's channel without error.
    ///
    /// This is the denominator a shedding assertion needs: `dropped_messages()`
    /// on the stream says how many were evicted, and only this says how many were
    /// offered in the first place.
    pub(crate) fn push_delivered(&self) -> usize {
        self.push_delivered
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Push messages the sink's channel refused, its receiver being gone.
    pub(crate) fn push_delivery_failed(&self) -> usize {
        self.push_delivery_failed
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Wire bytes of the push messages counted by [`Self::push_delivered`]. This
    /// is the payload a paused sink's channel retains.
    pub(crate) fn push_delivered_bytes(&self) -> usize {
        self.push_delivered_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    fn record_queue_depths(&self, to_send: usize, to_receive: usize) {
        self.messages_to_send_high_water
            .fetch_max(to_send, std::sync::atomic::Ordering::Relaxed);
        self.messages_to_receive_high_water
            .fetch_max(to_receive, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_pub_sub_delivered(&self, bytes: usize) {
        self.pub_sub_delivered
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.pub_sub_delivered_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_pub_sub_delivery_failed(&self) {
        self.pub_sub_delivery_failed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_push_delivered(&self, bytes: usize) {
        self.push_delivered
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.push_delivered_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_push_delivery_failed(&self) {
        self.push_delivery_failed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

// Why `Config::max_messages_per_wave` exists, kept next to the code that obeys
// it in `try_handle_message`.
//
// Draining the message channel until it is empty convoys the entire in-flight
// concurrency into one `writev`, so every caller waits for the whole batch to be
// written *and* answered. Capping the wave keeps a batch in flight at the server
// while the next one is being collected.
//
// The default (48) was calibrated against a live Redis over concurrency levels
// 64 → 1024 (see `RUSTIS_VS_REDIS_RS.md`, H13): the optimum is flat between 32
// and 128, 48 is within ~12% of the per-level optimum everywhere, and below 48
// in-flight messages the cap never fires, so low-concurrency behaviour is
// unchanged whatever it is set to.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Disconnected,
    Connected,
    EnteringMonitor,
    Monitor,
    LeavingMonitor,
}

struct MessageToSend {
    pub message: Message,
}

impl MessageToSend {
    pub(crate) fn new(message: Message) -> Self {
        Self { message }
    }
}

#[derive(Debug)]
struct MessageToReceive {
    pub message: Message,
    pub num_commands: usize,
    pub pending_responses: Vec<RespResponse>,
}

impl MessageToReceive {
    pub(crate) fn new(message: Message, num_commands: usize) -> Self {
        Self {
            message,
            num_commands,
            // A batch collects exactly `num_commands` responses; size the buffer
            // once instead of letting it grow.
            pending_responses: Vec::with_capacity(num_commands),
        }
    }
}

struct PendingSubscription {
    pub channel_or_pattern: Bytes,
    pub subscription_type: SubscriptionType,
    pub sender: PubSubSender,
    /// indicates if more subscriptions will arrive in the same batch
    pub more_to_come: bool,
}

pub(crate) struct NetworkHandler {
    status: Status,
    connection: Connection,
    /// for retries
    msg_sender: WeakMsgSender,
    msg_receiver: MsgReceiver,
    messages_to_send: VecDeque<MessageToSend>,
    messages_to_receive: VecDeque<MessageToReceive>,
    pending_subscriptions: VecDeque<PendingSubscription>,
    pending_unsubscriptions: VecDeque<HashMap<Bytes, SubscriptionType>>,
    subscriptions: HashMap<Bytes, (SubscriptionType, PubSubSender)>,
    /// Subscriptions whose subscriber is gone, collected while a push is being
    /// routed and unsubscribed from at the end of the read wave.
    ///
    /// Delivery is matched on the synchronous read path, but sending the
    /// UNSUBSCRIBE needs the async send path, so the two are separated.
    orphaned_subscriptions: Vec<(Bytes, SubscriptionType)>,
    is_reply_on: bool,
    /// `CLIENT REPLY SKIP` silences the reply of the command that follows it, and
    /// only that one — unlike `OFF`, which silences the connection until `ON`.
    skip_next_reply: bool,
    /// Connection-attached state to replay when the socket is remade. Owned here
    /// and lent as `&mut` to whichever connection is being built: the network task
    /// is its only user, so no `Arc` and no lock are involved.
    connection_state: ConnectionState,
    /// Sink for client-side-caching invalidation pushes, active while the
    /// connection is in `Status::Connected`. Kept separate from `monitor_sender`
    /// so registering one push consumer cannot silently overwrite the other's
    /// slot — the two flows are routed by distinct `Status` states, so a single
    /// shared field was only ever a latent trap, not a working multiplexer.
    invalidation_sender: Option<PushSender>,
    /// Sink for MONITOR output, active while the connection is in
    /// `Status::Monitor` / `LeavingMonitor`. See `invalidation_sender`.
    monitor_sender: Option<PushSender>,
    reconnect_sender: ReconnectSender,
    auto_resubscribe: bool,
    auto_remonitor: bool,
    reconnection_state: ReconnectionState,
    /// Per-message retry cap from `Config::max_command_attempts` (`0` = unlimited).
    max_command_attempts: usize,
    /// Send-wave cap from `Config::max_messages_per_wave`.
    max_messages_per_wave: usize,
    /// Memory budget for `messages_to_send`, from
    /// `Config::backpressure.max_queued_bytes` (`0` = unlimited).
    max_queued_bytes: usize,
    /// Running total of `Message::queued_bytes` over `messages_to_send`.
    ///
    /// Maintained incrementally at every push and pop rather than recomputed:
    /// the queue is walked often enough that summing it per message would be
    /// quadratic in the queue depth.
    queued_bytes: usize,
    /// Number of incoming results belonging to a message that has already been
    /// resolved, and which must therefore be dropped instead of matched.
    results_to_discard: usize,
    #[cfg(test)]
    send_batch_test_hook: Option<SendBatchTestHook>,
    #[cfg(test)]
    queue_metrics_test_hook: Option<QueueMetricsTestHook>,
}

impl NetworkHandler {
    pub(crate) async fn connect(
        config: Config,
    ) -> Result<(MsgSender, JoinHandle<()>, ReconnectSender, Arc<str>)> {
        // Reject an incoherent config here rather than letting a zeroed knob
        // surface later as a stall or a rejected reply.
        config.validate()?;

        // options
        let auto_resubscribe = config.auto_resubscribe;
        let auto_remonitor = config.auto_remonitor;
        let max_command_attempts = config.max_command_attempts;
        let max_messages_per_wave = config.max_messages_per_wave;
        let max_queued_bytes = config.backpressure.max_queued_bytes;
        let reconnection_config = config.reconnection.clone();
        #[cfg(test)]
        let send_batch_test_hook = config.send_batch_test_hook.clone();
        #[cfg(test)]
        let queue_metrics_test_hook = config.queue_metrics_test_hook.clone();

        // One registry per client: two clients built from the same `Config` must
        // not share the state either of them sets at runtime, which is exactly
        // why this is lent to the connection rather than carried by the config.
        let mut connection_state = ConnectionState::default();

        let connection = Connection::connect(config, &mut connection_state).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) =
            tokio::sync::mpsc::unbounded_channel();
        let (reconnect_sender, _): (ReconnectSender, ReconnectReceiver) = broadcast::channel(32);
        let tag = connection.tag().to_owned();

        let mut network_handler = NetworkHandler {
            status: Status::Connected,
            connection,
            msg_sender: msg_sender.downgrade(),
            msg_receiver,
            messages_to_send: VecDeque::new(),
            messages_to_receive: VecDeque::new(),
            pending_subscriptions: VecDeque::new(),
            pending_unsubscriptions: VecDeque::new(),
            subscriptions: HashMap::new(),
            orphaned_subscriptions: Vec::new(),
            is_reply_on: true,
            skip_next_reply: false,
            connection_state,
            invalidation_sender: None,
            monitor_sender: None,
            reconnect_sender: reconnect_sender.clone(),
            auto_resubscribe,
            auto_remonitor,
            reconnection_state: ReconnectionState::new(reconnection_config),
            max_command_attempts,
            max_messages_per_wave,
            max_queued_bytes,
            queued_bytes: 0,
            results_to_discard: 0,
            #[cfg(test)]
            send_batch_test_hook,
            #[cfg(test)]
            queue_metrics_test_hook,
        };

        // Every event emitted by the network task, and by the connection code it
        // calls into, inherits this span. That is what carries the connection
        // identity, so no message below has to spell it out.
        let span = info_span!("connection", tag = %tag);

        let join_handle = spawn(
            async move {
                if let Err(e) = network_handler.network_loop().await {
                    error!("network loop ended in error: {e}");
                }
            }
            .instrument(span),
        );

        Ok((msg_sender, join_handle, reconnect_sender, tag))
    }

    async fn network_loop(&mut self) -> Result<()> {
        loop {
            select! {
                msg = poll_fn(|cx| self.msg_receiver.poll_recv(cx)).fuse() => {
                    if !self.try_handle_message(msg).await { break; }
                },
                result = self.connection.read().fuse() => {
                    if !self.try_handle_result(result).await { break; }
                }
            }
        }

        debug!("end of network loop");
        Ok(())
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "one increment per message taken from the channel in this wave, \
                  and the wave is cut at `max_messages_per_wave` below."
    )]
    async fn try_handle_message(&mut self, mut msg: Option<Message>) -> bool {
        let mut is_channel_closed = false;
        // Messages queued since the last flush, for the wave cap below.
        let mut queued: usize = 0;

        loop {
            if let Some(msg) = msg {
                self.handle_message(msg);
                queued += 1;
            } else {
                is_channel_closed = true;
                break;
            }

            // Send in waves rather than accumulating the whole channel into
            // one write (see `Config::max_messages_per_wave`).
            if queued >= self.max_messages_per_wave {
                if self.status != Status::Disconnected {
                    self.send_messages().await;
                }
                queued = 0;
            }

            match self.msg_receiver.try_recv() {
                Ok(m) => msg = Some(m),
                Err(_) => {
                    // there are no messages available, but channel is not yet closed
                    break;
                }
            }
        }

        if self.status != Status::Disconnected {
            self.send_messages().await
        }

        !is_channel_closed
    }

    /// Test-only: samples the current queue depths into the metrics hook.
    ///
    /// Called wherever a depth can be at its peak — right after the pushes, and
    /// right before a drain or a purge rebuilds the queue.
    #[cfg(test)]
    fn record_queue_depths(&self) {
        if let Some(hook) = &self.queue_metrics_test_hook {
            hook.record_queue_depths(self.messages_to_send.len(), self.messages_to_receive.len());
        }
    }

    /// Whether queuing `cost` more bytes would breach the send-queue budget.
    ///
    /// An empty queue always admits, whatever the size: refusing a command
    /// larger than the whole budget would make it permanently unsendable rather
    /// than merely delayed. The queue is therefore bounded by the budget plus at
    /// most one message.
    fn would_exceed_queue_budget(&self, cost: usize) -> bool {
        self.max_queued_bytes != 0
            && self.queued_bytes != 0
            // A saturated sum is still above any budget, so saturating here gives
            // the same answer without an overflow to reason about.
            && self.queued_bytes.saturating_add(cost) > self.max_queued_bytes
    }

    /// Queues a message for sending, keeping the byte accounting in step.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the running total counts bytes of buffers that are actually \
                  allocated and still queued, so it is bounded by the memory \
                  holding them. Saturating instead would silently desynchronise \
                  the backpressure accounting from what is really queued."
    )]
    fn queue_message(&mut self, msg: Message) {
        self.queued_bytes += msg.queued_bytes();
        self.messages_to_send.push_back(MessageToSend::new(msg));
    }

    fn handle_message(&mut self, mut msg: Message) {
        trace!("[{:?}] Will handle message: {msg:?}", self.status);

        // Shed an incoming command rather than let the send queue grow past its
        // memory budget, which is what a reconnection outage would otherwise do.
        //
        // Only a *new* message is refused. A replayed or retried one has already
        // been charged an attempt, so `attempts > 0` marks a command the caller
        // was told had been accepted; dropping it here would lose it silently.
        // An invalidation registers a sink instead of queuing a command, so it
        // costs the queue nothing and is never refused. Neither is a message
        // that is about to be failed with `DisconnectedByPeer` for opting out of
        // retries: it never reaches the queue, so blaming the queue for it would
        // report the wrong cause.
        let will_be_queued = self.status != Status::Disconnected || msg.retry_on_error;
        if will_be_queued
            && msg.attempts == 0
            && !matches!(msg.kind, MessageKind::Invalidation { .. })
            && self.would_exceed_queue_budget(msg.queued_bytes())
        {
            debug!(
                "send queue is full ({} bytes), shedding command: {:?}",
                self.queued_bytes,
                msg.commands()
            );
            msg.send_error(Error::from(ClientError::SendQueueFull));
            return;
        }

        let mut collision_error = None;

        match &self.status {
            Status::Connected => {
                match &mut msg.kind {
                    MessageKind::PubSub {
                        subscription_type,
                        subscriptions,
                        ..
                    } => {
                        for (channel_or_pattern, _sender) in subscriptions.iter() {
                            if self.subscriptions.contains_key(channel_or_pattern) {
                                debug!(
                                    "[{:?}] There is already a subscription on channel `{}`",
                                    self.status,
                                    String::from_utf8_lossy(channel_or_pattern)
                                );
                                collision_error = Some(Error::from(ClientError::AlreadySubscribed));
                                break;
                            }
                        }

                        if collision_error.is_none() {
                            let subscriptions = std::mem::take(subscriptions);
                            // The closure below never runs on an empty set, so
                            // the saturated value is unreachable rather than wrong.
                            let last_subscription_index = subscriptions.len().saturating_sub(1);
                            let pending_subscriptions = subscriptions.into_iter().enumerate().map(
                                |(index, (channel_or_pattern, sender))| PendingSubscription {
                                    channel_or_pattern,
                                    subscription_type: *subscription_type,
                                    sender,
                                    more_to_come: index < last_subscription_index,
                                },
                            );

                            self.pending_subscriptions.extend(pending_subscriptions);
                        }
                    }
                    MessageKind::Monitor { push_sender, .. } => {
                        self.status = Status::EnteringMonitor;
                        let push_sender = push_sender.take();
                        if let Some(push_sender) = push_sender {
                            debug!("Registering MONITOR push_sender");
                            self.monitor_sender = Some(push_sender);
                        }
                    }
                    MessageKind::Invalidation { push_sender } => {
                        let push_sender = push_sender.take();
                        if let Some(push_sender) = push_sender {
                            debug!("Registering Invalidation push_sender");
                            self.invalidation_sender = Some(push_sender);
                        }
                        return; // no message to send
                    }
                    MessageKind::Single { command, .. } => {
                        if let CommandKind::Unsbuscribe(subscription_type) = command.kind() {
                            self.pending_unsubscriptions.push_back(
                                command.args().map(|a| (a, *subscription_type)).collect(),
                            );
                        }
                    }

                    _ => (),
                }

                if let Some(err) = collision_error {
                    msg.send_error(err);
                } else {
                    self.queue_message(msg);
                }
            }
            Status::Disconnected => {
                if msg.retry_on_error {
                    debug!(
                        "network disconnected, queuing command: {:?}",
                        msg.commands()
                    );
                    self.queue_message(msg);
                } else {
                    debug!(
                        "network disconnected, sending command in error: {:?}",
                        msg.commands()
                    );
                    msg.send_error(Error::from(ErrorKind::DisconnectedByPeer));
                }
            }
            Status::EnteringMonitor => self.queue_message(msg),
            Status::Monitor => {
                for command in msg.commands() {
                    if matches!(command.kind(), CommandKind::Reset) {
                        self.status = Status::LeavingMonitor;
                    }
                }
                self.queue_message(msg);
            }
            Status::LeavingMonitor => {
                self.queue_message(msg);
            }
        }

        #[cfg(test)]
        self.record_queue_depths();
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "both counters add one per command in the send queue: the queue is \
                  bounded by `max_messages_per_wave` and each command owns an \
                  allocated buffer, so neither total can approach `usize::MAX`."
    )]
    async fn send_messages(&mut self) {
        // Sampled before the drain, where the send queue is at its deepest.
        #[cfg(test)]
        self.record_queue_depths();

        // The line is only worth emitting for an actual batch, and deciding that
        // needs the count, so the count is taken here rather than inside the
        // macro argument. Guarding it with `enabled!` instead would silence the
        // line for every `log`-only consumer, which the bridge exists to serve.
        //
        // The walk is bounded by `max_messages_per_wave` and each step is a
        // discriminant read; the loop below iterates the same queue and encodes
        // every command in it.
        if !self.messages_to_send.is_empty() {
            let num_commands = self
                .messages_to_send
                .iter()
                .fold(0, |sum, msg| sum + msg.message.num_commands());
            if num_commands > 1 {
                debug!("sending batch of {num_commands} commands");
            }
        }

        // Test-only: force retry reasons onto the first message of this drain so
        // a test can reproduce a redirected message ahead of unrelated ones.
        #[cfg(test)]
        if let Some(hook) = &self.send_batch_test_hook
            && !self.messages_to_send.is_empty()
            && let Some(reasons) = hook.take_injection()
            && let Some(front) = self.messages_to_send.front_mut()
        {
            front.message.retry_reasons = Some(reasons);
        }

        let start_idx = self.messages_to_receive.len();

        while let Some(message_to_send) = self.messages_to_send.pop_front() {
            let mut msg = message_to_send.message;
            self.queued_bytes = self.queued_bytes.saturating_sub(msg.queued_bytes());

            // Scope the retry reasons to the current message: they must not
            // leak onto the other messages sharing this send batch.
            let mut retry_reasons = SmallVec::<[RetryReason; 10]>::new();
            let reasons = msg.retry_reasons.take();
            if let Some(reasons) = reasons {
                retry_reasons.extend(reasons);
            }

            let mut num_commands_to_receive: usize = 0;

            // Commands are fed one by one on purpose. Batching them into a single
            // call (to hoist the stream-variant `match` out of the loop and issue
            // one pre-computed reserve) was implemented and measured against a live
            // Redis: no change (long pipeline +1.3%, p=0.53). The 8 KiB write-buffer
            // flush (see `CommandEncoder::encode`) already caps the buffer, so there
            // is nothing to amortize. Keep the per-command loop.
            for command in msg.commands_mut() {
                let kind = *command.kind();

                match kind {
                    CommandKind::ClientReply(ClientReplyMode::On) => {
                        self.is_reply_on = true;
                        self.skip_next_reply = false;
                        self.connection_state.record(StateSlot::ReplyMode, command);
                    }
                    CommandKind::ClientReply(ClientReplyMode::Off) => {
                        self.is_reply_on = false;
                        self.skip_next_reply = false;
                        self.connection_state.record(StateSlot::ReplyMode, command);
                    }
                    // `SKIP` is not connection state: it is consumed by the next
                    // command and leaves the connection as it found it.
                    CommandKind::ClientReply(ClientReplyMode::Skip) => {
                        self.skip_next_reply = true;
                    }
                    CommandKind::ConnectionState(slot) => {
                        self.connection_state.record(slot, command);
                    }
                    // The server restores every connection default here, so the
                    // client's picture of the connection must go with it —
                    // including the reply mode, which `RESET` itself answers
                    // through.
                    CommandKind::Reset => {
                        self.connection_state.clear();
                        self.is_reply_on = true;
                        self.skip_next_reply = false;
                        self.subscriptions.clear();
                    }
                    _ => (),
                }

                // The registry just changed, so the copy a cluster connection
                // replays onto a joining node has to change with it. This is the
                // only place connection state is recorded, which makes it the only
                // sync point needed.
                if matches!(
                    kind,
                    CommandKind::ConnectionState(_)
                        | CommandKind::ClientReply(_)
                        | CommandKind::Reset
                ) {
                    self.connection
                        .sync_connection_state(&self.connection_state);
                }

                let expects_reply = if !self.is_reply_on {
                    false
                } else if matches!(kind, CommandKind::ClientReply(ClientReplyMode::Skip)) {
                    // `CLIENT REPLY SKIP` is not answered either.
                    false
                } else if self.skip_next_reply {
                    self.skip_next_reply = false;
                    false
                } else {
                    true
                };

                if expects_reply {
                    num_commands_to_receive += 1;
                }

                // Test-only: record the retry reasons this command is fed with,
                // so a test can assert reasons do not leak across messages.
                #[cfg(test)]
                if let Some(hook) = &self.send_batch_test_hook {
                    let command_name = String::from_utf8_lossy(command.name()).into_owned();
                    hook.record_fed(command_name.clone(), retry_reasons.len());

                    // Arm a read-side kill onto this command if a test queued one
                    // for its name, reusing the per-command countdown so the
                    // existing `feed` path picks it up.
                    if let Some(num_reads) = hook.take_kill_on_read_for(&command_name) {
                        command
                            .kill_connection_on_read
                            .store(num_reads, std::sync::atomic::Ordering::SeqCst);
                    }
                }

                if let Err(e) = self.connection.feed(command, &retry_reasons).await {
                    error!("Feed error: {e}");
                    msg.send_error(e);
                    return;
                }
            }

            if num_commands_to_receive > 0 {
                self.messages_to_receive
                    .push_back(MessageToReceive::new(msg, num_commands_to_receive));
            }
        }

        if let Err(e) = self.connection.flush().await {
            error!("Flush error: {e}");

            while self.messages_to_receive.len() > start_idx {
                if let Some(msg_to_receive) = self.messages_to_receive.pop_back() {
                    msg_to_receive.message.send_error(e.clone());
                }
            }
        }
    }

    async fn try_handle_result(&mut self, result: Option<Result<RespResponse>>) -> bool {
        let Some(result) = result else {
            return self.reconnect().await;
        };
        // A protocol decode error desynchronizes the stream; attributing it to the
        // head-of-queue message blames an innocent caller. Reconnect instead, which
        // resynchronizes the stream and purges/replays in-flight messages cleanly.
        if let Err(e) = &result
            && is_connection_level_error(e)
        {
            debug!("Connection-level read error, reconnecting: {e}");
            return self.reconnect().await;
        }
        // The demotion signal is read before the result is handed over, because
        // `handle_result` consumes it. The caller still receives the `READONLY`
        // itself — replacing it with the reconnection's `DisconnectedByPeer` would
        // hide why the write was refused — and the rediscovery happens after the
        // whole batch has been dispatched, so a burst of refused writes costs one
        // reconnection rather than one each.
        let mut master_demoted = self.master_demoted(&result);
        self.handle_result(result);

        // OPTIMIZATION : Drain the next available results in the buffer
        while let Poll::Ready(result) = self.connection.try_read() {
            let Some(result) = result else {
                return self.reconnect().await;
            };
            if let Err(e) = &result
                && is_connection_level_error(e)
            {
                debug!("Connection-level read error, reconnecting: {e}");
                return self.reconnect().await;
            }
            master_demoted |= self.master_demoted(&result);
            self.handle_result(result);
        }

        if master_demoted {
            debug!("The master was demoted to replica, rediscovering it");
            return self.reconnect().await;
        }

        // Nothing else flushes the send queue on this path: an UNSUBSCRIBE left
        // queued here would wait for the next command the application happens
        // to send, which on a pure subscriber never comes.
        self.unsubscribe_orphaned_subscriptions().await;

        true
    }

    /// Whether this result says the master is now a replica *and* a reconnection
    /// would find the new one — the two halves that together make rediscovery worth
    /// it. Where reconnecting cannot look the master up again, a `READONLY` is left
    /// as the per-message error it is, rather than churning the connection to come
    /// back to the same demoted node.
    fn master_demoted(&self, result: &Result<RespResponse>) -> bool {
        indicates_demoted_master(result) && self.connection.rediscovers_master_on_reconnect()
    }

    /// Hands a matched reply to its caller, waking it.
    ///
    /// Called from [`Self::receive_result`] the moment the reply is matched,
    /// before the next ready reply is parsed: on a multi-thread runtime another
    /// worker resumes the caller in parallel while this task keeps draining,
    /// which shortens first-reply latency on the critical path.
    /// `command` names what the reply answers, so an abandoned one can be traced
    /// back on a multiplexed connection where hundreds are in flight.
    fn dispatch_result<T>(
        &self,
        sender: tokio::sync::oneshot::Sender<T>,
        value: T,
        command: Option<&Bytes>,
    ) {
        if sender.send(value).is_err() {
            // A caller that gave up on its reply is the documented contract of a
            // `command_timeout` or a dropped future, not a fault. At `warn!` a
            // service with deadlines would flood its logs exactly when Redis is
            // slow, which is when the log is read.
            let command = command.map_or_else(
                || Cow::Borrowed("<none>"),
                |name| String::from_utf8_lossy(name),
            );
            debug!("Dropping the reply to {command}: its receiver is gone");
        }
    }

    fn handle_result(&mut self, result: Result<RespResponse>) {
        match self.status {
            Status::Disconnected => (),
            Status::Connected => match &result {
                Ok(response) if response.is_push() => {
                    if let Some(response) = self.try_match_pubsub_message(result) {
                        if response.is_err() {
                            self.receive_result(response);
                        } else {
                            match &mut self.invalidation_sender {
                                Some(push_sender) => {
                                    #[cfg(test)]
                                    let delivered_bytes = response
                                        .as_ref()
                                        .map(|response| response.retained_bytes())
                                        .unwrap_or(0);
                                    let sent = push_sender.send(response);
                                    #[cfg(test)]
                                    if let Some(hook) = &self.queue_metrics_test_hook {
                                        if sent.is_ok() {
                                            hook.record_push_delivered(delivered_bytes);
                                        } else {
                                            hook.record_push_delivery_failed();
                                        }
                                    }
                                    if let Err(e) = sent {
                                        warn!("Cannot send push message result to caller: {e}");
                                    }
                                }
                                None => {
                                    warn!(
                                        "Received a push message with no sender configured: {response:?}"
                                    )
                                }
                            }
                        }
                    }
                }
                _ => {
                    self.receive_result(result);
                }
            },
            Status::EnteringMonitor => {
                self.receive_result(result);
                self.status = Status::Monitor;
            }
            Status::Monitor => match &result {
                Ok(response) if response.is_monitor() => {
                    self.deliver_monitor_result(result);
                }
                _ => self.receive_result(result),
            },
            Status::LeavingMonitor => match &result {
                Ok(response) if response.is_monitor() => {
                    self.deliver_monitor_result(result);
                }
                _ => {
                    self.receive_result(result);
                    self.status = Status::Connected;
                }
            },
        }
    }

    /// Hands a monitor line to the `MONITOR` sink, if one is registered.
    fn deliver_monitor_result(&self, result: Result<RespResponse>) {
        #[cfg(test)]
        let delivered_bytes = result
            .as_ref()
            .map(|response| response.retained_bytes())
            .unwrap_or(0);

        let Some(push_sender) = &self.monitor_sender else {
            return;
        };

        let sent = push_sender.send(result);
        #[cfg(test)]
        if let Some(hook) = &self.queue_metrics_test_hook {
            if sent.is_ok() {
                hook.record_push_delivered(delivered_bytes);
            } else {
                hook.record_push_delivery_failed();
            }
        }
        if let Err(e) = sent {
            warn!("Cannot send monitor result to caller: {e}");
        }
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "each subtraction has its guard on the branch above it: \
                  `results_to_discard` is only decremented inside `> 0`, and \
                  `num_commands` is only decremented in the arm where it is neither \
                  1 nor being resolved — and it never starts at 0, because \
                  `send_messages` only enqueues a message to receive when it wrote \
                  at least one command expecting a reply. The retry counter is \
                  compared against `max_command_attempts` on the next line."
    )]
    fn receive_result(&mut self, result: Result<RespResponse>) {
        // Responses owed to a message that was already resolved as a whole: the
        // commands were executed, so their replies still arrive, but there is no
        // caller left for them. Matching them would shift every subsequent
        // response by one.
        if self.results_to_discard > 0 {
            self.results_to_discard -= 1;
            debug!("discarding response of an already resolved message: {result:?}");
            return;
        }

        match self.messages_to_receive.front_mut() {
            Some(message_to_receive) => {
                trace!("message_to_receive: {:?}", message_to_receive);

                if message_to_receive.num_commands == 1 || result.is_err() {
                    if let Some(mut message_to_receive) = self.messages_to_receive.pop_front() {
                        // A batch message is sent as several independent
                        // commands, each awaiting its own response. Resolving
                        // the whole message on the error of one of them leaves
                        // the commands queued behind it without a caller, while
                        // their replies are already on their way.
                        if message_to_receive.num_commands > 1 {
                            self.results_to_discard += message_to_receive.num_commands - 1;
                        }

                        let mut should_retry = false;

                        if let Err(e) = &result
                            && matches!(e.kind(), ErrorKind::Retry(_))
                        {
                            should_retry = true;
                        } else if message_to_receive.message.retry_reasons.is_some() {
                            should_retry = true;
                        }

                        if should_retry {
                            if let Err(ErrorKind::Retry(reasons)) = result.map_err(Error::into_kind)
                            {
                                if let Some(retry_reasons) =
                                    &mut message_to_receive.message.retry_reasons
                                {
                                    retry_reasons.extend(reasons);
                                } else {
                                    message_to_receive.message.retry_reasons =
                                        Some(Vec::from_iter(reasons));
                                }
                            }

                            // Bound message-level retries: a command caught in a
                            // pathological redirect loop would otherwise be replayed
                            // forever. Count this attempt and fail the message with a
                            // distinct error once the cap is reached.
                            message_to_receive.message.attempts += 1;
                            if max_attempts_reached(
                                message_to_receive.message.attempts,
                                self.max_command_attempts,
                            ) {
                                debug!(
                                    "Message reached the maximum number of attempts, failing it"
                                );
                                message_to_receive.message.send_error(Error::from(
                                    ClientError::MaxCommandAttemptsReached,
                                ));
                            }
                            // retry: upgrade the weak handle just long enough to
                            // requeue the message. A failed upgrade means every
                            // client is gone and the channel is closing, so the
                            // retry is moot.
                            else if let Some(msg_sender) = self.msg_sender.upgrade() {
                                if let Err(e) = msg_sender.send(message_to_receive.message) {
                                    error!("Cannot retry message: {e}");
                                }
                            } else {
                                debug!("Cannot retry message: channel closed");
                            }
                        } else {
                            trace!("Will respond to: {:?}", message_to_receive.message);

                            // This path answers the caller directly instead of
                            // going through `Message::send_error`, so it names
                            // the command itself. It carries the server's own
                            // errors — a `WRONGTYPE`, a `NOPERM` — which are
                            // exactly the ones a caller cannot act on without
                            // knowing what drew them.
                            let command_name = message_to_receive.message.command_name();
                            let result = match (result, &command_name) {
                                (Err(e), Some(command)) => Err(e.with_command(command.clone())),
                                (result, _) => result,
                            };

                            match message_to_receive.message.kind {
                                MessageKind::Single {
                                    result_sender: Some(result_sender),
                                    ..
                                }
                                | MessageKind::PubSub { result_sender, .. }
                                | MessageKind::Monitor { result_sender, .. } => {
                                    self.dispatch_result(
                                        result_sender,
                                        result,
                                        command_name.as_ref(),
                                    );
                                }
                                MessageKind::Batch { results_sender, .. } => match result {
                                    Ok(resp_buf) => {
                                        message_to_receive.pending_responses.push(resp_buf);
                                        self.dispatch_result(
                                            results_sender,
                                            Ok(message_to_receive.pending_responses),
                                            command_name.as_ref(),
                                        );
                                    }
                                    Err(e) => {
                                        self.dispatch_result(
                                            results_sender,
                                            Err(e),
                                            command_name.as_ref(),
                                        );
                                    }
                                },
                                MessageKind::Invalidation { .. }
                                | MessageKind::Single {
                                    result_sender: None,
                                    ..
                                } => {
                                    debug!("forget value {result:?}")
                                    // fire & forget
                                }
                            }
                        }
                    }
                } else {
                    match result {
                        Ok(value) => {
                            message_to_receive.pending_responses.push(value);
                            message_to_receive.num_commands -= 1;
                        }
                        Err(e) => {
                            if let ErrorKind::Retry(reasons) = e.into_kind() {
                                if let Some(retry_reasons) =
                                    &mut message_to_receive.message.retry_reasons
                                {
                                    retry_reasons.extend(reasons);
                                } else {
                                    message_to_receive.message.retry_reasons =
                                        Some(Vec::from_iter(reasons));
                                }
                            }
                        }
                    }
                }
            }
            None => {
                // Disconnection errors legitimately end here (no message is left
                // to carry them). An `Ok` frame with an empty in-flight queue is
                // unexpected — a mis-routed push, a buggy server/proxy, or a
                // desynchronized stream — but a network loop must never panic on
                // wire input: that would kill the sole owner of the routing state
                // and permanently wedge the client with no reconnection. Drop the
                // stray frame and log it instead.
                if result.is_ok() {
                    warn!(
                        "Dropping an unexpected response with no message awaiting it: {result:?}"
                    );
                }
            }
        }
    }

    /// Records a subscription whose subscriber is gone, so the read wave ends by
    /// unsubscribing from it.
    ///
    /// A delivery fails only when the receiving half has been dropped, which is
    /// permanent: retrying it on the next message would never succeed. Leaving
    /// the entry in place instead keeps the server publishing to a channel
    /// nobody can receive on, for as long as the connection lives. Removing it
    /// here also makes this the *first* and only failed delivery for that
    /// channel, so one warning and one UNSUBSCRIBE are emitted, not one per
    /// message.
    fn orphan_subscription(&mut self, orphaned: Option<(Bytes, SubscriptionType)>) {
        let Some((channel_or_pattern, subscription_type)) = orphaned else {
            return;
        };
        self.subscriptions.remove(&channel_or_pattern);
        self.orphaned_subscriptions
            .push((channel_or_pattern, subscription_type));
    }

    /// Unsubscribes from every subscription whose subscriber turned out to be
    /// gone while this read wave was routed.
    ///
    /// Routed on the synchronous read path, sent here because sending is async.
    /// The commands go through [`Self::handle_message`] rather than straight
    /// into the send queue so that the pub/sub bookkeeping is built exactly as
    /// for a caller-issued UNSUBSCRIBE: without its `pending_unsubscriptions`
    /// entry the confirmation push would arrive with nothing to match and shift
    /// every later response by one. They are fire-and-forget, like the ones
    /// `PubSubSplitSink::drop` sends, since no caller is left to be answered.
    async fn unsubscribe_orphaned_subscriptions(&mut self) {
        if self.orphaned_subscriptions.is_empty() {
            return;
        }

        for (channel_or_pattern, subscription_type) in
            std::mem::take(&mut self.orphaned_subscriptions)
        {
            let PreparedCommand { mut command, .. } = match subscription_type {
                SubscriptionType::Channel => self.connection.unsubscribe(channel_or_pattern),
                SubscriptionType::Pattern => self.connection.punsubscribe(channel_or_pattern),
                SubscriptionType::ShardChannel => self.connection.sunsubscribe(channel_or_pattern),
            };
            // A command built here has never been through the caller thread,
            // where `Client` computes the slots. SUNSUBSCRIBE names a shard
            // channel as a key, so without this it routes on slot 0 — to
            // whichever node owns that slot rather than the one holding the
            // subscription, which answers it happily and changes nothing.
            if self.connection.is_cluster() {
                command.compute_slots();
            }
            self.handle_message(Message::single_forget(command, true));
        }

        if self.status != Status::Disconnected {
            self.send_messages().await;
        }
    }

    fn try_match_pubsub_message(
        &mut self,
        value: Result<RespResponse>,
    ) -> Option<Result<RespResponse>> {
        if let Ok(ref_value) = &value {
            if let Ok(pub_sub_message) = PubSubPush::try_from(ref_value) {
                match pub_sub_message {
                    PubSubPush::Message(channel_or_pattern, _)
                    | PubSubPush::SMessage(channel_or_pattern, _) => {
                        #[cfg(test)]
                        let delivered_bytes = ref_value.retained_bytes();
                        // The key is looked up alongside its value because
                        // sending consumes `value`, and with it the borrowed
                        // channel name: naming the channel in the log, and
                        // cleaning the subscription up, both need a name that
                        // outlives the send.
                        let orphaned = match self.subscriptions.get_key_value(channel_or_pattern) {
                            Some((key, (subscription_type, pub_sub_sender))) => {
                                let sent = pub_sub_sender.send(value);
                                #[cfg(test)]
                                if let Some(hook) = &self.queue_metrics_test_hook {
                                    if sent.is_ok() {
                                        hook.record_pub_sub_delivered(delivered_bytes);
                                    } else {
                                        hook.record_pub_sub_delivery_failed();
                                    }
                                }
                                match sent {
                                    Ok(()) => None,
                                    Err(e) => {
                                        warn!(
                                            "Cannot send pub/sub message to caller from channel `{}`: {e}",
                                            String::from_utf8_lossy(key)
                                        );
                                        Some((key.clone(), *subscription_type))
                                    }
                                }
                            }
                            None => {
                                error!(
                                    "Unexpected message on channel `{}`",
                                    String::from_utf8_lossy(channel_or_pattern)
                                );
                                None
                            }
                        };
                        self.orphan_subscription(orphaned);
                        None
                    }
                    PubSubPush::Subscribe(channel_or_pattern)
                    | PubSubPush::PSubscribe(channel_or_pattern)
                    | PubSubPush::SSubscribe(channel_or_pattern) => {
                        // Peek before popping: a mismatched confirmation must not
                        // consume (and silently drop) the pending subscriber. Only
                        // pop once we know the front entry is the one being confirmed.
                        let matches = self
                            .pending_subscriptions
                            .front()
                            .is_some_and(|p| p.channel_or_pattern == channel_or_pattern);
                        if matches && let Some(pending_sub) = self.pending_subscriptions.pop_front()
                        {
                            if self
                                .subscriptions
                                .insert(
                                    pending_sub.channel_or_pattern,
                                    (pending_sub.subscription_type, pending_sub.sender),
                                )
                                .is_some()
                            {
                                return Some(Err(Error::from(ClientError::AlreadySubscribed)));
                            }

                            if pending_sub.more_to_come {
                                return None;
                            }

                            self.receive_result(Ok(RespResponse::ok()));
                        } else {
                            error!(
                                "Unexpected subscription confirmation on channel `{}`",
                                String::from_utf8_lossy(channel_or_pattern)
                            );
                            // Surface the anomaly to the caller instead of reporting
                            // a spurious success; the pending entry is left intact.
                            self.receive_result(Err(Error::from(
                                ClientError::UnexpectedSubscriptionConfirmation,
                            )));
                        }
                        None
                    }
                    PubSubPush::Unsubscribe(channel_or_pattern)
                    | PubSubPush::PUnsubscribe(channel_or_pattern)
                    | PubSubPush::SUnsubscribe(channel_or_pattern) => {
                        self.subscriptions.remove(channel_or_pattern);
                        if let Some(remaining) = self.pending_unsubscriptions.front_mut() {
                            if remaining.len() > 1 {
                                if remaining.remove(channel_or_pattern).is_none() {
                                    error!(
                                        "Cannot find channel or pattern to remove: `{}`",
                                        String::from_utf8_lossy(channel_or_pattern)
                                    );
                                }
                                None
                            } else {
                                // last unsubscription notification received
                                let Some(mut remaining) = self.pending_unsubscriptions.pop_front()
                                else {
                                    error!(
                                        "Cannot find channel or pattern to remove: `{}`",
                                        String::from_utf8_lossy(channel_or_pattern)
                                    );
                                    return None;
                                };
                                if remaining.remove(channel_or_pattern).is_none() {
                                    error!(
                                        "Cannot find channel or pattern to remove: `{}`",
                                        String::from_utf8_lossy(channel_or_pattern)
                                    );
                                    return None;
                                }
                                self.receive_result(Ok(RespResponse::ok()));
                                None
                            }
                        } else {
                            Some(value)
                        }
                    }
                    PubSubPush::PMessage(pattern, channel, _) => {
                        #[cfg(test)]
                        let delivered_bytes = ref_value.retained_bytes();
                        let orphaned = match self.subscriptions.get_key_value(pattern) {
                            Some((key, (subscription_type, pub_sub_sender))) => {
                                let sent = pub_sub_sender.send(value);
                                #[cfg(test)]
                                if let Some(hook) = &self.queue_metrics_test_hook {
                                    if sent.is_ok() {
                                        hook.record_pub_sub_delivered(delivered_bytes);
                                    } else {
                                        hook.record_pub_sub_delivery_failed();
                                    }
                                }
                                match sent {
                                    Ok(()) => None,
                                    Err(e) => {
                                        warn!(
                                            "Cannot send pub/sub message to caller for pattern `{}`: {e}",
                                            String::from_utf8_lossy(key)
                                        );
                                        Some((key.clone(), *subscription_type))
                                    }
                                }
                            }
                            None => {
                                error!(
                                    "Unexpected message on channel `{}` for pattern `{}`",
                                    String::from_utf8_lossy(channel),
                                    String::from_utf8_lossy(pattern)
                                );
                                None
                            }
                        };
                        self.orphan_subscription(orphaned);
                        None
                    }
                }
            } else {
                Some(value)
            }
        } else {
            Some(value)
        }
    }

    /// Nested inside the connection span, so everything a reconnection does —
    /// purging in-flight messages, replaying subscriptions, failing what
    /// exhausted its retry budget — is grouped under one identifiable unit
    /// instead of being interleaved with ordinary traffic.
    #[tracing::instrument(name = "reconnect", skip_all)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the retry counter is compared against `max_command_attempts` \
                  immediately after each increment, and `queued_bytes` accounts \
                  bytes of buffers that are allocated and requeued — see \
                  `queue_message`."
    )]
    async fn reconnect(&mut self) -> bool {
        debug!("reconnecting...");
        let old_status = self.status;
        self.status = Status::Disconnected;

        // The responses we were waiting to discard died with the connection;
        // keeping the count would discard legitimate responses afterwards.
        self.results_to_discard = 0;

        // A `SKIP` waiting for the command it silences died with the connection too.
        self.skip_next_reply = false;

        // A fresh connection is subscribed to nothing, so an orphaned
        // subscription has already achieved what its UNSUBSCRIBE was for.
        self.orphaned_subscriptions.clear();

        // Purge every non-retryable message, wherever it sits in the queue,
        // and keep the retryable ones in order. A prefix-only purge would leave
        // a non-retryable message behind a retryable one, and it would then be
        // replayed on reconnect — double-executing a command whose caller
        // opted out of retries.
        // A reconnection replay is also a retry attempt: count it and fail a message
        // that has exhausted its budget instead of replaying it once more.
        let max_command_attempts = self.max_command_attempts;

        // Sampled before the purge, so a purge reads as the high-water mark
        // stopping rather than as a depth that fell.
        #[cfg(test)]
        self.record_queue_depths();

        let mut retained_to_receive = VecDeque::with_capacity(self.messages_to_receive.len());
        while let Some(mut message_to_receive) = self.messages_to_receive.pop_front() {
            if !message_to_receive.message.retry_on_error {
                message_to_receive
                    .message
                    .send_error(Error::from(ErrorKind::DisconnectedByPeer));
            } else {
                message_to_receive.message.attempts += 1;
                if max_attempts_reached(message_to_receive.message.attempts, max_command_attempts) {
                    message_to_receive
                        .message
                        .send_error(Error::from(ClientError::MaxCommandAttemptsReached));
                } else {
                    retained_to_receive.push_back(message_to_receive);
                }
            }
        }
        self.messages_to_receive = retained_to_receive;

        let mut retained_to_send = VecDeque::with_capacity(self.messages_to_send.len());
        // The queue is rebuilt below, so the byte total is rebuilt with it
        // rather than adjusted message by message.
        self.queued_bytes = 0;
        while let Some(mut message_to_send) = self.messages_to_send.pop_front() {
            if !message_to_send.message.retry_on_error {
                message_to_send
                    .message
                    .send_error(Error::from(ErrorKind::DisconnectedByPeer));
            } else {
                message_to_send.message.attempts += 1;
                if max_attempts_reached(message_to_send.message.attempts, max_command_attempts) {
                    message_to_send
                        .message
                        .send_error(Error::from(ClientError::MaxCommandAttemptsReached));
                } else {
                    self.queued_bytes += message_to_send.message.queued_bytes();
                    retained_to_send.push_back(message_to_send);
                }
            }
        }
        self.messages_to_send = retained_to_send;

        loop {
            if let Some(delay) = self.reconnection_state.next_delay() {
                debug!("Waiting {delay} ms before reconnection");

                // keep on receiving new message during the delay
                let start = Instant::now();
                // A pathologically large reconnection delay would overflow the
                // monotonic clock; cap it rather than panicking the network task.
                let end = start
                    .checked_add(Duration::from_millis(delay))
                    .or_else(|| start.checked_add(Duration::from_secs(3600)))
                    .unwrap_or(start);
                loop {
                    let delay = end.duration_since(Instant::now());
                    let result =
                        timeout(delay, poll_fn(|cx| self.msg_receiver.poll_recv(cx))).await;
                    if let Ok(msg) = result {
                        if !self.try_handle_message(msg).await {
                            return false;
                        }
                    } else {
                        // delay has expired
                        break;
                    }
                }
            } else {
                error!("Max reconnection attempts reached: the client is finished");
                while let Some(message_to_receive) = self.messages_to_receive.pop_front() {
                    message_to_receive
                        .message
                        .send_error(Error::from(ErrorKind::DisconnectedByPeer));
                }
                while let Some(message_to_send) = self.messages_to_send.pop_front() {
                    message_to_send
                        .message
                        .send_error(Error::from(ErrorKind::DisconnectedByPeer));
                }
                self.queued_bytes = 0;
                return false;
            }

            if let Err(e) = self.connection.reconnect(&mut self.connection_state).await {
                error!("Failed to reconnect: {e:?}");
                continue;
            }

            // The new connection was restored from the same registry, so the
            // mirror the send loop uses has to be derived from it rather than
            // left at whatever the dead connection ended on.
            self.is_reply_on = self.connection_state.is_reply_on();

            if self.auto_resubscribe
                && let Err(e) = self.auto_resubscribe().await
            {
                error!("Failed to reconnect: {e:?}");
                continue;
            }

            if self.auto_remonitor
                && let Err(e) = self.auto_remonitor(old_status).await
            {
                error!("Failed to reconnect: {e:?}");
                continue;
            }

            if let Err(e) = self.reconnect_sender.send(()) {
                debug!("Cannot send reconnect notification to clients: {e}");
            }

            // Restore the connection status before replaying in-flight
            // messages so that they are routed through `handle_message`,
            // exactly as fresh messages and the retry path are.
            if let Status::Monitor | Status::EnteringMonitor = old_status {
                if self.monitor_sender.is_some() {
                    self.status = Status::Monitor;
                } else {
                    self.status = Status::Connected;
                }
            } else {
                self.status = Status::Connected;
            }

            // Replay every in-flight message through `handle_message` rather
            // than pushing it straight into `messages_to_send`. This rebuilds
            // the pub/sub bookkeeping (`pending_subscriptions` /
            // `pending_unsubscriptions`) for the replayed messages exactly as
            // the retry path does. Bypassing it would replay, for instance, an
            // UNSUBSCRIBE without a matching `pending_unsubscriptions` entry:
            // its confirmation push would then go unmatched, the stale message
            // would keep its slot in the receive queue, and every subsequent
            // response would be shifted by one, permanently. Messages already
            // sent but awaiting a reply are replayed before the ones still
            // queued, preserving the original global send order.
            let to_replay: Vec<Message> = std::mem::take(&mut self.messages_to_receive)
                .into_iter()
                .map(|message_to_receive| message_to_receive.message)
                .chain(
                    std::mem::take(&mut self.messages_to_send)
                        .into_iter()
                        .map(|message_to_send| message_to_send.message),
                )
                .collect();
            // `messages_to_send` was emptied above and `handle_message` charges
            // every message it queues, so the total has to start from zero or
            // the replayed messages would be counted twice.
            self.queued_bytes = 0;
            for message in to_replay {
                self.handle_message(message);
            }

            self.send_messages().await;

            info!("reconnected!");
            self.reconnection_state.reset_attempts();
            return true;
        }
    }

    async fn auto_resubscribe(&mut self) -> Result<()> {
        // Drop every pending unsubscription first, emitting nothing. On a fresh
        // connection the server is subscribed to nothing, so a pending
        // unsubscription has already achieved its goal. Removing the channels
        // from `subscriptions` up front also prevents the resubscribe loop
        // below from restoring subscriptions the caller was in the middle of
        // cancelling.
        for map in self.pending_unsubscriptions.drain(..) {
            for channel_or_pattern in map.into_keys() {
                self.subscriptions.remove(&channel_or_pattern);
            }
        }

        if !self.subscriptions.is_empty() {
            for (channel_or_pattern, (subscription_type, _)) in &self.subscriptions {
                match subscription_type {
                    SubscriptionType::Channel => {
                        self.connection.subscribe(channel_or_pattern).await?;
                    }
                    SubscriptionType::Pattern => {
                        self.connection.psubscribe(channel_or_pattern).await?;
                    }
                    SubscriptionType::ShardChannel => {
                        self.connection.ssubscribe(channel_or_pattern).await?;
                    }
                }
            }
        }

        if !self.pending_subscriptions.is_empty() {
            for pending_sub in self.pending_subscriptions.drain(..) {
                match pending_sub.subscription_type {
                    SubscriptionType::Channel => {
                        self.connection
                            .subscribe(pending_sub.channel_or_pattern.clone())
                            .await?;
                    }
                    SubscriptionType::Pattern => {
                        self.connection
                            .psubscribe(pending_sub.channel_or_pattern.clone())
                            .await?;
                    }
                    SubscriptionType::ShardChannel => {
                        self.connection
                            .ssubscribe(pending_sub.channel_or_pattern.clone())
                            .await?;
                    }
                }

                self.subscriptions.insert(
                    pending_sub.channel_or_pattern,
                    (pending_sub.subscription_type, pending_sub.sender),
                );
            }
        }

        Ok(())
    }

    async fn auto_remonitor(&mut self, old_status: Status) -> Result<()> {
        if let Status::Monitor | Status::EnteringMonitor = old_status {
            self.connection.send(&cmd("MONITOR").into()).await?;
        }

        Ok(())
    }
}

/// Whether an error surfaced by `connection.read()` is a connection-level failure
/// (a protocol decode error, a transport/IO error, or end of stream) rather than a
/// per-message one.
///
/// A decode error desynchronizes the byte stream, so it belongs to the connection,
/// not to whichever caller happens to sit at the head of the receive queue; it must
/// trigger a reconnect (clean purge + replay) instead of being dispatched as that
/// caller's result. Per-message errors that legitimately arrive here — a cluster
/// `ErrorKind::Retry` (ASK/MOVED) and a `ErrorKind::Redis` command error from a failing
/// shard — must be delivered to the caller, so this is a positive allow-list of the
/// framing/transport errors: anything unlisted is treated as per-message, which is
/// the safe default (a stray error reaches one caller instead of churning the whole
/// connection).
#[inline]
fn is_connection_level_error(error: &Error) -> bool {
    match error.kind() {
        ErrorKind::IO(_) | ErrorKind::EOF => true,
        ErrorKind::Client(client_error) => client_error.is_framing_error(),
        _ => false,
    }
}

/// Whether this reply says the node being talked to is no longer the master:
/// `-READONLY` is what a master demoted to replica answers to a write, on a
/// connection the server does not close.
///
/// It stays a per-message error — the caller who issued the write receives it — but
/// on a Sentinel connection it is also the only signal that the topology moved, and
/// so the trigger for rediscovering the master through the sentinels.
///
/// A command error arrives here as a successfully read error *frame*, not as an
/// `Err`: the `ErrorKind::Redis` only exists once a caller deserializes it. The tag
/// check comes first, so an ordinary reply never pays for a view.
#[inline]
fn indicates_demoted_master(result: &Result<RespResponse>) -> bool {
    match result {
        Ok(response) => {
            response.is_error()
                && matches!(response.view(), Ok(RespView::Error(message))
                    if matches!(RedisError::try_from(message),
                        Ok(error) if error.kind == RedisErrorKind::Readonly))
        }
        Err(e) => {
            matches!(e.kind(), ErrorKind::Redis(error) if error.kind == RedisErrorKind::Readonly)
        }
    }
}

/// Whether a message that has been attempted `attempts` times has reached the
/// configured per-message cap. `cap == 0` means unlimited (the default), matching
/// the historical behavior of never bounding retries at the message level.
#[inline]
fn max_attempts_reached(attempts: usize, cap: usize) -> bool {
    cap != 0 && attempts >= cap
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{indicates_demoted_master, is_connection_level_error, max_attempts_reached};

    #[test]
    fn zero_cap_is_unlimited() {
        assert!(!max_attempts_reached(1, 0));
        assert!(!max_attempts_reached(1_000_000, 0));
    }

    #[test]
    fn cap_reached_at_or_above_limit() {
        assert!(!max_attempts_reached(2, 3));
        assert!(max_attempts_reached(3, 3));
        assert!(max_attempts_reached(4, 3));
    }
    use crate::{ClientError, Error, ErrorKind, RedisError, RedisErrorKind};

    #[test]
    fn per_message_errors_are_not_connection_level() {
        // Cluster redirection and a failing-shard Redis error must reach the
        // caller, not tear down the connection.
        assert!(!is_connection_level_error(&Error::from(ErrorKind::Retry(
            Default::default()
        ))));
        assert!(!is_connection_level_error(&Error::from(ErrorKind::Redis(
            RedisError {
                kind: RedisErrorKind::NoPerm,
                description: "no permission".to_owned(),
            }
        ))));
        // A caller-side client error is not a stream desync either.
        assert!(!is_connection_level_error(&Error::from(
            ClientError::CrossSlot
        )));
    }

    #[test]
    fn decode_and_transport_errors_are_connection_level() {
        assert!(is_connection_level_error(&Error::from(
            ClientError::CannotParseInteger
        )));
        assert!(is_connection_level_error(&Error::from(
            ClientError::UnknownRespTag('?')
        )));
        assert!(is_connection_level_error(&Error::from(
            ClientError::MaxNestingDepthExceeded
        )));
        assert!(is_connection_level_error(&Error::from(ErrorKind::EOF)));
    }

    /// The handler's allow-list is deliberately narrower than what a user calls
    /// a connection failure: it only names the errors that can arrive on the
    /// read path. Whatever it does tear the connection down for, though, the
    /// user has to see as a connection failure too, or the two answers
    /// contradict each other for the same error.
    #[test]
    fn the_public_predicate_agrees_on_every_connection_level_error() {
        for error in [
            Error::from(ClientError::CannotParseInteger),
            Error::from(ClientError::UnknownRespTag('?')),
            Error::from(ClientError::MaxNestingDepthExceeded),
            Error::from(ClientError::VerbatimStringTooShort),
            Error::from(ClientError::BulkLengthTooLarge),
            Error::from(ErrorKind::EOF),
            Error::from(ErrorKind::IO(std::sync::Arc::new(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "reset",
            )))),
        ] {
            assert!(is_connection_level_error(&error));
            assert!(
                error.is_connection_error(),
                "the handler reconnects on {error:?} but the user is told the connection is fine"
            );
        }
    }

    /// `READONLY` is the one command error that says something about the *node*
    /// rather than the command: it is what a master demoted to replica answers to
    /// a write, on a socket the server never closed. It reaches the handler as a
    /// read error *frame*, which is the form that has to be recognized.
    #[test]
    fn readonly_is_the_only_demotion_signal() {
        assert!(indicates_demoted_master(&Ok(decode_one(
            "-READONLY You can't write against a read only replica.\r\n"
        ))));

        // Another command error says nothing about the node's role, and neither
        // does an ordinary reply.
        assert!(!indicates_demoted_master(&Ok(decode_one(
            "-NOPERM no permission\r\n"
        ))));
        assert!(!indicates_demoted_master(&Ok(decode_one("+OK\r\n"))));
        assert!(!indicates_demoted_master(&Ok(decode_one(":12\r\n"))));

        // And the same signal already turned into an `Error`, as the cluster path
        // hands per-shard errors up.
        assert!(indicates_demoted_master(&Err(Error::from(
            ErrorKind::Redis(RedisError {
                kind: RedisErrorKind::Readonly,
                description: "You can't write against a read only replica.".to_owned(),
            })
        ))));
        assert!(!indicates_demoted_master(&Err(Error::from(
            ErrorKind::Retry(Default::default())
        ))));
        assert!(!indicates_demoted_master(&Err(Error::from(ErrorKind::EOF))));
    }

    /// Decodes `str`, which must hold exactly one complete frame.
    fn decode_one(str: &str) -> crate::resp::RespResponse {
        use tokio_util::codec::Decoder;

        let mut buf: bytes::BytesMut = str.into();
        crate::resp::BufferDecoder::new()
            .decode(&mut buf)
            .unwrap()
            .expect("one complete frame")
    }

    /// The demotion signal must stay a per-message error: the caller who issued the
    /// write is entitled to the `READONLY` itself, not to whatever the ensuing
    /// reconnection substitutes for it.
    #[test]
    fn readonly_stays_a_per_message_error() {
        assert!(!is_connection_level_error(&Error::from(ErrorKind::Redis(
            RedisError {
                kind: RedisErrorKind::Readonly,
                description: "You can't write against a read only replica.".to_owned(),
            }
        ))));
    }
}
