use super::util::RefPubSubMessage;
use crate::{
    Connection, Error, JoinHandle, ReconnectionState, Result, RetryReason,
    client::{Commands, Config, Message},
    commands::InternalPubSubCommands,
    resp::{Command, RespBuf, cmd},
    spawn, timeout,
};
use futures_channel::{mpsc, oneshot};
use futures_util::{FutureExt, SinkExt, StreamExt, select};
use log::{Level, debug, error, info, log_enabled, trace, warn};
use smallvec::SmallVec;
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};
use tokio::{sync::broadcast, time::Instant};

pub(crate) type MsgSender = mpsc::UnboundedSender<Message>;
pub(crate) type MsgReceiver = mpsc::UnboundedReceiver<Message>;
pub(crate) type ResultSender = oneshot::Sender<Result<RespBuf>>;
pub(crate) type ResultReceiver = oneshot::Receiver<Result<RespBuf>>;
pub(crate) type ResultsSender = oneshot::Sender<Result<Vec<RespBuf>>>;
pub(crate) type ResultsReceiver = oneshot::Receiver<Result<Vec<RespBuf>>>;
pub(crate) type PubSubSender = mpsc::UnboundedSender<Result<RespBuf>>;
pub(crate) type PubSubReceiver = mpsc::UnboundedReceiver<Result<RespBuf>>;
pub(crate) type PushSender = mpsc::UnboundedSender<Result<RespBuf>>;
pub(crate) type PushReceiver = mpsc::UnboundedReceiver<Result<RespBuf>>;
pub(crate) type ReconnectSender = broadcast::Sender<()>;
pub(crate) type ReconnectReceiver = broadcast::Receiver<()>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Disconnected,
    Connected,
    Subscribing,
    Subscribed,
    EnteringMonitor,
    Monitor,
    LeavingMonitor,
}

#[derive(Clone, Copy)]
enum SubscriptionType {
    Channel,
    Pattern,
    ShardChannel,
}

struct MessageToSend {
    pub message: Message,
    pub attempts: usize,
}

impl MessageToSend {
    pub fn new(message: Message) -> Self {
        Self {
            message,
            attempts: 0,
        }
    }
}

struct MessageToReceive {
    pub message: Message,
    pub num_commands: usize,
    pub attempts: usize,
}

impl MessageToReceive {
    pub fn new(message: Message, num_commands: usize, attempts: usize) -> Self {
        Self {
            message,
            num_commands,
            attempts,
        }
    }
}

struct PendingSubscription {
    pub channel_or_pattern: Vec<u8>,
    pub subscription_type: SubscriptionType,
    pub sender: PubSubSender,
    /// indicates if more subscriptions will arrive in the same batch
    pub more_to_come: bool,
}

pub(crate) struct NetworkHandler {
    status: Status,
    connection: Connection,
    /// for retries
    msg_sender: MsgSender,
    msg_receiver: MsgReceiver,
    messages_to_send: VecDeque<MessageToSend>,
    messages_to_receive: VecDeque<MessageToReceive>,
    pending_subscriptions: VecDeque<PendingSubscription>,
    pending_unsubscriptions: VecDeque<HashMap<Vec<u8>, SubscriptionType>>,
    subscriptions: HashMap<Vec<u8>, (SubscriptionType, PubSubSender)>,
    is_reply_on: bool,
    push_sender: Option<PushSender>,
    pending_replies: Option<Vec<RespBuf>>,
    reconnect_sender: ReconnectSender,
    auto_resubscribe: bool,
    auto_remonitor: bool,
    tag: String,
    reconnection_state: ReconnectionState,
}

impl NetworkHandler {
    pub async fn connect(
        config: Config,
    ) -> Result<(MsgSender, JoinHandle<()>, ReconnectSender, String)> {
        // options
        let auto_resubscribe = config.auto_resubscribe;
        let auto_remonitor = config.auto_remonitor;
        let reconnection_config = config.reconnection.clone();

        let connection = Connection::connect(config).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) = mpsc::unbounded();
        let (reconnect_sender, _): (ReconnectSender, ReconnectReceiver) = broadcast::channel(32);
        let tag = connection.tag().to_owned();

