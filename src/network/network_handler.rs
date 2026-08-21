use super::connection_mode::{ConnectionMode, ReplyRoute};
use super::message_queue::{MessageQueue, MessageToReceive, ReplyMatch};
use super::pub_sub_push::PubSubPush;
use super::reply_mode::ReplyMode;
use super::retry_policy::RetryPolicy;
use super::router::{
    Delivery, PendingSubscription, Router, SubscriptionConfirmed, UnsubscriptionConfirmed,
};
#[cfg(test)]
use super::test_hooks::{QueueMetricsTestHook, SendBatchTestHook};
use crate::{
    ClientError, Connection, ConnectionState, Error, ErrorKind, JoinHandle, MasterWatch,
    ReconnectionState, RedisError, RedisErrorKind, Result, RetryReason,
    client::{Config, Message, MessageKind, PreparedCommand, ServerConfig, StatsRecorder},
    commands::InternalPubSubCommands,
    resp::{
        ClientReplyMode, CommandKind, RespResponse, RespView, StateSlot, SubscriptionType, cmd,
    },
    sleep, spawn, timeout_future,
};
use bytes::Bytes;
use futures_util::{FutureExt, select};
use smallvec::SmallVec;
use std::borrow::Cow;

use std::{future::poll_fn, sync::Arc, task::Poll, time::Duration};
use tokio::{sync::broadcast, time::Instant};
use tracing::{Instrument, debug, error, info, info_span, trace, warn};

// Backpressure note. Nothing here ever blocks a sender: the network task owns
// the connection's whole routing state, so making it wait on a consumer would
// stall every other caller. Memory is bounded by shedding instead, and each
// place that can grow sheds in the way that suits what it carries.
//
// - `messages_to_send` grows while the connection is down and every reconnection
//   fails. `messages_to_receive` grows while the connection accepts bytes and
//   answers none. Both are capped by one `BackpressureConfig::max_queued_bytes`:
//   a message is charged when it is queued and released when its reply arrives,
//   writing it being no reason to free the memory it holds. The budget is
//   enforced on *incoming* messages only, anything replayed or retried having
//   already been accepted. Left uncapped the send queue was measured retaining
//   100 000 commands and 229 MiB.
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

// Why `Config::max_messages_per_wave` exists, kept next to the code that obeys
// it in `try_handle_message`.
//
// Draining the message channel until it is empty convoys the entire in-flight
// concurrency into one `writev`, so every caller waits for the whole batch to be
// written *and* answered. Capping the wave keeps a batch in flight at the server
// while the next one is being collected.
//
// It bounds both directions, not only the write size: the `select!` gives the
// two one task between them, so each wave hands control back after this many
// messages rather than running its source dry.
//
// The default (48) was calibrated against a live Redis over concurrency levels
// 64 → 1024 (see `RUSTIS_VS_REDIS_RS.md`, H13): the optimum is flat between 32
// and 128, 48 is within ~12% of the per-level optimum everywhere, and below 48
// in-flight messages the cap never fires, so low-concurrency behaviour is
// unchanged whatever it is set to.

pub(crate) struct NetworkHandler {
    /// What the connection is carrying, and where that sends a reply.
    mode: ConnectionMode,
    connection: Connection,
    /// for retries
    msg_sender: WeakMsgSender,
    msg_receiver: MsgReceiver,
    /// The two message queues and the totals that bound them.
    queue: MessageQueue,
    /// Where a push goes: the pub/sub subscription table and the two push sinks.
    router: Router,
    /// Which commands the server answers: the client's mirror of `CLIENT REPLY`.
    reply_mode: ReplyMode,
    /// Connection-attached state to replay when the socket is remade. Owned here
    /// and lent as `&mut` to whichever connection is being built: the network task
    /// is its only user, so no `Arc` and no lock are involved.
    connection_state: ConnectionState,
    reconnect_sender: ReconnectSender,
    auto_resubscribe: bool,
    auto_remonitor: bool,
    reconnection_state: ReconnectionState,
    /// When a reply sends its message back for another attempt, and how many
    /// attempts it gets.
    retry_policy: RetryPolicy,
    /// Send-wave cap from `Config::max_messages_per_wave`.
    max_messages_per_wave: usize,
    /// The `+switch-master` subscription, on a Sentinel connection only.
    ///
    /// It is held here rather than inside [`Connection`] because the `select!`
    /// already borrows the connection mutably to read it: a branch reading the
    /// subscription has to be a field the borrow checker can see is a different
    /// one.
    master_watch: Option<MasterWatch>,
    /// Connection-level counters a client reads: link state, server version,
    /// reconnections, shed commands. The queue depths are published by
    /// [`MessageQueue`], which owns them.
    ///
    /// Written only here, from the single network task, and only with `Relaxed`
    /// stores: an observability counter must never order anything.
    stats: Arc<StatsRecorder>,
    #[cfg(test)]
    send_batch_test_hook: Option<SendBatchTestHook>,
    #[cfg(test)]
    queue_metrics_test_hook: Option<QueueMetricsTestHook>,
}

