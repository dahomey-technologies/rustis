use crate::{
    client::{Commands, Config, Message},
    commands::InternalPubSubCommands,
    resp::{cmd, Command, CommandArgs, Value},
    spawn, Connection, Error, JoinHandle, Result, RetryReason,
};
use futures::{
    channel::{mpsc, oneshot},
    select, FutureExt, SinkExt, StreamExt,
};
use log::{debug, error, info, log_enabled, warn, Level};
use smallvec::SmallVec;
use std::collections::{HashMap, VecDeque};
use tokio::sync::broadcast;

pub(crate) type MsgSender = mpsc::UnboundedSender<Message>;
pub(crate) type MsgReceiver = mpsc::UnboundedReceiver<Message>;
pub(crate) type ValueSender = oneshot::Sender<Result<Value>>;
pub(crate) type ValueReceiver = oneshot::Receiver<Result<Value>>;
pub(crate) type PubSubSender = mpsc::UnboundedSender<Result<Value>>;
pub(crate) type PubSubReceiver = mpsc::UnboundedReceiver<Result<Value>>;
pub(crate) type PushSender = mpsc::UnboundedSender<Result<Value>>;
pub(crate) type PushReceiver = mpsc::UnboundedReceiver<Result<Value>>;
pub(crate) type ReconnectSender = broadcast::Sender<()>;
pub(crate) type ReconnectReceiver = broadcast::Receiver<()>;

#[derive(Clone, Copy, Debug)]
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
enum SubcriptionType {
    Channel,
    Pattern,
    ShardChannel,
}

struct MessageToSend {
    pub message: Message,
    pub retry_attempts: usize,
}

impl MessageToSend {
    pub fn new(message: Message) -> Self {
        Self {
            message,
            retry_attempts: 1,
        }
    }
}

struct MessageToReceive {
    pub message: Message,
    pub num_commands: usize,
    pub retry_attempts: usize,
}

impl MessageToReceive {
    pub fn new(message: Message, num_commands: usize, retry_attemps: usize) -> Self {
        Self {
            message,
            num_commands,
            retry_attempts: retry_attemps,
        }
    }
}

pub(crate) struct NetworkHandler {
    status: Status,
    connection: Connection,
    /// for retries
    msg_sender: MsgSender,
    msg_receiver: MsgReceiver,
    messages_to_send: VecDeque<MessageToSend>,
    messages_to_receive: VecDeque<MessageToReceive>,
    pending_subscriptions: HashMap<Vec<u8>, (SubcriptionType, PubSubSender)>,
    pending_unsubscriptions: VecDeque<HashMap<Vec<u8>, SubcriptionType>>,
    subscriptions: HashMap<Vec<u8>, (SubcriptionType, PubSubSender)>,
    is_reply_on: bool,
    push_sender: Option<PushSender>,
    pending_replies: Option<Vec<Value>>,
    reconnect_sender: ReconnectSender,
    auto_resubscribe: bool,
    auto_remonitor: bool,
    max_command_attempts: usize,
}

impl NetworkHandler {
    pub async fn connect(config: Config) -> Result<(MsgSender, JoinHandle<()>, ReconnectSender)> {
        // options
        let auto_resubscribe = config.auto_resubscribe;
        let auto_remonitor = config.auto_remonitor;
        let max_command_attempts = config.max_command_attempts;

        let connection = Connection::connect(config).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) = mpsc::unbounded();
        let (reconnect_sender, _): (ReconnectSender, ReconnectReceiver) = broadcast::channel(32);

        let mut network_handler = NetworkHandler {
            status: Status::Connected,
            connection,
            msg_sender: msg_sender.clone(),
            msg_receiver,
            messages_to_send: VecDeque::new(),
            messages_to_receive: VecDeque::new(),
            pending_subscriptions: HashMap::new(),
            pending_unsubscriptions: VecDeque::new(),
            subscriptions: HashMap::new(),
            is_reply_on: true,
            push_sender: None,
            pending_replies: None,
            reconnect_sender: reconnect_sender.clone(),
            auto_resubscribe,
            auto_remonitor,
            max_command_attempts,
        };

        let join_handle = spawn(async move {
            if let Err(e) = network_handler.network_loop().await {
                error!("network loop ended in error: {e}");
            }
        });