        let mut network_handler = NetworkHandler {
            status: Status::Connected,
            connection,
            msg_sender: msg_sender.clone(),
            msg_receiver,
            messages_to_send: VecDeque::new(),
            messages_to_receive: VecDeque::new(),
            pending_subscriptions: VecDeque::new(),
            pending_unsubscriptions: VecDeque::new(),
            subscriptions: HashMap::new(),
            is_reply_on: true,
            push_sender: None,
            pending_replies: None,
            reconnect_sender: reconnect_sender.clone(),
            auto_resubscribe,
            auto_remonitor,
            tag: tag.clone(),
            reconnection_state: ReconnectionState::new(reconnection_config),
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
                msg = self.msg_receiver.next().fuse() => {
                    if !self.try_handle_message(msg).await { break; }
                } ,
                result = self.connection.read().fuse() => {
                    if !self.handle_result(result).await { break; }
                }
            }
        }

        debug!("[{}] end of network loop", self.tag);
        Ok(())
    }

    async fn try_handle_message(&mut self, mut msg: Option<Message>) -> bool {
        let is_channel_closed: bool;

        loop {
            if let Some(msg) = msg {
                self.handle_message(msg).await;
            } else {
                is_channel_closed = true;
                break;
            }

            match self.msg_receiver.try_next() {
                Ok(m) => msg = m,
                Err(_) => {
                    // there are no messages available, but channel is not yet closed
                    is_channel_closed = false;
                    break;
                }
            }
        }

        if self.status != Status::Disconnected {
            self.send_messages().await
        }

        !is_channel_closed
    }

    async fn handle_message(&mut self, mut msg: Message) {
        trace!(
            "[{}][{:?}] Will handle message: {msg:?}",
            self.tag, self.status
        );
        let pub_sub_senders = msg.pub_sub_senders.take();
        if let Some(pub_sub_senders) = pub_sub_senders {
            let subscription_type = match &msg.commands {
                Commands::Single(command, _) => match command.name {
                    "SUBSCRIBE" => SubscriptionType::Channel,
                    "PSUBSCRIBE" => SubscriptionType::Pattern,
                    "SSUBSCRIBE" => SubscriptionType::ShardChannel,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            };

            for (channel_or_pattern, _sender) in pub_sub_senders.iter() {
                if self.subscriptions.contains_key(channel_or_pattern) {
                    debug!(
                        "[{}][{:?}] There is already a subscription on channel `{}`",
                        self.tag,
                        self.status,
                        String::from_utf8_lossy(channel_or_pattern)
                    );
                    msg.commands.send_error(
                        &self.tag,
                        Error::Client(
                            format!(
                                "There is already a subscription on channel `{}`",
                                String::from_utf8_lossy(channel_or_pattern)
                            )
                            .to_string(),
                        ),
                    );
                    return;
                }
            }

            let num_pending_subscriptions = pub_sub_senders.len();
            let pending_subscriptions = pub_sub_senders.into_iter().enumerate().map(
                |(index, (channel_or_pattern, sender))| PendingSubscription {
                    channel_or_pattern,
                    subscription_type,
                    sender,
                    more_to_come: index < num_pending_subscriptions - 1,
                },
            );

            self.pending_subscriptions.extend(pending_subscriptions);
        }

        let push_sender = msg.push_sender.take();
        if let Some(push_sender) = push_sender {
            debug!("[{}] Registering push_sender", self.tag);
            self.push_sender = Some(push_sender);
        }

        match &self.status {
            Status::Connected => {
                for command in &msg.commands {
                    match command.name {
                        "SUBSCRIBE" | "PSUBSCRIBE" | "SSUBSCRIBE" => {
                            self.status = Status::Subscribing;
                        }
                        "MONITOR" => {
                            self.status = Status::EnteringMonitor;
                        }
                        _ => (),
                    }
                }
                self.messages_to_send.push_back(MessageToSend::new(msg));
            }
            Status::Subscribing => {
                self.messages_to_send.push_back(MessageToSend::new(msg));
            }
            Status::Subscribed => {
                for command in &msg.commands {
                    let subscription_type = match command.name {
                        "UNSUBSCRIBE" => Some(SubscriptionType::Channel),
                        "PUNSUBSCRIBE" => Some(SubscriptionType::Pattern),
                        "SUNSUBSCRIBE" => Some(SubscriptionType::ShardChannel),
                        _ => None,
                    };
                    if let Some(subscription_type) = subscription_type {
                        self.pending_unsubscriptions.push_back(
                            (&command.args)
                                .into_iter()
                                .map(|a| (a.to_vec(), subscription_type))
                                .collect(),
                        );
                    }
                }
                self.messages_to_send.push_back(MessageToSend::new(msg));
            }
            Status::Disconnected => {
                if msg.retry_on_error {
                    debug!(
                        "[{}] network disconnected, queuing command: {:?}",
                        self.tag, msg.commands
                    );
                    self.messages_to_send.push_back(MessageToSend::new(msg));
                } else {
                    debug!(
                        "[{}] network disconnected, ending command in error: {:?}",
                        self.tag, msg.commands
                    );
                    msg.commands.send_error(
                        &self.tag,
                        Error::Client("Disconnected from server".to_string()),
                    );
                }
            }
            Status::EnteringMonitor => self.messages_to_send.push_back(MessageToSend::new(msg)),
            Status::Monitor => {
                for command in &msg.commands {
                    if command.name == "RESET" {
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
                .fold(0, |sum, msg| sum + msg.message.commands.len());
            if num_commands > 1 {
                debug!("[{}] sending batch of {} commands", self.tag, num_commands);
            }
        }

        let mut commands_to_write = SmallVec::<[&mut Command; 10]>::new();
        let mut commands_to_receive = SmallVec::<[usize; 10]>::new();
        let mut retry_reasons = SmallVec::<[RetryReason; 10]>::new();

        for message_to_send in self.messages_to_send.iter_mut() {
            let msg = &mut message_to_send.message;
            let commands = &mut msg.commands;
            let mut num_commands_to_receive: usize = 0;

            for command in commands.into_iter() {
                if command.name == "CLIENT" {
                    let mut args = (&command.args).into_iter();

                    match (args.next(), args.next()) {
                        (Some(b"REPLY"), Some(b"OFF")) => self.is_reply_on = false,
                        (Some(b"REPLY"), Some(b"SKIP")) => self.is_reply_on = false,
                        (Some(b"REPLY"), Some(b"ON")) => self.is_reply_on = true,
                        _ => (),
                    }
                }

                if self.is_reply_on {
                    num_commands_to_receive += 1;
                }

                commands_to_write.push(command);
            }

            commands_to_receive.push(num_commands_to_receive);

            let reasons = msg.retry_reasons.take();
            if let Some(reasons) = reasons {
                retry_reasons.extend(reasons);
            }
        }

        if let Err(e) = self
            .connection
            .write_batch(commands_to_write, &retry_reasons)
            .await
        {
            error!("[{}] Error while writing batch: {e}", self.tag);

            let mut idx: usize = 0;
            while let Some(msg) = self.messages_to_send.pop_front() {
                if commands_to_receive[idx] > 0 {
                    msg.message.commands.send_error(&self.tag, e.clone());
                }
                idx += 1;
            }
        } else {
            let mut idx: usize = 0;
            while let Some(msg) = self.messages_to_send.pop_front() {
                if commands_to_receive[idx] > 0 {
                    self.messages_to_receive.push_back(MessageToReceive::new(
                        msg.message,
                        commands_to_receive[idx],
                        msg.attempts,
                    ));
                }
                idx += 1;
            }
        }
    }

    async fn handle_result(&mut self, result: Option<Result<RespBuf>>) -> bool {
        match result {
            Some(result) => match self.status {
                Status::Disconnected => (),
                Status::Connected => match &result {
                    Ok(resp_buf) if resp_buf.is_push_message() => match &mut self.push_sender {
                        Some(push_sender) => {
                            if let Err(e) = push_sender.send(result).await {
                                warn!("[{}] Cannot send monitor result to caller: {e}", self.tag);
                            }
                        }
                        None => {
                            warn!(
                                "[{}] Received a push message with no sender configured: {resp_buf}",
                                self.tag
                            )
                        }
                    },
                    _ => {
                        self.receive_result(result);
                    }
                },
                Status::Subscribing => {
                    if result.is_ok() {
                        self.status = Status::Subscribed;
                    } else {
                        self.status = Status::Connected;
                    }

                    if let Some(resp_buf) = self.try_match_pubsub_message(result).await {
                        self.receive_result(resp_buf);
                    }
                }
                Status::Subscribed => {
                    if let Some(resp_buf) = self.try_match_pubsub_message(result).await {
                        self.receive_result(resp_buf);
                        if self.subscriptions.is_empty() && self.pending_subscriptions.is_empty() {
                            debug!(
                                "[{}] goint out from Pub/Sub state to connected state",
                                self.tag
                            );
                            self.status = Status::Connected;
                        }
                    }
                }
                Status::EnteringMonitor => {
                    self.receive_result(result);
                    self.status = Status::Monitor;
                }
                Status::Monitor => match &result {
                    Ok(resp_buf) if resp_buf.is_monitor_message() => {
                        if let Some(push_sender) = &mut self.push_sender
                            && let Err(e) = push_sender.send(result).await
                        {
                            warn!("[{}] Cannot send monitor result to caller: {e}", self.tag);
                        }
                    }
                    _ => self.receive_result(result),
                },
                Status::LeavingMonitor => match &result {
                    Ok(resp_buf) if resp_buf.is_monitor_message() => {
                        if let Some(push_sender) = &mut self.push_sender
                            && let Err(e) = push_sender.send(result).await
                        {
                            warn!("[{}] Cannot send monitor result to caller: {e}", self.tag);
                        }
                    }
                    _ => {
                        self.receive_result(result);
                        self.status = Status::Connected;
                    }
                },
            },
            // disconnection
            None => return self.reconnect().await,
        }

        true
    }

    fn receive_result(&mut self, result: Result<RespBuf>) {
        match self.messages_to_receive.front_mut() {
            Some(message_to_receive) => {
                if message_to_receive.num_commands == 1 || result.is_err() {
                    if let Some(mut message_to_receive) = self.messages_to_receive.pop_front() {
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
                                        Some(SmallVec::<[RetryReason; 10]>::from_iter(reasons));
                                }
                            }

                            // retry
                            let result = self.msg_sender.unbounded_send(message_to_receive.message);
                            if let Err(e) = result {
                                error!("[{}] Cannot retry message: {e}", self.tag);
                            }
                        } else {
                            trace!(
                                "[{}] Will respond to: {:?}",
                                self.tag, message_to_receive.message
                            );
                            match message_to_receive.message.commands {
                                Commands::Single(_, Some(result_sender)) => {
                                    if let Err(e) = result_sender.send(result) {
                                        warn!(
                                            "[{}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                                            self.tag
                                        );
                                    }
                                }
                                Commands::Batch(_, results_sender) => match result {
                                    Ok(resp_buf) => {
                                        let pending_replies = self.pending_replies.take();

                                        if let Some(mut pending_replies) = pending_replies {
                                            pending_replies.push(resp_buf);
                                            if let Err(e) = results_sender.send(Ok(pending_replies))
                                            {
                                                warn!(
                                                    "[{}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                                                    self.tag
                                                );
                                            }
                                        } else if let Err(e) =
                                            results_sender.send(Ok(vec![resp_buf]))
                                        {
                                            warn!(
                                                "[{}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                                                self.tag
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        if let Err(e) = results_sender.send(Err(e)) {
                                            warn!(
                                                "[{}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                                                self.tag
                                            );
                                        }
                                    }
                                },
                                Commands::None | Commands::Single(_, None) => {
                                    debug!("[{}] forget value {result:?}", self.tag)
                                    // fire & forget
                                }
                            }
                        }
                    }
                } else {
                    if self.pending_replies.is_none() {
                        self.pending_replies = Some(Vec::new());
                    }

                    if let Some(pending_replies) = &mut self.pending_replies {
                        match result {
                            Ok(value) => {
                                pending_replies.push(value);
                                message_to_receive.num_commands -= 1;
                            }
                            Err(Error::Retry(reasons)) => {
                                if let Some(retry_reasons) =
                                    &mut message_to_receive.message.retry_reasons
                                {
                                    retry_reasons.extend(reasons);
                                } else {
                                    message_to_receive.message.retry_reasons =
                                        Some(SmallVec::<[RetryReason; 10]>::from_iter(reasons));
                                }
                            }
                            _ => (),
                        }
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

    async fn try_match_pubsub_message(
        &mut self,
        value: Result<RespBuf>,
    ) -> Option<Result<RespBuf>> {
        if let Ok(ref_value) = &value {
            if let Some(pub_sub_message) = RefPubSubMessage::from_resp(ref_value) {
                match pub_sub_message {
                    RefPubSubMessage::Message(channel_or_pattern, _)
                    | RefPubSubMessage::SMessage(channel_or_pattern, _) => {
                        match self.subscriptions.get_mut(channel_or_pattern) {
                            Some((_subscription_type, pub_sub_sender)) => {
                                if let Err(e) = pub_sub_sender.unbounded_send(value) {
                                    let error_desc = e.to_string();
                                    if let Ok(ref_value) = &e.into_inner()
                                        && let Some(
                                            RefPubSubMessage::Message(channel_or_pattern, _)
                                            | RefPubSubMessage::SMessage(channel_or_pattern, _),
                                        ) = RefPubSubMessage::from_resp(ref_value)
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
                    RefPubSubMessage::Subscribe(channel_or_pattern)
                    | RefPubSubMessage::PSubscribe(channel_or_pattern)
                    | RefPubSubMessage::SSubscribe(channel_or_pattern) => {
                        if let Some(pending_sub) = self.pending_subscriptions.pop_front() {
                            if pending_sub.channel_or_pattern == channel_or_pattern {
                                if self
                                    .subscriptions
                                    .insert(
                                        channel_or_pattern.to_vec(),
                                        (pending_sub.subscription_type, pending_sub.sender),
                                    )
                                    .is_some()
                                {
                                    return Some(Err(Error::Client(
                                        format!(
                                            "There is already a subscription on channel `{}`",
                                            String::from_utf8_lossy(channel_or_pattern)
                                        )
                                        .to_string(),
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
                        Some(Ok(RespBuf::ok()))
                    }
                    RefPubSubMessage::Unsubscribe(channel_or_pattern)
                    | RefPubSubMessage::PUnsubscribe(channel_or_pattern)
                    | RefPubSubMessage::SUnsubscribe(channel_or_pattern) => {
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
                                Some(Ok(RespBuf::ok()))
                            }
                        } else {
                            Some(value)
                        }
                    }
                    RefPubSubMessage::PMessage(pattern, channel, _) => {
                        match self.subscriptions.get_mut(pattern) {
                            Some((_subscription_type, pub_sub_sender)) => {
                                if let Err(e) = pub_sub_sender.send(value).await {
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

        while let Some(message_to_receive) = self.messages_to_receive.front() {
            if !message_to_receive.message.retry_on_error {
                if let Some(message_to_receive) = self.messages_to_receive.pop_front() {
                    message_to_receive.message.commands.send_error(
                        &self.tag,
                        Error::Client("Disconnected from server".to_string()),
                    );
                }
            } else {
                break;
            }
        }

        while let Some(message_to_send) = self.messages_to_send.front() {
            if !message_to_send.message.retry_on_error {
                if let Some(message_to_send) = self.messages_to_send.pop_front() {
                    message_to_send.message.commands.send_error(
                        &self.tag,
                        Error::Client("Disconnected from server".to_string()),
                    );
                }
            } else {
                break;
            }
        }

        loop {
            if let Some(delay) = self.reconnection_state.next_delay() {
                debug!("[{}] Waiting {delay} ms before reconnection", self.tag);

                // keep on receiving new message during the delay
                let start = Instant::now();
                let end = start.checked_add(Duration::from_millis(delay)).unwrap();
                loop {
                    let delay = end.duration_since(Instant::now());
                    let result = timeout(delay, self.msg_receiver.next().fuse()).await;
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
                    message_to_receive.message.commands.send_error(
                        &self.tag,
                        Error::Client("Disconnected from server".to_string()),
                    );
                }
                while let Some(message_to_send) = self.messages_to_send.pop_front() {
                    message_to_send.message.commands.send_error(
                        &self.tag,
                        Error::Client("Disconnected from server".to_string()),
                    );
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
                )
            }

            while let Some(message_to_receive) = self.messages_to_receive.pop_back() {
                self.messages_to_send.push_front(MessageToSend {
                    message: message_to_receive.message,
                    attempts: message_to_receive.attempts,
                });
            }

            self.send_messages().await;

            if !self.subscriptions.is_empty() {
                self.status = Status::Subscribed;
            } else if let Status::Monitor | Status::EnteringMonitor = old_status {
                if self.push_sender.is_some() {
                    self.status = Status::Monitor;
                }
            } else {
                self.status = Status::Connected;
            }

            info!("[{}] reconnected!", self.tag);
            self.reconnection_state.reset_attempts();
            return true;
        }
    }

    async fn auto_resubscribe(&mut self) -> Result<()> {
        if !self.subscriptions.is_empty() {
            for (channel_or_pattern, (subscription_type, _)) in &self.subscriptions {
                match subscription_type {
                    SubscriptionType::Channel => {
                        self.connection
                            .subscribe(channel_or_pattern.clone())
                            .await?;
                    }
                    SubscriptionType::Pattern => {
                        self.connection
                            .psubscribe(channel_or_pattern.clone())
                            .await?;
                    }
                    SubscriptionType::ShardChannel => {
                        self.connection
                            .ssubscribe(channel_or_pattern.clone())
                            .await?;
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

        if !self.pending_unsubscriptions.is_empty() {
            for mut map in self.pending_unsubscriptions.drain(..) {
                for (channel_or_pattern, subscription_type) in map.drain() {
                    match subscription_type {
                        SubscriptionType::Channel => {
                            self.connection
                                .subscribe(channel_or_pattern.clone())
                                .await?;
                        }
                        SubscriptionType::Pattern => {
                            self.connection
                                .psubscribe(channel_or_pattern.clone())
                                .await?;
                        }
                        SubscriptionType::ShardChannel => {
                            self.connection
                                .ssubscribe(channel_or_pattern.clone())
                                .await?;
                        }
                    }

                    self.subscriptions.remove(&channel_or_pattern);
                }
            }
        }

        Ok(())
    }

    async fn auto_remonitor(&mut self, old_status: Status) -> Result<()> {
        if let Status::Monitor | Status::EnteringMonitor = old_status {
            self.connection.send(&cmd("MONITOR")).await?;
        }

        Ok(())
    }
}