impl NetworkHandler {
    pub(crate) async fn connect(
        config: Config,
    ) -> Result<(
        MsgSender,
        JoinHandle<()>,
        ReconnectSender,
        Arc<str>,
        Arc<StatsRecorder>,
    )> {
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

        // Only a Sentinel deployment has a failover to be told about, and only it
        // names the Sentinels to hear it from.
        let master_watch = match &config.server {
            ServerConfig::Sentinel(sentinel_config) => {
                Some(MasterWatch::new(sentinel_config, &config))
            }
            _ => None,
        };

        let connection = Connection::connect(config, &mut connection_state).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) =
            tokio::sync::mpsc::unbounded_channel();
        let (reconnect_sender, _): (ReconnectSender, ReconnectReceiver) = broadcast::channel(32);
        let tag = connection.tag().to_owned();
        let stats = StatsRecorder::new();
        stats.set_connected(true);
        stats.set_server_version(connection.server_version());

        let mut network_handler = NetworkHandler {
            mode: ConnectionMode::Connected,
            connection,
            msg_sender: msg_sender.downgrade(),
            msg_receiver,
            queue: MessageQueue::new(max_queued_bytes, Arc::clone(&stats)),
            router: Router::new(),
            reply_mode: ReplyMode::new(),
            connection_state,
            reconnect_sender: reconnect_sender.clone(),
            auto_resubscribe,
            auto_remonitor,
            reconnection_state: ReconnectionState::new(reconnection_config),
            retry_policy: RetryPolicy::new(max_command_attempts),
            max_messages_per_wave,
            master_watch,
            stats: Arc::clone(&stats),
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

        Ok((msg_sender, join_handle, reconnect_sender, tag, stats))
    }