        Ok((msg_sender, join_handle, reconnect_sender))
    }

    async fn network_loop(&mut self) -> Result<()> {
        loop {
            select! {
                msg = self.msg_receiver.next().fuse() => {
                    debug!("self.msg_receiver.next().fuse()");
                    if !self.handle_message(msg).await { break; }
                } ,
                value = self.connection.read().fuse() => {
                    debug!("self.connection.read().fuse()");
                    self.handle_result(value).await;
                }
            }
        }

        debug!("end of network loop");
        Ok(())
    }

    async fn handle_message(&mut self, mut msg: Option<Message>) -> bool {
        let is_channel_closed: bool;

        loop {
            if let Some(mut msg) = msg {
                let pub_sub_senders = msg.pub_sub_senders.take();
                if let Some(pub_sub_senders) = pub_sub_senders {
                    let subscription_type = match &msg.commands {
                        Commands::Single(command) => match command.name {
                            "SUBSCRIBE" => SubcriptionType::Channel,
                            "PSUBSCRIBE" => SubcriptionType::Pattern,
                            "SSUBSCRIBE" => SubcriptionType::ShardChannel,
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };

                    let pending_subscriptions = pub_sub_senders
                        .into_iter()
                        .map(|(channel, sender)| (channel, (subscription_type, sender)));

                    self.pending_subscriptions.extend(pending_subscriptions);
                }

                let push_sender = msg.push_sender.take();
                if let Some(push_sender) = push_sender {
                    debug!("Registering push_sender");
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
                            if let "UNSUBSCRIBE" | "PUNSUBSCRIBE" | "SUNSUBSCRIBE" = command.name {
                                let subscription_type = match command.name {
                                    "UNSUBSCRIBE" => SubcriptionType::Channel,
                                    "PUNSUBSCRIBE" => SubcriptionType::Pattern,
                                    "SUNSUBSCRIBE" => SubcriptionType::ShardChannel,
                                    _ => unreachable!(),
                                };
                                self.pending_unsubscriptions.push_back(
                                    command
                                        .args
                                        .iter()
                                        .map(|a| (a.as_bytes().to_vec(), subscription_type))
                                        .collect(),
                                );
                            }
                        }
                        self.messages_to_send.push_back(MessageToSend::new(msg));
                    }
                    Status::Disconnected => {
                        debug!("network disconnected, queuing command: {:?}", msg.commands);
                        self.messages_to_send.push_back(MessageToSend::new(msg));
                    }
                    Status::EnteringMonitor => {
                        self.messages_to_send.push_back(MessageToSend::new(msg))
                    }
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

        if let Status::Disconnected = self.status {
        } else {
            self.send_messages().await
        }

        !is_channel_closed
    }

    async fn send_messages(&mut self) {
        if log_enabled!(Level::Debug) {
            let num_commands = self
                .messages_to_send
                .iter()
                .fold(0, |sum, msg| sum + msg.message.commands.len());
            if num_commands > 1 {
                debug!("sending batch of {} commands", num_commands);
            }
        }

        let mut commands_to_write = SmallVec::<[&Command; 10]>::new();
        let mut commands_to_receive = SmallVec::<[usize; 10]>::new();
        let mut retry_reasons = SmallVec::<[RetryReason; 10]>::new();

        for message_to_send in self.messages_to_send.iter_mut() {
            let msg = &mut message_to_send.message;
            let commands = &msg.commands;
            let mut num_commands_to_receive: usize = 0;

            for command in commands.into_iter() {
                if command.name == "CLIENT" {
                    match &command.args {
                        CommandArgs::Array2(args)
                            if args[0].as_bytes() == b"REPLY"
                                && (args[1].as_bytes() == b"OFF"
                                    || args[1].as_bytes() == b"SKIP") =>
                        {
                            self.is_reply_on = false
                        }
                        CommandArgs::Array2(args)
                            if args[0].as_bytes() == b"REPLY" && args[1].as_bytes() == b"ON" =>
                        {
                            self.is_reply_on = true
                        }
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
            .write_batch(commands_to_write.into_iter(), &retry_reasons)
            .await
        {
            error!("Error while writing batch: {e}");
        } else {
            let mut idx: usize = 0;
            while let Some(msg) = self.messages_to_send.pop_front() {
                if commands_to_receive[idx] > 0 {
                    self.messages_to_receive.push_back(MessageToReceive::new(
                        msg.message,
                        commands_to_receive[idx],
                        msg.retry_attempts,
                    ));
                }
                idx += 1;
            }
        }
    }

    async fn handle_result(&mut self, result: Option<Result<Value>>) {
        match result {
            Some(value) => match self.status {
                Status::Disconnected => (),
                Status::Connected => {
                    if let Ok(Value::Push(_)) = &value {
                        match &mut self.push_sender {
                            Some(push_sender) => {
                                if let Err(e) = push_sender.send(value).await {
                                    warn!("Cannot send monitor result to caller: {e}");
                                }
                            }
                            None => warn!(
                                "Received a push message with no sender configured: {value:?}"
                            ),
                        }
                    } else {
                        self.receive_result(value);
                    }
                }
                Status::Subscribing => {
                    if value.is_ok() {
                        self.status = Status::Subscribed;
                    } else {
                        self.status = Status::Connected;
                    }

                    if let Some(value) = self.try_match_pubsub_message(value).await {
                        self.receive_result(value);
                    }
                }
                Status::Subscribed => {
                    if let Some(value) = self.try_match_pubsub_message(value).await {
                        self.receive_result(value);
                    }
                }
                Status::EnteringMonitor => {
                    self.receive_result(value);
                    self.status = Status::Monitor;
                }
                Status::Monitor => match &value {
                    // monitor events are a SimpleString beginning by a numeric (unix timestamp)
                    Ok(Value::SimpleString(monitor_event))
                        if monitor_event.starts_with(char::is_numeric) =>
                    {
                        if let Some(push_sender) = &mut self.push_sender {
                            if let Err(e) = push_sender.send(value).await {
                                warn!("Cannot send monitor result to caller: {e}");
                            }
                        }
                    }
                    _ => self.receive_result(value),
                },
                Status::LeavingMonitor => match &value {
                    // monitor events are a SimpleString beginning by a numeric (unix timestamp)
                    Ok(Value::SimpleString(monitor_event))
                        if monitor_event.starts_with(char::is_numeric) =>
                    {
                        if let Some(push_sender) = &mut self.push_sender {
                            if let Err(e) = push_sender.send(value).await {
                                warn!("Cannot send monitor result to caller: {e}");
                            }
                        }
                    }
                    _ => {
                        self.receive_result(value);
                        self.status = Status::Connected;
                    }
                },
            },
            // disconnection
            None => self.reconnect().await,
        }
    }

    fn receive_result(&mut self, value: Result<Value>) {
        match self.messages_to_receive.front_mut() {
            Some(message_to_receive) => {
                if message_to_receive.num_commands == 1 || value.is_err() {
                    if let Some(mut message_to_receive) = self.messages_to_receive.pop_front() {
                        let mut should_retry = false;

                        if let Err(Error::Retry(_)) = &value {
                            should_retry = true;
                        } else if message_to_receive.message.retry_reasons.is_some() {
                            should_retry = true;
                        }

                        if should_retry {
                            if let Err(Error::Retry(reasons)) = value {
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
                                error!("Cannot retry message: {e}");
                            }
                        } else if let Some(value_sender) = message_to_receive.message.value_sender {
                            match value {
                                Ok(value) => {
                                    let pending_replies = self.pending_replies.take();

                                    if let Some(mut pending_replies) = pending_replies {
                                        pending_replies.push(value);
                                        if let Err(e) =
                                            value_sender.send(Ok(Value::Array(pending_replies)))
                                        {
                                            warn!("Cannot send value to caller because receiver is not there anymore: {:?}", e);
                                        }
                                    } else if let Err(e) = value_sender.send(Ok(value)) {
                                        warn!("Cannot send value to caller because receiver is not there anymore: {:?}", e);
                                    }
                                }
                                Err(_) => {
                                    if let Err(e) = value_sender.send(value) {
                                        warn!("Cannot send value to caller because receiver is not there anymore: {:?}", e);
                                    }
                                }
                            }
                        } else {
                            debug!("forget value {value:?}"); // fire & forget
                        }
                    }
                } else {
                    if self.pending_replies.is_none() {
                        self.pending_replies = Some(Vec::new());
                    }

                    if let Some(pending_replies) = &mut self.pending_replies {
                        match value {
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
                assert!(value.is_err(), "Received unexpected message: {value:?}",);
            }
        }
    }

    async fn try_match_pubsub_message(&mut self, value: Result<Value>) -> Option<Result<Value>> {
        // first pass check if received value if a PubSub message with matching on references
        let is_pub_sub_message = match value {
            Ok(Value::Array(ref items)) | Ok(Value::Push(ref items)) => {
                match &items[..] {
                    [Value::BulkString(command), Value::BulkString(channel_or_pattern), _] => {
                        match command.as_slice() {
                            b"message" | b"smessage" => true,
                            b"subscribe" | b"psubscribe" | b"ssubscribe" => {
                                if let Some(pub_sub_sender) =
                                    self.pending_subscriptions.remove(channel_or_pattern)
                                {
                                    self.subscriptions
                                        .insert(channel_or_pattern.clone(), pub_sub_sender);
                                }
                                if !self.pending_subscriptions.is_empty() {
                                    return None;
                                }
                                false
                            }
                            b"unsubscribe" | b"punsubscribe" | b"sunsubscribe" => {
                                self.subscriptions.remove(channel_or_pattern);
                                if let Some(remaining) = self.pending_unsubscriptions.front_mut() {
                                    if remaining.len() > 1 {
                                        if remaining.remove(channel_or_pattern).is_none() {
                                            error!(
                                                "Cannot find channel or pattern to remove: {}",
                                                String::from_utf8_lossy(channel_or_pattern)
                                            );
                                        }
                                        return None;
                                    } else {
                                        // last unsubscription notification received
                                        let Some(mut remaining) = self.pending_unsubscriptions.pop_front() else {
                                            error!(
                                                "Cannot find channel or pattern to remove: {}", 
                                                String::from_utf8_lossy(channel_or_pattern)
                                            );
                                            return None;
                                        };
                                        if remaining.remove(channel_or_pattern).is_none() {
                                            error!(
                                                "Cannot find channel or pattern to remove: {}",
                                                String::from_utf8_lossy(channel_or_pattern)
                                            );
                                            return None;
                                        }
                                        return Some(Ok(Value::SimpleString("OK".to_owned())));
                                    }
                                }
                                false
                            }
                            _ => false,
                        }
                    }
                    [Value::BulkString(command), Value::BulkString(_pattern), Value::BulkString(_channel), Value::BulkString(_payload)] => {
                        command.as_slice() == b"pmessage"
                    }
                    _ => false,
                }
            }
            _ => false,
        };

        // because value is not consumed we can send it back to the caller
        // if it is not a PubSub message
        if !is_pub_sub_message {
            return Some(value);
        }

        // second pass, move payload into pub_sub_sender by consuming received value
        if let Ok(Value::Array(items)) | Ok(Value::Push(items)) = value {
            let mut iter = items.into_iter();
            match (
                iter.next(),
                iter.next(),
                iter.next(),
                iter.next(),
                iter.next(),
            ) {
                // message or smessage
                (
                    Some(Value::BulkString(_command)),
                    Some(Value::BulkString(channel)),
                    Some(payload),
                    None,
                    None,
                ) => match self.subscriptions.get_mut(&channel) {
                    Some((_subscription_type, pub_sub_sender)) => {
                        if let Err(e) = pub_sub_sender
                            .send(Ok(Value::Array(vec![Value::BulkString(channel), payload])))
                            .await
                        {
                            warn!("Cannot send pub/sub message to caller: {e}");
                        }
                        return None;
                    }
                    None => {
                        error!(
                            "Unexpected message on channel '{:?}'",
                            String::from_utf8_lossy(&channel)
                        );
                        return None;
                    }
                },
                // pmessage
                (
                    Some(Value::BulkString(_command)),
                    Some(Value::BulkString(pattern)),
                    Some(Value::BulkString(channel)),
                    Some(payload),
                    None,
                ) => match self.subscriptions.get_mut(&pattern) {
                    Some((_subscription_type, pub_sub_sender)) => {
                        if let Err(e) = pub_sub_sender
                            .send(Ok(Value::Array(vec![
                                Value::BulkString(pattern),
                                Value::BulkString(channel),
                                payload,
                            ])))
                            .await
                        {
                            warn!("Cannot send pub/sub message to caller: {e}");
                        }
                        return None;
                    }
                    None => {
                        error!(
                            "Unexpected message on channel '{:?}' for pattern '{:?}'",
                            String::from_utf8_lossy(&channel),
                            String::from_utf8_lossy(&pattern)
                        );
                        return None;
                    }
                },
                _ => (),
            }
        }

        unreachable!();
    }

    async fn reconnect(&mut self) {
        let old_status = self.status;
        self.status = Status::Disconnected;

        if let Err(e) = self.connection.reconnect().await {
            while let Some(message_to_receive) = self.messages_to_receive.front() {
                if message_to_receive.retry_attempts >= self.max_command_attempts {
                    debug!(
                        "{:?}, max attempts reached",
                        message_to_receive.message.commands
                    );
                    if let Some(message_to_receive) = self.messages_to_receive.pop_front() {
                        if let Some(value_sender) = message_to_receive.message.value_sender {
                            if let Err(e) = value_sender
                                .send(Err(Error::Client("Disconnected from server".to_string())))
                            {
                                warn!(
                                    "Cannot send value to caller because receiver is not there anymore: {:?}",
                                    e
                                );
                            }
                        }
                    }
                } else {
                    break;
                }
            }

            for message_to_receive in &mut self.messages_to_receive {
                message_to_receive.retry_attempts += 1;
                debug!(
                    "{:?}, scheduling attempt {}",
                    message_to_receive.message.commands, message_to_receive.retry_attempts
                );
            }

            while let Some(message_to_send) = self.messages_to_send.front() {
                if message_to_send.retry_attempts >= self.max_command_attempts {
                    debug!(
                        "{:?}, max attempts reached",
                        message_to_send.message.commands
                    );
                    if let Some(message_to_send) = self.messages_to_send.pop_front() {
                        if let Some(value_sender) = message_to_send.message.value_sender {
                            if let Err(e) = value_sender
                                .send(Err(Error::Client("Disconnected from server".to_string())))
                            {
                                warn!(
                                    "Cannot send value to caller because receiver is not there anymore: {:?}",
                                    e
                                );
                            }
                        }
                    }
                } else {
                    break;
                }
            }

            for message_to_send in &mut self.messages_to_send {
                message_to_send.retry_attempts += 1;
                debug!(
                    "{:?}, scheduling attempt {}",
                    message_to_send.message.commands, message_to_send.retry_attempts
                );
            }

            error!("Failed to reconnect: {:?}", e);
            return;
        }

        if self.auto_resubscribe {
            if let Err(e) = self.auto_resubscribe().await {
                error!("Failed to reconnect: {:?}", e);
                return;
            }
        }

        if self.auto_remonitor {
            if let Err(e) = self.auto_remonitor(old_status).await {
                error!("Failed to reconnect: {:?}", e);
                return;
            }
        }

        if let Err(e) = self.reconnect_sender.send(()) {
            debug!("Cannot send reconnect notification to clients: {e}")
        }

        while let Some(message_to_receive) = self.messages_to_receive.pop_back() {
            debug!(
                "resending {:?}, attempt {}",
                message_to_receive.message.commands, message_to_receive.retry_attempts
            );
            self.messages_to_send.push_front(MessageToSend {
                message: message_to_receive.message,
                retry_attempts: message_to_receive.retry_attempts,
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

        info!("reconnected!");
    }

    async fn auto_resubscribe(&mut self) -> Result<()> {
        if !self.subscriptions.is_empty() {
            for (channel_or_pattern, (subscription_type, _)) in &self.subscriptions {
                match subscription_type {
                    SubcriptionType::Channel => {
                        self.connection
                            .subscribe(channel_or_pattern.clone())
                            .await?;
                    }
                    SubcriptionType::Pattern => {
                        self.connection
                            .psubscribe(channel_or_pattern.clone())
                            .await?;
                    }
                    SubcriptionType::ShardChannel => {
                        self.connection
                            .ssubscribe(channel_or_pattern.clone())
                            .await?;
                    }
                }
            }
        }

        if !self.pending_subscriptions.is_empty() {
            for (channel_or_pattern, (subscription_type, sender)) in
                self.pending_subscriptions.drain()
            {
                match subscription_type {
                    SubcriptionType::Channel => {
                        self.connection
                            .subscribe(channel_or_pattern.clone())
                            .await?;
                    }
                    SubcriptionType::Pattern => {
                        self.connection
                            .psubscribe(channel_or_pattern.clone())
                            .await?;
                    }
                    SubcriptionType::ShardChannel => {
                        self.connection
                            .ssubscribe(channel_or_pattern.clone())
                            .await?;
                    }
                }

                self.subscriptions
                    .insert(channel_or_pattern, (subscription_type, sender));
            }
        }

        if !self.pending_unsubscriptions.is_empty() {
            for mut map in self.pending_unsubscriptions.drain(..) {
                for (channel_or_pattern, subscription_type) in map.drain() {
                    match subscription_type {
                        SubcriptionType::Channel => {
                            self.connection
                                .subscribe(channel_or_pattern.clone())
                                .await?;
                        }
                        SubcriptionType::Pattern => {
                            self.connection
                                .psubscribe(channel_or_pattern.clone())
                                .await?;
                        }
                        SubcriptionType::ShardChannel => {
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
