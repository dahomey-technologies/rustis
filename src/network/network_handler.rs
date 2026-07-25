use super::pub_sub_message::PubSubMessage;
use crate::{
    ClientError, Connection, Error, JoinHandle, ReconnectionState, Result, RetryReason,
    client::{Config, Message, MessageKind},
    commands::InternalPubSubCommands,
    resp::{ClientReplyMode, CommandKind, RespResponse, SubscriptionType, cmd},
    spawn, timeout,
};
use bytes::Bytes;
use futures_channel::mpsc;
use futures_util::{FutureExt, select};
use log::{Level, debug, error, info, log_enabled, trace, warn};
use smallvec::SmallVec;
use std::{
    collections::{HashMap, VecDeque},
    future::poll_fn,
    sync::Arc,
    task::Poll,
    time::Duration,
};
use tokio::{sync::broadcast, time::Instant};

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
pub(crate) type PubSubSender = mpsc::UnboundedSender<Result<RespResponse>>;
pub(crate) type PubSubReceiver = mpsc::UnboundedReceiver<Result<RespResponse>>;
pub(crate) type PushSender = mpsc::UnboundedSender<Result<RespResponse>>;
pub(crate) type PushReceiver = mpsc::UnboundedReceiver<Result<RespResponse>>;
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
impl SendBatchTestHook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues retry reasons to be forced onto the first message of the next
    /// send drain (or `None` to skip that drain).
    pub fn push_injection(&self, reasons: Option<Vec<RetryReason>>) {
        self.inject_first_message_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .push_back(reasons);
    }

    /// Returns the recorded `(command name, number of retry reasons fed)`
    /// entries, in feed order.
    pub fn fed_retry_reasons(&self) -> Vec<(String, usize)> {
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
    pub fn arm_kill_on_read_for(&self, command_name: &str, num_reads: usize) {
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

/// Maximum number of messages collected into a single write (see
/// [`NetworkHandler::try_handle_message`]).
///
/// Draining the message channel until it is empty convoys the entire in-flight
/// concurrency into one `writev`, so every caller waits for the whole batch to be
/// written *and* answered. Capping the wave keeps a batch in flight at the server
/// while the next one is being collected.
///
/// Calibrated against a live Redis over concurrency levels 64 → 1024 (see
/// `RUSTIS_VS_REDIS_RS.md`, H13). The optimum is flat between 32 and 128 and only
/// mildly concurrency-dependent; what matters is that the cap stays *below* the
/// in-flight concurrency, otherwise it never fires and the convoy returns. 48 is
/// within ~12% of the per-level optimum everywhere and beats the uncapped drain
/// at every level from 64 tasks up. Below 48 concurrent in-flight messages the cap
/// never fires, so low-concurrency behaviour is unchanged.
const MAX_MESSAGES_PER_WAVE: usize = 48;

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
    pub fn new(message: Message) -> Self {
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
    pub fn new(message: Message, num_commands: usize) -> Self {
        Self {
            message,
            num_commands,
            pending_responses: Vec::new(),
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
    is_reply_on: bool,
    push_sender: Option<PushSender>,
    reconnect_sender: ReconnectSender,
    auto_resubscribe: bool,
    auto_remonitor: bool,
    tag: Arc<str>,
    reconnection_state: ReconnectionState,
    /// Number of incoming results belonging to a message that has already been
    /// resolved, and which must therefore be dropped instead of matched.
    results_to_discard: usize,
    #[cfg(test)]
    send_batch_test_hook: Option<SendBatchTestHook>,
}

impl NetworkHandler {
    pub async fn connect(
        config: Config,
    ) -> Result<(MsgSender, JoinHandle<()>, ReconnectSender, Arc<str>)> {
        // options
        let auto_resubscribe = config.auto_resubscribe;
        let auto_remonitor = config.auto_remonitor;
        let reconnection_config = config.reconnection.clone();
        #[cfg(test)]
        let send_batch_test_hook = config.send_batch_test_hook.clone();

        let connection = Connection::connect(config).await?;
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
            is_reply_on: true,
            push_sender: None,
            reconnect_sender: reconnect_sender.clone(),
            auto_resubscribe,
            auto_remonitor,
            tag: tag.clone(),
            reconnection_state: ReconnectionState::new(reconnection_config),
            results_to_discard: 0,
            #[cfg(test)]
            send_batch_test_hook,
        };

        let join_handle = spawn(async move {
            if let Err(e) = network_handler.network_loop().await {
                error!("[{}] network loop ended in error: {e}", network_handler.tag);
            }
        });

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

        debug!("[{}] end of network loop", self.tag);
        Ok(())
    }

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
            // one write (see `MAX_MESSAGES_PER_WAVE`).
            if queued >= MAX_MESSAGES_PER_WAVE {
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

    fn handle_message(&mut self, mut msg: Message) {
        trace!(
            "[{}][{:?}] Will handle message: {msg:?}",
            self.tag, self.status
        );

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
                                    "[{}][{:?}] There is already a subscription on channel `{}`",
                                    self.tag,
                                    self.status,
                                    String::from_utf8_lossy(channel_or_pattern)
                                );
                                collision_error =
                                    Some(Error::Client(ClientError::AlreadySubscribed));
                                break;
                            }
                        }

                        if collision_error.is_none() {
                            let subscriptions = std::mem::take(subscriptions);
                            let num_pending_subscriptions = subscriptions.len();
                            let pending_subscriptions = subscriptions.into_iter().enumerate().map(
                                |(index, (channel_or_pattern, sender))| PendingSubscription {
                                    channel_or_pattern,
                                    subscription_type: *subscription_type,
                                    sender,
                                    more_to_come: index < num_pending_subscriptions - 1,
                                },
                            );

                            self.pending_subscriptions.extend(pending_subscriptions);
                        }
                    }
                    MessageKind::Monitor { push_sender, .. } => {
                        self.status = Status::EnteringMonitor;
                        let push_sender = push_sender.take();
                        if let Some(push_sender) = push_sender {
                            debug!("[{}] Registering MONITOR push_sender", self.tag);
                            self.push_sender = Some(push_sender);
                        }
                    }
                    MessageKind::Invalidation { push_sender } => {
                        let push_sender = push_sender.take();
                        if let Some(push_sender) = push_sender {
                            debug!("[{}] Registering Invalidation push_sender", self.tag);
                            self.push_sender = Some(push_sender);
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
                    msg.send_error(&self.tag, err);
                } else {
                    self.messages_to_send.push_back(MessageToSend::new(msg));
                }
            }
            Status::Disconnected => {
                if msg.retry_on_error {
                    debug!(
                        "[{}] network disconnected, queuing command: {:?}",
                        self.tag,
                        msg.commands()
                    );
                    self.messages_to_send.push_back(MessageToSend::new(msg));
                } else {
                    debug!(
                        "[{}] network disconnected, sending command in error: {:?}",
                        self.tag,
                        msg.commands()
                    );
                    msg.send_error(&self.tag, Error::DisconnectedByPeer);
                }
            }
            Status::EnteringMonitor => self.messages_to_send.push_back(MessageToSend::new(msg)),
            Status::Monitor => {
                for command in msg.commands() {
                    if matches!(command.kind(), CommandKind::Reset) {
                        self.status = Status::LeavingMonitor;
                    }
                }
                self.messages_to_send.push_back(MessageToSend::new(msg));
            }
            Status::LeavingMonitor => {
                self.messages_to_send.push_back(MessageToSend::new(msg));
            }
        }
    }

    async fn send_messages(&mut self) {
        if log_enabled!(Level::Debug) {
            let num_commands = self
                .messages_to_send
                .iter()
                .fold(0, |sum, msg| sum + msg.message.num_commands());
            if num_commands > 1 {
                debug!("[{}] sending batch of {} commands", self.tag, num_commands);
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
                match command.kind() {
                    CommandKind::ClientReply(ClientReplyMode::On) => self.is_reply_on = true,
                    CommandKind::ClientReply(ClientReplyMode::Off | ClientReplyMode::Skip) => {
                        self.is_reply_on = false
                    }
                    _ => (),
                }

                if self.is_reply_on {
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
                    error!("[{}] Feed error: {e}", self.tag);
                    msg.send_error(&self.tag, e);
                    return;
                }
            }

            if num_commands_to_receive > 0 {
                self.messages_to_receive
                    .push_back(MessageToReceive::new(msg, num_commands_to_receive));
            }
        }

        if let Err(e) = self.connection.flush().await {
            error!("[{}] Flush error: {e}", self.tag);

            while self.messages_to_receive.len() > start_idx {
                if let Some(msg_to_receive) = self.messages_to_receive.pop_back() {
                    msg_to_receive.message.send_error(&self.tag, e.clone());
                }
            }
        }
    }

    async fn try_handle_result(&mut self, result: Option<Result<RespResponse>>) -> bool {
        let Some(result) = result else {
            return self.reconnect().await;
        };
        self.handle_result(result);

        // OPTIMIZATION : Drain the next available results in the buffer
        while let Poll::Ready(result) = self.connection.try_read() {
            let Some(result) = result else {
                return self.reconnect().await;
            };
            self.handle_result(result);
        }

        true
    }

    /// Hands a matched reply to its caller, waking it.
    ///
    /// Called from [`Self::receive_result`] the moment the reply is matched,
    /// before the next ready reply is parsed: on a multi-thread runtime another
    /// worker resumes the caller in parallel while this task keeps draining,
    /// which shortens first-reply latency on the critical path.
    fn dispatch_result<T>(&self, sender: tokio::sync::oneshot::Sender<T>, value: T) {
        if sender.send(value).is_err() {
            warn!(
                "[{}] Cannot send value to caller because receiver is not there anymore",
                self.tag
            );
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
                            match &mut self.push_sender {
                                Some(push_sender) => {
                                    if let Err(e) = push_sender.unbounded_send(response) {
                                        warn!(
                                            "[{}] Cannot send push message result to caller: {e}",
                                            self.tag
                                        );
                                    }
                                }
                                None => {
                                    warn!(
                                        "[{}] Received a push message with no sender configured: {response:?}",
                                        self.tag
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
                    if let Some(push_sender) = &mut self.push_sender
                        && let Err(e) = push_sender.unbounded_send(result)
                    {
                        warn!("[{}] Cannot send monitor result to caller: {e}", self.tag);
                    }
                }
                _ => self.receive_result(result),
            },
            Status::LeavingMonitor => match &result {
                Ok(response) if response.is_monitor() => {
                    if let Some(push_sender) = &mut self.push_sender
                        && let Err(e) = push_sender.unbounded_send(result)
                    {
                        warn!("[{}] Cannot send monitor result to caller: {e}", self.tag);
                    }
                }
                _ => {
                    self.receive_result(result);
                    self.status = Status::Connected;
                }
            },
        }
    }

    fn receive_result(&mut self, result: Result<RespResponse>) {
        // Responses owed to a message that was already resolved as a whole: the
        // commands were executed, so their replies still arrive, but there is no
        // caller left for them. Matching them would shift every subsequent
        // response by one.
        if self.results_to_discard > 0 {
            self.results_to_discard -= 1;
            debug!(
                "[{}] discarding response of an already resolved message: {result:?}",
                self.tag
            );
            return;
        }

        match self.messages_to_receive.front_mut() {
            Some(message_to_receive) => {
                log::trace!("message_to_receive: {:?}", message_to_receive);

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

                        if let Err(Error::Retry(_)) = &result {
                            should_retry = true;
                        } else if message_to_receive.message.retry_reasons.is_some() {
                            should_retry = true;
                        }

                        if should_retry {
                            if let Err(Error::Retry(reasons)) = result {
                                if let Some(retry_reasons) =
                                    &mut message_to_receive.message.retry_reasons
                                {
                                    retry_reasons.extend(reasons);
                                } else {
                                    message_to_receive.message.retry_reasons =
                                        Some(Vec::from_iter(reasons));
                                }
                            }

                            // retry: upgrade the weak handle just long enough to
                            // requeue the message. A failed upgrade means every
                            // client is gone and the channel is closing, so the
                            // retry is moot.
                            if let Some(msg_sender) = self.msg_sender.upgrade() {
                                if let Err(e) = msg_sender.send(message_to_receive.message) {
                                    error!("[{}] Cannot retry message: {e}", self.tag);
                                }
                            } else {
                                debug!("[{}] Cannot retry message: channel closed", self.tag);
                            }
                        } else {
                            trace!(
                                "[{}] Will respond to: {:?}",
                                self.tag, message_to_receive.message
                            );

                            match message_to_receive.message.kind {
                                MessageKind::Single {
                                    result_sender: Some(result_sender),
                                    ..
                                }
                                | MessageKind::PubSub { result_sender, .. }
                                | MessageKind::Monitor { result_sender, .. } => {
                                    self.dispatch_result(result_sender, result);
                                }
                                MessageKind::Batch { results_sender, .. } => match result {
                                    Ok(resp_buf) => {
                                        message_to_receive.pending_responses.push(resp_buf);
                                        self.dispatch_result(
                                            results_sender,
                                            Ok(message_to_receive.pending_responses),
                                        );
                                    }
                                    Err(e) => {
                                        self.dispatch_result(results_sender, Err(e));
                                    }
                                },
                                MessageKind::Invalidation { .. }
                                | MessageKind::Single {
                                    result_sender: None,
                                    ..
                                } => {
                                    debug!("[{}] forget value {result:?}", self.tag)
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
                        Err(Error::Retry(reasons)) => {
                            if let Some(retry_reasons) =
                                &mut message_to_receive.message.retry_reasons
                            {
                                retry_reasons.extend(reasons);
                            } else {
                                message_to_receive.message.retry_reasons =
                                    Some(Vec::from_iter(reasons));
                            }
                        }
                        _ => (),
                    }
                }
            }
            None => {
                // disconnection errors could end here but ok values should match a value_sender instance
                assert!(
                    result.is_err(),
                    "[{}] Received unexpected message: {result:?}",
                    self.tag
                );
            }
        }
    }

    fn try_match_pubsub_message(
        &mut self,
        value: Result<RespResponse>,
    ) -> Option<Result<RespResponse>> {
        if let Ok(ref_value) = &value {
            if let Ok(pub_sub_message) = PubSubMessage::try_from(ref_value) {
                match pub_sub_message {
                    PubSubMessage::Message(channel_or_pattern, _)
                    | PubSubMessage::SMessage(channel_or_pattern, _) => {
                        match self.subscriptions.get_mut(channel_or_pattern) {
                            Some((_subscription_type, pub_sub_sender)) => {
                                if let Err(e) = pub_sub_sender.unbounded_send(value) {
                                    let error_desc = e.to_string();
                                    if let Ok(ref_value) = &e.into_inner()
                                        && let Some(
                                            PubSubMessage::Message(channel_or_pattern, _)
                                            | PubSubMessage::SMessage(channel_or_pattern, _),
                                        ) = PubSubMessage::try_from(ref_value).ok()
                                    {
                                        warn!(
                                            "[{}] Cannot send pub/sub message to caller from channel `{}`: {error_desc}",
                                            self.tag,
                                            String::from_utf8_lossy(channel_or_pattern)
                                        );
                                    }
                                }
                            }
                            None => {
                                error!(
                                    "[{}] Unexpected message on channel `{}`",
                                    self.tag,
                                    String::from_utf8_lossy(channel_or_pattern)
                                );
                            }
                        }
                        None
                    }
                    PubSubMessage::Subscribe(channel_or_pattern)
                    | PubSubMessage::PSubscribe(channel_or_pattern)
                    | PubSubMessage::SSubscribe(channel_or_pattern) => {
                        if let Some(pending_sub) = self.pending_subscriptions.pop_front() {
                            if pending_sub.channel_or_pattern == channel_or_pattern {
                                if self
                                    .subscriptions
                                    .insert(
                                        pending_sub.channel_or_pattern,
                                        (pending_sub.subscription_type, pending_sub.sender),
                                    )
                                    .is_some()
                                {
                                    return Some(Err(Error::Client(
                                        ClientError::AlreadySubscribed,
                                    )));
                                }

                                if pending_sub.more_to_come {
                                    return None;
                                }
                            } else {
                                error!(
                                    "[{}] Unexpected subscription confirmation on channel `{}`",
                                    self.tag,
                                    String::from_utf8_lossy(channel_or_pattern)
                                );
                            }
                        } else {
                            error!(
                                "[{}] Cannot find pending subscription for channel `{}`",
                                self.tag,
                                String::from_utf8_lossy(channel_or_pattern)
                            );
                        }
                        self.receive_result(Ok(RespResponse::ok()));
                        None
                    }
                    PubSubMessage::Unsubscribe(channel_or_pattern)
                    | PubSubMessage::PUnsubscribe(channel_or_pattern)
                    | PubSubMessage::SUnsubscribe(channel_or_pattern) => {
                        self.subscriptions.remove(channel_or_pattern);
                        if let Some(remaining) = self.pending_unsubscriptions.front_mut() {
                            if remaining.len() > 1 {
                                if remaining.remove(channel_or_pattern).is_none() {
                                    error!(
                                        "[{}] Cannot find channel or pattern to remove: `{}`",
                                        self.tag,
                                        String::from_utf8_lossy(channel_or_pattern)
                                    );
                                }
                                None
                            } else {
                                // last unsubscription notification received
                                let Some(mut remaining) = self.pending_unsubscriptions.pop_front()
                                else {
                                    error!(
                                        "[{}] Cannot find channel or pattern to remove: `{}`",
                                        self.tag,
                                        String::from_utf8_lossy(channel_or_pattern)
                                    );
                                    return None;
                                };
                                if remaining.remove(channel_or_pattern).is_none() {
                                    error!(
                                        "[{}] Cannot find channel or pattern to remove: `{}`",
                                        self.tag,
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
                    PubSubMessage::PMessage(pattern, channel, _) => {
                        match self.subscriptions.get_mut(pattern) {
                            Some((_subscription_type, pub_sub_sender)) => {
                                if let Err(e) = pub_sub_sender.unbounded_send(value) {
                                    warn!(
                                        "[{}] Cannot send pub/sub message to caller: {e}",
                                        self.tag
                                    );
                                }
                            }
                            None => {
                                error!(
                                    "[{}] Unexpected message on channel `{}` for pattern `{}`",
                                    self.tag,
                                    String::from_utf8_lossy(channel),
                                    String::from_utf8_lossy(pattern)
                                );
                            }
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

    async fn reconnect(&mut self) -> bool {
        debug!("[{}] reconnecting...", self.tag);
        let old_status = self.status;
        self.status = Status::Disconnected;

        // The responses we were waiting to discard died with the connection;
        // keeping the count would discard legitimate responses afterwards.
        self.results_to_discard = 0;

        // Purge every non-retryable message, wherever it sits in the queue,
        // and keep the retryable ones in order. A prefix-only purge would leave
        // a non-retryable message behind a retryable one, and it would then be
        // replayed on reconnect — double-executing a command whose caller
        // opted out of retries.
        let mut retained_to_receive = VecDeque::with_capacity(self.messages_to_receive.len());
        while let Some(message_to_receive) = self.messages_to_receive.pop_front() {
            if message_to_receive.message.retry_on_error {
                retained_to_receive.push_back(message_to_receive);
            } else {
                message_to_receive
                    .message
                    .send_error(&self.tag, Error::DisconnectedByPeer);
            }
        }
        self.messages_to_receive = retained_to_receive;

        let mut retained_to_send = VecDeque::with_capacity(self.messages_to_send.len());
        while let Some(message_to_send) = self.messages_to_send.pop_front() {
            if message_to_send.message.retry_on_error {
                retained_to_send.push_back(message_to_send);
            } else {
                message_to_send
                    .message
                    .send_error(&self.tag, Error::DisconnectedByPeer);
            }
        }
        self.messages_to_send = retained_to_send;

        loop {
            if let Some(delay) = self.reconnection_state.next_delay() {
                debug!("[{}] Waiting {delay} ms before reconnection", self.tag);

                // keep on receiving new message during the delay
                let start = Instant::now();
                let end = start.checked_add(Duration::from_millis(delay)).unwrap();
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
                warn!("[{}] Max reconnection attempts reached", self.tag);
                while let Some(message_to_receive) = self.messages_to_receive.pop_front() {
                    message_to_receive
                        .message
                        .send_error(&self.tag, Error::DisconnectedByPeer);
                }
                while let Some(message_to_send) = self.messages_to_send.pop_front() {
                    message_to_send
                        .message
                        .send_error(&self.tag, Error::DisconnectedByPeer);
                }
                return false;
            }

            if let Err(e) = self.connection.reconnect().await {
                error!("[{}] Failed to reconnect: {e:?}", self.tag);
                continue;
            }

            if self.auto_resubscribe
                && let Err(e) = self.auto_resubscribe().await
            {
                error!("[{}] Failed to reconnect: {e:?}", self.tag);
                continue;
            }

            if self.auto_remonitor
                && let Err(e) = self.auto_remonitor(old_status).await
            {
                error!("[{}] Failed to reconnect: {e:?}", self.tag);
                continue;
            }

            if let Err(e) = self.reconnect_sender.send(()) {
                debug!(
                    "[{}] Cannot send reconnect notification to clients: {e}",
                    self.tag
                );
            }

            // Restore the connection status before replaying in-flight
            // messages so that they are routed through `handle_message`,
            // exactly as fresh messages and the retry path are.
            if let Status::Monitor | Status::EnteringMonitor = old_status {
                if self.push_sender.is_some() {
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
            for message in to_replay {
                self.handle_message(message);
            }

            self.send_messages().await;

            info!("[{}] reconnected!", self.tag);
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