    async fn network_loop(&mut self) -> Result<()> {
        loop {
            // The connection owns what its upkeep is and when it is due; the
            // loop owns only the fact that it must not run on a branch the
            // `select!` can cancel halfway.
            let until_maintenance = self
                .connection
                .next_maintenance()
                .map_or(NO_MAINTENANCE_DELAY, |due| {
                    due.saturating_duration_since(Instant::now())
                });

            select! {
                msg = poll_fn(|cx| self.msg_receiver.poll_recv(cx)).fuse() => {
                    if !self.try_handle_message(msg).await { break; }
                },
                result = self.connection.read().fuse() => {
                    if !self.try_handle_result(result).await { break; }
                },
                () = sleep(until_maintenance).fuse() => {
                    if self.connection.run_maintenance().await {
                        info!("The Sentinels announce another master, rediscovering it");
                        if !self.reconnect().await { break; }
                    }
                },
                () = watch_switch(&mut self.master_watch).fuse() => {
                    info!("A Sentinel announced a failover, rediscovering the master");
                    if !self.reconnect().await { break; }
                }
            }

            // One publication per iteration rather than one per site that moves
            // the totals: the queues only change inside this body, so no reader
            // can tell the difference and the accounting keeps a single owner.
            self.queue.publish();
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
        // Messages taken from the channel without returning to the `select!`.
        let mut handled: usize = 0;

        loop {
            if let Some(msg) = msg {
                self.handle_message(msg);
                queued += 1;
                handled += 1;
            } else {
                is_channel_closed = true;
                break;
            }

            // Send in waves rather than accumulating the whole channel into
            // one write (see `Config::max_messages_per_wave`).
            if queued >= self.max_messages_per_wave {
                if !self.mode.is_disconnected() {
                    self.send_messages().await;
                }
                queued = 0;
            }

            // Hand the loop back to the `select!`, so a caller flooding the
            // channel cannot hold the one task the two directions share and
            // starve every reply. The channel keeps whatever is left; the next
            // poll takes the next wave.
            if handled >= self.max_messages_per_wave {
                break;
            }

            match self.msg_receiver.try_recv() {
                Ok(m) => msg = Some(m),
                Err(_) => {
                    // there are no messages available, but channel is not yet closed
                    break;
                }
            }
        }

        if !self.mode.is_disconnected() {
            self.send_messages().await
        }

        #[cfg(test)]
        if let Some(hook) = &self.queue_metrics_test_hook {
            hook.record_write_wave(handled);
        }

        !is_channel_closed
    }

    /// Test-only: samples the current queue depths into the metrics hook.
    ///
    /// Called wherever a depth can be at its peak — right after the pushes, and
    /// right before a drain or a purge rebuilds the queue — and once after a
    /// drain, so `queued_commands` is a live value rather than only a peak.
    #[cfg(test)]
    fn record_queue_depths(&self) {
        if let Some(hook) = &self.queue_metrics_test_hook {
            hook.record_queue_depths(
                self.queue.to_send_len(),
                self.queue.to_receive_len(),
                self.queue.queued_commands(),
            );
        }
    }

    fn handle_message(&mut self, mut msg: Message) {
        trace!("[{:?}] Will handle message: {msg:?}", self.mode);

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
        let will_be_queued = !self.mode.is_disconnected() || msg.retry_on_error;
        if will_be_queued
            && msg.attempts == 0
            && !matches!(msg.kind, MessageKind::Invalidation { .. })
            && self.queue.would_exceed_budget(msg.queued_bytes())
        {
            debug!(
                "send queue is full ({} bytes), shedding command: {:?}",
                self.queue.queued_bytes(),
                msg.commands()
            );
            self.stats.record_shed();
            msg.send_error(Error::from(ClientError::SendQueueFull));
            return;
        }

        let mut collision_error = None;

        match &self.mode {
            ConnectionMode::Connected => {
                match &mut msg.kind {
                    MessageKind::PubSub {
                        subscription_type,
                        subscriptions,
                        ..
                    } => {
                        for (channel_or_pattern, _sender) in subscriptions.iter() {
                            if self.router.is_subscribed(channel_or_pattern) {
                                debug!(
                                    "[{:?}] There is already a subscription on channel `{}`",
                                    self.mode,
                                    String::from_utf8_lossy(channel_or_pattern)
                                );
                                collision_error = Some(Error::from(ClientError::AlreadySubscribed));
                                break;
                            }
                        }

                        if collision_error.is_none() {
                            let subscriptions = std::mem::take(subscriptions);
                            let pending_subscriptions =
                                subscriptions
                                    .into_iter()
                                    .map(|(channel_or_pattern, sender)| PendingSubscription {
                                        channel_or_pattern,
                                        subscription_type: *subscription_type,
                                        sender,
                                    });

                            self.router.expect_subscriptions(pending_subscriptions);
                        }
                    }
                    MessageKind::Monitor { push_sender, .. } => {
                        self.mode.enter_monitor();
                        let push_sender = push_sender.take();
                        if let Some(push_sender) = push_sender {
                            debug!("Registering MONITOR push_sender");
                            self.router.set_monitor_sink(push_sender);
                        }
                    }
                    MessageKind::Invalidation { push_sender } => {
                        let push_sender = push_sender.take();
                        if let Some(push_sender) = push_sender {
                            debug!("Registering Invalidation push_sender");
                            self.router.set_invalidation_sink(push_sender);
                        }
                        return; // no message to send
                    }
                    MessageKind::Single { command, .. } => {
                        if let CommandKind::Unsbuscribe(subscription_type) = command.kind() {
                            // A channel-less form names nothing: what it cancels
                            // is every subscription of its kind the connection
                            // holds, and only the router knows those. Read off
                            // the command instead, it would wait for no
                            // confirmation and release its caller at once —
                            // while, in a cluster, the other nodes are still
                            // cancelling.
                            let channels = if command.num_args() > 0 {
                                command.args().map(|a| (a, *subscription_type)).collect()
                            } else {
                                self.router.subscriptions_of(*subscription_type)
                            };
                            self.router.expect_unsubscriptions(channels);
                        }
                    }

                    _ => (),
                }

                if let Some(err) = collision_error {
                    msg.send_error(err);
                } else {
                    self.queue.push_to_send(msg);
                }
            }
            ConnectionMode::Disconnected => {
                if msg.retry_on_error {
                    debug!(
                        "network disconnected, queuing command: {:?}",
                        msg.commands()
                    );
                    self.queue.push_to_send(msg);
                } else {
                    debug!(
                        "network disconnected, sending command in error: {:?}",
                        msg.commands()
                    );
                    msg.send_error(Error::from(ErrorKind::DisconnectedByPeer));
                }
            }
            // Monitoring, at either edge: nothing here subscribes or registers a
            // sink, so the message is only queued — and watched for the `RESET`
            // that ends the stream.
            _ => {
                for command in msg.commands() {
                    self.mode.observe_queued(*command.kind());
                }
                self.queue.push_to_send(msg);
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
        // needs the count, so the count is read here rather than inside the
        // macro argument. Guarding it with `enabled!` instead would silence the
        // line for every `log`-only consumer, which the bridge exists to serve.
        // Reading a maintained total costs the same whether or not anything is
        // listening, which is why the total is maintained.
        if self.queue.queued_commands() > 1 {
            debug!("sending batch of {} commands", self.queue.queued_commands());
        }

        // Test-only: force retry reasons onto the first message of this drain so
        // a test can reproduce a redirected message ahead of unrelated ones.
        #[cfg(test)]
        if let Some(hook) = &self.send_batch_test_hook
            && !self.queue.to_send_is_empty()
            && let Some(reasons) = hook.take_injection()
            && let Some(front) = self.queue.front_to_send_mut()
        {
            front.message.retry_reasons = Some(reasons);
        }

        let start_len = self.queue.to_receive_len();

        // Taking a message releases its command charge and hands back its byte
        // charge, which follows the message: writing it frees no memory. A
        // message that awaits no reply is released below, having nowhere else to
        // be released.
        while let Some((mut msg, cost)) = self.queue.pop_to_send() {
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
                    // `SKIP` is not connection state: it is consumed by the next
                    // command and leaves the connection as it found it.
                    CommandKind::ClientReply(ClientReplyMode::On | ClientReplyMode::Off) => {
                        self.connection_state.record(StateSlot::ReplyMode, command);
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
                        self.router.clear_subscriptions();
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

                if self.reply_mode.admit(kind) {
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
                self.queue.await_reply(msg, num_commands_to_receive, cost);
            } else {
                self.queue.release(cost);
            }
        }

        if let Err(e) = self.connection.flush().await {
            error!("Flush error: {e}");

            for msg_to_receive in self.queue.rollback_awaiting(start_len) {
                msg_to_receive.message.send_error(e.clone());
            }
        }

        // Sampled after the drain too: the command total is a live figure, and a
        // queue that emptied is only observable from the low side.
        #[cfg(test)]
        self.record_queue_depths();
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the drain loop runs only while `handled` is below \
                  `max_messages_per_wave`, so the counter is bounded by it."
    )]
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
        // Replies handled without returning to the `select!`.
        let mut handled: usize = 1;

        // OPTIMIZATION : Drain the next available results in the buffer, up to
        // the same wave cap the send side obeys: a firehose — a `MONITOR` feed, a
        // busy subscription — would otherwise hold the task and starve every
        // send. The frames left are still in the decoder's buffer, so the next
        // `read` returns them without waiting on the socket.
        while handled < self.max_messages_per_wave
            && let Poll::Ready(result) = self.connection.try_read()
        {
            handled += 1;
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

        #[cfg(test)]
        if let Some(hook) = &self.queue_metrics_test_hook {
            hook.record_read_wave(handled);
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
    /// before the next ready reply is parsed. What overlaps is the wake-up: on a
    /// multi-thread runtime the woken caller resumes on another worker while this
    /// task keeps draining. The draining itself overlaps with nothing — the
    /// network task is single — so the early dispatch buys the first caller's
    /// latency, not parallel parsing.
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
        let is_monitor_line = matches!(&result, Ok(response) if response.is_monitor());

        match self.mode.route_reply(is_monitor_line) {
            ReplyRoute::Dropped => (),
            ReplyRoute::MonitorSink => self.deliver_monitor_result(result),
            ReplyRoute::ToCaller => self.receive_result(result),
            ReplyRoute::Routed => match &result {
                Ok(response) if response.is_push() => {
                    if let Some(response) = self.try_match_pubsub_message(result) {
                        if response.is_err() {
                            self.receive_result(response);
                        } else {
                            match self.router.invalidation_sink_mut() {
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
        }
    }

    /// Hands a monitor line to the `MONITOR` sink, if one is registered.
    fn deliver_monitor_result(&self, result: Result<RespResponse>) {
        #[cfg(test)]
        let delivered_bytes = result
            .as_ref()
            .map(|response| response.retained_bytes())
            .unwrap_or(0);

        let Some(push_sender) = self.router.monitor_sink() else {
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

    fn receive_result(&mut self, result: Result<RespResponse>) {
        match self.queue.match_reply(result) {
            ReplyMatch::Discarded(result) => {
                debug!("discarding response of an already resolved message: {result:?}");
            }
            ReplyMatch::Absorbed => (),
            ReplyMatch::Completed(message_to_receive, result) => {
                trace!("message_to_receive: {message_to_receive:?}");
                self.resolve(message_to_receive, result);
            }
            ReplyMatch::Unmatched(result) => {
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

    /// Sends a resolved message back for another attempt, or answers its caller.
    fn resolve(&mut self, mut message_to_receive: MessageToReceive, result: Result<RespResponse>) {
        if self
            .retry_policy
            .asks_for_retry(&result, &message_to_receive.message)
        {
            self.retry_policy
                .absorb_reasons(&mut message_to_receive.message, result);

            // A command caught in a pathological redirect loop would otherwise be
            // replayed forever, so a message out of budget is failed with a
            // distinct error.
            if !self
                .retry_policy
                .charge_attempt(&mut message_to_receive.message)
            {
                debug!("Message reached the maximum number of attempts, failing it");
                message_to_receive
                    .message
                    .send_error(Error::from(ClientError::MaxCommandAttemptsReached));
            }
            // retry: upgrade the weak handle just long enough to requeue the
            // message. A failed upgrade means every client is gone and the
            // channel is closing, so the retry is moot.
            else if let Some(msg_sender) = self.msg_sender.upgrade() {
                if let Err(e) = msg_sender.send(message_to_receive.message) {
                    error!("Cannot retry message: {e}");
                }
            } else {
                debug!("Cannot retry message: channel closed");
            }

            return;
        }

        trace!("Will respond to: {:?}", message_to_receive.message);

        // This path answers the caller directly instead of going through
        // `Message::send_error`, so it names the command itself. It carries the
        // server's own errors — a `WRONGTYPE`, a `NOPERM` — which are exactly
        // the ones a caller cannot act on without knowing what drew them.
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
                self.dispatch_result(result_sender, result, command_name.as_ref());
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
                    self.dispatch_result(results_sender, Err(e), command_name.as_ref());
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

    /// Test-only: records what a pub/sub delivery did, so a test can measure how
    /// much traffic a paused subscriber's channel absorbs.
    #[cfg_attr(
        not(test),
        expect(unused_variables, reason = "no hook outside a test build")
    )]
    fn record_delivery(&self, delivery: &Delivery) {
        #[cfg(test)]
        if let Some(hook) = &self.queue_metrics_test_hook {
            match delivery {
                Delivery::Delivered { retained_bytes } => {
                    hook.record_pub_sub_delivered(*retained_bytes);
                }
                Delivery::SubscriberGone => hook.record_pub_sub_delivery_failed(),
                // Nothing was offered to a channel, so nothing is counted
                // either way: the message named a channel this connection
                // holds no subscription for.
                Delivery::NoSubscriber => (),
            }
        }
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
        if !self.router.has_orphaned() {
            return;
        }

        for (channel_or_pattern, subscription_type) in self.router.take_orphaned() {
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

        if !self.mode.is_disconnected() {
            self.send_messages().await;
        }
    }

    fn try_match_pubsub_message(
        &mut self,
        value: Result<RespResponse>,
    ) -> Option<Result<RespResponse>> {
        if let Ok(ref_value) = &value {
            // A node holding no subscription of the kind answers a channel-less
            // UNSUBSCRIBE with a nil channel. It confirms nothing and cancels
            // nothing, so it is dropped rather than reported to a caller who
            // never named it — in a cluster every master answers, and all but
            // the ones holding a subscription answer this.
            if is_empty_unsubscribe_confirmation(ref_value) {
                return None;
            }
            if let Ok(pub_sub_message) = PubSubPush::try_from(ref_value) {
                match pub_sub_message {
                    PubSubPush::Message(channel_or_pattern, _)
                    | PubSubPush::SMessage(channel_or_pattern, _) => {
                        // The name is copied before the send, which consumes
                        // `value` and with it the borrowed channel name a log
                        // line still needs.
                        let named = Bytes::copy_from_slice(channel_or_pattern);
                        let delivery = self.router.deliver(&named, value);
                        self.record_delivery(&delivery);
                        match delivery {
                            Delivery::Delivered { .. } => (),
                            Delivery::SubscriberGone => warn!(
                                "Cannot send pub/sub message to caller from channel `{}`: the receiver is gone",
                                String::from_utf8_lossy(&named)
                            ),
                            Delivery::NoSubscriber => error!(
                                "Unexpected message on channel `{}`",
                                String::from_utf8_lossy(&named)
                            ),
                        }
                        None
                    }
                    PubSubPush::Subscribe(channel_or_pattern)
                    | PubSubPush::PSubscribe(channel_or_pattern)
                    | PubSubPush::SSubscribe(channel_or_pattern) => {
                        let named = Bytes::copy_from_slice(channel_or_pattern);
                        match self.router.confirm_subscription(&named) {
                            SubscriptionConfirmed::AlreadySubscribed => {
                                return Some(Err(Error::from(ClientError::AlreadySubscribed)));
                            }
                            // A batch of subscriptions is answered once, at the
                            // last confirmation.
                            SubscriptionConfirmed::Registered { more_to_come: true } => {
                                return None;
                            }
                            SubscriptionConfirmed::Registered {
                                more_to_come: false,
                            } => self.receive_result(Ok(RespResponse::ok())),
                            SubscriptionConfirmed::Unexpected => {
                                error!(
                                    "Unexpected subscription confirmation on channel `{}`",
                                    String::from_utf8_lossy(&named)
                                );
                                // Surface the anomaly to the caller instead of reporting
                                // a spurious success; the pending entry is left intact.
                                self.receive_result(Err(Error::from(
                                    ClientError::UnexpectedSubscriptionConfirmation,
                                )));
                            }
                        }
                        None
                    }
                    PubSubPush::Unsubscribe(channel_or_pattern)
                    | PubSubPush::PUnsubscribe(channel_or_pattern)
                    | PubSubPush::SUnsubscribe(channel_or_pattern) => {
                        let named = Bytes::copy_from_slice(channel_or_pattern);
                        match self.router.confirm_unsubscription(&named) {
                            UnsubscriptionConfirmed::More => None,
                            UnsubscriptionConfirmed::Complete => {
                                self.receive_result(Ok(RespResponse::ok()));
                                None
                            }
                            // Nobody here asked for it, so it belongs to the
                            // caller as a plain reply.
                            UnsubscriptionConfirmed::Unsolicited => Some(value),
                        }
                    }
                    PubSubPush::PMessage(pattern, channel, _) => {
                        let named_pattern = Bytes::copy_from_slice(pattern);
                        let named_channel = Bytes::copy_from_slice(channel);
                        let delivery = self.router.deliver(&named_pattern, value);
                        self.record_delivery(&delivery);
                        match delivery {
                            Delivery::Delivered { .. } => (),
                            Delivery::SubscriberGone => warn!(
                                "Cannot send pub/sub message to caller for pattern `{}`: the receiver is gone",
                                String::from_utf8_lossy(&named_pattern)
                            ),
                            Delivery::NoSubscriber => error!(
                                "Unexpected message on channel `{}` for pattern `{}`",
                                String::from_utf8_lossy(&named_channel),
                                String::from_utf8_lossy(&named_pattern)
                            ),
                        }
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
    async fn reconnect(&mut self) -> bool {
        debug!("reconnecting...");
        let was_monitoring = self.abandon_connection();

        loop {
            let Some(delay) = self.reconnection_state.next_delay() else {
                self.abandon_client();
                return false;
            };

            debug!("Waiting {delay} ms before reconnection");
            if !self
                .serve_until(backoff_deadline(Instant::now(), delay))
                .await
            {
                return false;
            }

            if let Err(e) = self.connection.reconnect(&mut self.connection_state).await {
                error!("Failed to reconnect: {e:?}");
                continue;
            }

            if let Err(e) = self.restore_connection(was_monitoring).await {
                error!("Failed to reconnect: {e:?}");
                continue;
            }

            return true;
        }
    }

    /// Gives up the dead socket and reports whether it was carrying a MONITOR
    /// stream, which is what the restored one has to resume.
    fn abandon_connection(&mut self) -> bool {
        let was_monitoring = self.mode.disconnect().is_monitoring();
        self.stats.set_connected(false);

        // A `SKIP` waiting for the command it silences died with the connection too.
        self.reply_mode.forget_pending_skip();

        // A fresh connection is subscribed to nothing, so an orphaned
        // subscription has already achieved what its UNSUBSCRIBE was for.
        self.router.clear_orphaned();

        // Sampled before the purge, so a purge reads as the high-water mark
        // stopping rather than as a depth that fell.
        #[cfg(test)]
        self.record_queue_depths();

        // Keep only what may be replayed, and fail the rest. The reconnection
        // replay counts as one attempt, which the purge charges.
        self.queue.purge_for_replay(&self.retry_policy);

        was_monitoring
    }

    /// Gives up the client itself: the reconnection policy is out of attempts,
    /// so nothing queued will ever be sent.
    fn abandon_client(&mut self) {
        error!("Max reconnection attempts reached: the client is finished");
        // Taking empties both queues and both totals, so nothing is left
        // holding a charge for a message that will never be sent.
        for message in self.queue.take_all() {
            message.send_error(Error::from(ErrorKind::DisconnectedByPeer));
        }
        self.queue.publish();
    }

    /// Keeps serving callers until `deadline`, and reports whether the task
    /// should carry on.
    ///
    /// A backoff is a wait, not an outage: a caller that sends during it is
    /// queued like any other and goes out with the replay. Parking the channel
    /// instead would make every reconnection look like a burst of failures to
    /// callers whose command was never even attempted.
    ///
    /// `false` means the message channel closed — the last client is gone and
    /// there is nothing left to reconnect for.
    async fn serve_until(&mut self, deadline: Instant) -> bool {
        loop {
            let delay = deadline.duration_since(Instant::now());
            match timeout_future(delay, poll_fn(|cx| self.msg_receiver.poll_recv(cx))).await {
                Ok(msg) => {
                    if !self.try_handle_message(msg).await {
                        return false;
                    }
                }
                // The deadline expired: time to try the socket again.
                Err(_) => return true,
            }
        }
    }

    /// Puts back on a fresh socket everything the dead one was carrying, and
    /// replays what it still owed.
    async fn restore_connection(&mut self, was_monitoring: bool) -> Result<()> {
        // The new connection was restored from the same registry, so the
        // mirror the send loop uses has to be derived from it rather than
        // left at whatever the dead connection ended on.
        self.reply_mode.restore(self.connection_state.is_reply_on());

        if self.auto_resubscribe {
            self.auto_resubscribe().await?;
        }

        if self.auto_remonitor {
            self.auto_remonitor(was_monitoring).await?;
        }

        // Nobody listening is not a reason to abandon a connection that is up:
        // this is a notification, not a step of the restoration.
        if let Err(e) = self.reconnect_sender.send(()) {
            debug!("Cannot send reconnect notification to clients: {e}");
        }

        // Restore what the connection carries before replaying in-flight
        // messages so that they are routed through `handle_message`,
        // exactly as fresh messages and the retry path are.
        self.mode
            .restore(was_monitoring, self.router.has_monitor_sink());

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
        // Taking zeroes both totals with the queues it empties, so a message
        // the replay queues again is charged once rather than once per pass.
        for message in self.queue.take_all() {
            self.handle_message(message);
        }

        self.send_messages().await;

        info!("reconnected!");
        self.stats.set_connected(true);
        self.stats
            .set_server_version(self.connection.server_version());
        self.stats.record_reconnection();
        self.reconnection_state.reset_attempts();

        Ok(())
    }

    async fn auto_resubscribe(&mut self) -> Result<()> {
        // The router decides what the new socket has to carry: the pending
        // unsubscriptions are dropped, a fresh connection being subscribed to
        // nothing, and a pending subscription is promoted to a confirmed one,
        // being re-issued here.
        for (channel_or_pattern, subscription_type) in self.router.take_resubscriptions() {
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

        Ok(())
    }

    async fn auto_remonitor(&mut self, was_monitoring: bool) -> Result<()> {
        if was_monitoring {
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

/// Whether this push is a `nil` unsubscription confirmation — what a server
/// answers a channel-less UNSUBSCRIBE when it holds no such subscription.
///
/// `PubSubPush` cannot carry it: its channel is a bulk string there, and this
/// one has none.
fn is_empty_unsubscribe_confirmation(response: &RespResponse) -> bool {
    let Ok(RespView::Push(push)) = response.view() else {
        return false;
    };
    let mut fields = push.into_iter();
    let Some(Ok(RespView::BulkString(kind @ (b"unsubscribe" | b"punsubscribe" | b"sunsubscribe")))) =
        fields.next()
    else {
        return false;
    };
    let _ = kind;
    matches!(fields.next(), Some(Ok(RespView::Null)))
}

/// Waits for the `+switch-master` subscription to announce a failover, and never
/// resolves where there is no subscription.
///
/// Standing in for "never" is what keeps the `select!` one shape for every server
/// kind: a standalone or cluster deployment has no Sentinel to hear from, so its
/// branch simply never wins.
async fn watch_switch(master_watch: &mut Option<MasterWatch>) {
    match master_watch {
        Some(master_watch) => master_watch.switched().await,
        None => std::future::pending().await,
    }
}

/// Stands in for "never" on a connection with no upkeep of its own, so its
/// `select!` branch never wins and the loop stays one shape for every server kind.
const NO_MAINTENANCE_DELAY: Duration = Duration::from_secs(3600);

/// Stands in for a delay the monotonic clock cannot represent.
const UNREPRESENTABLE_BACKOFF: Duration = Duration::from_secs(3600);

/// When a backoff of `delay` milliseconds, started at `start`, is over.
///
/// A reconnection policy is caller-supplied, so the delay is an arbitrary `u64`
/// of milliseconds. Adding one to an `Instant` panics on overflow, and a panic
/// here kills the network task — the sole owner of the routing state, with no
/// reconnection loop left to recover — so the sum is checked rather than taken.
///
/// Which delays actually overflow is platform-dependent, and on the usual ones
/// none does: a Linux `Instant` holds a `timespec`, whose seconds are an `i64`,
/// so no `u64` of milliseconds comes close. This is a guard for the platforms
/// with a narrower clock, not a cap on what a policy may ask for — a delay of
/// `u64::MAX` is honoured as the ~584-million-year wait it is.
fn backoff_deadline(start: Instant, delay: u64) -> Instant {
    start
        .checked_add(Duration::from_millis(delay))
        .or_else(|| start.checked_add(UNREPRESENTABLE_BACKOFF))
        .unwrap_or(start)
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
    use super::{
        Duration, Instant, backoff_deadline, indicates_demoted_master, is_connection_level_error,
    };
    use crate::{ClientError, Error, ErrorKind, RedisError, RedisErrorKind};

    #[test]
    fn a_backoff_ends_where_its_delay_puts_it() {
        let start = Instant::now();
        assert_eq!(
            start + Duration::from_millis(250),
            backoff_deadline(start, 250)
        );
    }

    #[test]
    fn a_zero_delay_is_over_at_once() {
        let start = Instant::now();
        assert_eq!(start, backoff_deadline(start, 0));
    }

    #[test]
    fn the_largest_delay_a_policy_can_return_yields_a_deadline_instead_of_a_panic() {
        // A caller-supplied policy may return any `u64`. Whether that overflows
        // the clock is platform-dependent, so what is pinned is the invariant
        // that holds everywhere: a deadline in the future, and no panic.
        let start = Instant::now();
        assert!(backoff_deadline(start, u64::MAX) > start);
    }

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
                description: bytes::Bytes::from_static(b"no permission"),
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
                description: bytes::Bytes::from_static(
                    b"You can't write against a read only replica."
                ),
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
                description: bytes::Bytes::from_static(
                    b"You can't write against a read only replica."
                ),
            }
        ))));
    }
}
