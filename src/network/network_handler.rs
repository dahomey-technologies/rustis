use crate::{
    resp::{Command, CommandArgs, Value},
    spawn, Config, Connection, Error, Message, Result, RetryReason,
};
use futures::{
    channel::{mpsc, oneshot},
    select, FutureExt, SinkExt, StreamExt,
};
use log::{debug, error, info, log_enabled, Level};
use smallvec::SmallVec;
use std::collections::{HashMap, VecDeque};

pub(crate) type MsgSender = mpsc::UnboundedSender<Message>;
pub(crate) type MsgReceiver = mpsc::UnboundedReceiver<Message>;
pub(crate) type ValueSender = oneshot::Sender<Result<Value>>;
pub(crate) type ValueReceiver = oneshot::Receiver<Result<Value>>;
pub(crate) type PubSubSender = mpsc::UnboundedSender<Result<Value>>;
pub(crate) type PubSubReceiver = mpsc::UnboundedReceiver<Result<Value>>;
pub(crate) type MonitorSender = mpsc::UnboundedSender<Result<Value>>;
pub(crate) type MonitorReceiver = mpsc::UnboundedReceiver<Result<Value>>;

enum Status {
    Disconnected,
    Connected,
    Subscribing,
    Subscribed,
    EnteringMonitor,
    Monitor,
    LeavingMonitor,
}

pub(crate) struct NetworkHandler {
    status: Status,
    connection: Connection,
    /// for retries
    msg_sender: MsgSender,
    msg_receiver: MsgReceiver,
    messages_to_send: VecDeque<Message>,
    messages_to_receive: VecDeque<(Message, usize)>,
    pending_subscriptions: HashMap<Vec<u8>, PubSubSender>,
    /// for each UNSUBSCRIBE/PUNSUBSCRIBE message, the number of channels/patterns to unsubscribe from
    pending_unsubscriptions: VecDeque<usize>,
    subscriptions: HashMap<Vec<u8>, PubSubSender>,
    is_reply_on: bool,
    monitor_sender: Option<MonitorSender>,
    pending_replies: Option<Vec<Value>>,
}

impl NetworkHandler {
    pub async fn connect(config: Config) -> Result<MsgSender> {
        let connection = Connection::connect(config.clone()).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) = mpsc::unbounded();

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
            monitor_sender: None,
            pending_replies: None,
        };

        spawn(async move {
            if let Err(e) = network_handler.network_loop().await {
                error!("network loop ended in error: {e}");
            }
        });

        Ok(msg_sender)
    }

    async fn network_loop(&mut self) -> Result<()> {
        loop {
            select! {
                msg = self.msg_receiver.next().fuse() => if !self.handle_message(msg).await { break; },
                value = self.connection.read().fuse() => self.handle_result(value).await?,
            }
        }

        debug!("end of network loop");
        Ok(())
    }

    async fn handle_message(&mut self, msg: Option<Message>) -> bool {
        let is_channel_closed: bool;
        let mut msg = msg;

        loop {
            if let Some(mut msg) = msg {
                let pub_sub_senders = msg.pub_sub_senders.take();
                if let Some(pub_sub_senders) = pub_sub_senders {
                    self.pending_subscriptions.extend(pub_sub_senders);
                }

                match &self.status {
                    Status::Connected => {
                        for command in &msg.commands {
                            match command.name {
                                "SUBSCRIBE" | "PSUBSCRIBE" | "SSUBSCRIBE" => {
                                    self.status = Status::Subscribing;
                                }
                                "MONITOR" => {
                                    self.monitor_sender = msg.monitor_sender.take();
                                    self.status = Status::EnteringMonitor;
                                }
                                _ => (),
                            }
                        }
                        self.messages_to_send.push_back(msg);
                    }
                    Status::Subscribing => {
                        self.messages_to_send.push_back(msg);
                    }
                    Status::Subscribed => {
                        for command in &msg.commands {
                            if let "UNSUBSCRIBE" | "PUNSUBSCRIBE" | "SUNSUBSCRIBE" = command.name {
                                self.pending_unsubscriptions.push_back(command.args.len());
                            }
                        }
                        self.messages_to_send.push_back(msg);
                    }
                    Status::Disconnected => {
                        let value_sender = msg.value_sender;
                        if let Some(value_sender) = value_sender {
                            let _result = value_sender
                                .send(Err(Error::Client("Disconnected from server".to_string())));
                        }
                    }
                    Status::EnteringMonitor => self.messages_to_send.push_back(msg),
                    Status::Monitor => {
                        for command in &msg.commands {
                            if command.name == "RESET" {
                                self.status = Status::LeavingMonitor;
                            }
                        }
                        self.messages_to_send.push_back(msg);
                    }
                    Status::LeavingMonitor => {
                        self.messages_to_send.push_back(msg);
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

        self.send_messages().await;
        self.messages_to_send.clear();

        !is_channel_closed
    }

    async fn send_messages(&mut self) {
        if log_enabled!(Level::Debug) {
            let num_commands = self
                .messages_to_send
                .iter()
                .fold(0, |sum, msg| sum + msg.commands.len());
            if num_commands > 1 {
                debug!("sending batch of {} commands", num_commands);
            }
        }

        let mut commands_to_write = SmallVec::<[&Command; 10]>::new();
        let mut commands_to_receive = SmallVec::<[usize; 10]>::new();
        let mut retry_reasons = SmallVec::<[RetryReason; 10]>::new();

        for msg in self.messages_to_send.iter_mut() {
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
            let mut idx: usize = 0;
            while let Some(msg) = self.messages_to_send.pop_front() {
                if let Some(value_sender) = msg.value_sender {
                    if commands_to_receive[idx] > 0 {
                        let _result = value_sender.send(Err(Error::Client(e.to_string())));
                    }
                }
                idx += 1;
            }
        } else {
            let mut idx: usize = 0;
            while let Some(msg) = self.messages_to_send.pop_front() {
                if commands_to_receive[idx] > 0 {
                    self.messages_to_receive
                        .push_back((msg, commands_to_receive[idx]));
                }
                idx += 1;
            }
        }
    }

    async fn handle_result(&mut self, result: Option<Result<Value>>) -> Result<()> {
        match result {
            Some(value) => match self.status {
                Status::Disconnected => {
                    panic!("Should not happen!");
                }
                Status::Connected => {
                    self.receive_result(value);
                }
                Status::Subscribing => {
                    if value.is_ok() {
                        self.status = Status::Subscribed;
                    } else {
                        self.status = Status::Connected;
                    }

                    if let Some(value) = self.try_match_pubsub_message(value).await? {
                        self.receive_result(value);
                    }
                }
                Status::Subscribed => {
                    if let Some(value) = self.try_match_pubsub_message(value).await? {
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
                        if let Some(monitor_sender) = &mut self.monitor_sender {
                            monitor_sender.send(value).await?;
                        }
                    }
                    _ => self.receive_result(value),
                },
                Status::LeavingMonitor => match &value {
                    // monitor events are a SimpleString beginning by a numeric (unix timestamp)
                    Ok(Value::SimpleString(monitor_event))
                        if monitor_event.starts_with(char::is_numeric) =>
                    {
                        if let Some(monitor_sender) = &mut self.monitor_sender {
                            monitor_sender.send(value).await?;
                        }
                    }
                    _ => {
                        self.receive_result(value);
                        self.status = Status::Connected;
                    }
                },
            },
            // disconnection
            None => {
                self.status = Status::Disconnected;
                // reconnect
                debug!("reconnecting");
                match self.reconnect().await {
                    Ok(()) => {
                        self.status = Status::Connected;
                        info!("reconnected!");
                    }
                    Err(e) => {
                        error!("Failed to reconnect: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn receive_result(&mut self, value: Result<Value>) {
        match self.messages_to_receive.front_mut() {
            Some((msg, num_commands)) => {
                if *num_commands == 1 || value.is_err() {
                    if let Some((mut msg, _)) = self.messages_to_receive.pop_front() {
                        let mut should_retry = false;

                        if let Err(Error::Retry(_)) = &value {
                            should_retry = true;
                        } else if msg.retry_reasons.is_some() {
                            should_retry = true;
                        }

                        if should_retry {
                            if let Err(Error::Retry(reasons)) = value {
                                if let Some(retry_reasons) = &mut msg.retry_reasons {
                                    retry_reasons.extend(reasons);
                                } else {
                                    msg.retry_reasons =
                                        Some(SmallVec::<[RetryReason; 10]>::from_iter(reasons));
                                }
                            }

                            // retry
                            let result = self.msg_sender.unbounded_send(msg);
                            if let Err(e) = result {
                                error!("Cannot retry message: {e}");
                            }
                        } else if let Some(value_sender) = msg.value_sender {
                            match value {
                                Ok(value) => {
                                    let pending_replies = self.pending_replies.take();

                                    if let Some(mut pending_replies) = pending_replies {
                                        pending_replies.push(value);
                                        let _result = value_sender
                                            .send(Ok(Value::Array(Some(pending_replies))));
                                    } else {
                                        let _result = value_sender.send(Ok(value));
                                    }
                                }
                                Err(_) => {
                                    let _result = value_sender.send(value);
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
                                *num_commands -= 1;
                            }
                            Err(Error::Retry(reasons)) => {
                                if let Some(retry_reasons) = &mut msg.retry_reasons {
                                    retry_reasons.extend(reasons);
                                } else {
                                    msg.retry_reasons =
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

    async fn try_match_pubsub_message(
        &mut self,
        value: Result<Value>,
    ) -> Result<Option<Result<Value>>> {
        // first pass check if received value if a PubSub message with matching on references
        let is_pub_sub_message = match value {
            Ok(Value::Array(Some(ref items))) | Ok(Value::Push(Some(ref items))) => {
                match &items[..] {
                    [Value::BulkString(Some(command)), Value::BulkString(Some(channel)), _] =>
                    {
                        match command.as_slice() {
                            b"message" | b"smessage" => true,
                            b"subscribe" | b"psubscribe" | b"ssubscribe" => {
                                if let Some(pub_sub_sender) =
                                    self.pending_subscriptions.remove(channel)
                                {
                                    self.subscriptions.insert(channel.clone(), pub_sub_sender);
                                }
                                if !self.pending_subscriptions.is_empty() {
                                    return Ok(None);
                                }
                                false
                            }
                            b"unsubscribe" | b"punsubscribe" | b"sunsubscribe" => {
                                self.subscriptions.remove(channel);
                                if let Some(remaining) = self.pending_unsubscriptions.front_mut() {
                                    if *remaining > 1 {
                                        *remaining -= 1;
                                        return Ok(None);
                                    } else {
                                        // last unsubscription notification received
                                        self.pending_unsubscriptions.pop_front();
                                        return Ok(Some(Ok(Value::SimpleString("OK".to_owned()))));
                                    }
                                }
                                false
                            }
                            _ => false,
                        }
                    }
                    [Value::BulkString(Some(command)), Value::BulkString(Some(_pattern)), Value::BulkString(Some(_channel)), Value::BulkString(Some(_payload))] => {
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
            return Ok(Some(value));
        }

        // second pass, move payload into pub_sub_sender by consuming received value
        if let Ok(Value::Array(Some(items))) | Ok(Value::Push(Some(items))) = value {
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
                    Some(Value::BulkString(Some(_command))),
                    Some(Value::BulkString(Some(channel))),
                    Some(payload),
                    None,
                    None,
                ) => match self.subscriptions.get_mut(&channel) {
                    Some(pub_sub_sender) => {
                        pub_sub_sender
                            .send(Ok(Value::Array(Some(vec![
                                Value::BulkString(Some(channel)),
                                payload,
                            ]))))
                            .await?;
                        return Ok(None);
                    }
                    None => {
                        return Err(Error::Client(format!(
                            "Unexpected message on channel: {:?}",
                            String::from_utf8_lossy(&channel)
                        )));
                    }
                },
                // pmessage
                (
                    Some(Value::BulkString(Some(_command))),
                    Some(Value::BulkString(Some(pattern))),
                    Some(channel),
                    Some(payload),
                    None,
                ) => match self.subscriptions.get_mut(&pattern) {
                    Some(pub_sub_sender) => {
                        pub_sub_sender
                            .send(Ok(Value::Array(Some(vec![
                                Value::BulkString(Some(pattern)),
                                channel,
                                payload,
                            ]))))
                            .await?;
                        return Ok(None);
                    }
                    None => {
                        return Err(Error::Client(format!(
                            "Unexpected pmessage on channel: {:?}",
                            String::from_utf8_lossy(&pattern)
                        )));
                    }
                },
                _ => (),
            }
        }

        panic!("Should not reach this point");
    }

    pub(crate) async fn reconnect(&mut self) -> Result<()> {
        while let Some((msg, _)) = self.messages_to_receive.pop_front() {
            if let Some(value_sender) = msg.value_sender {
                let _result =
                    value_sender.send(Err(Error::Client("Disconnected from server".to_string())));
            }
        }

        self.connection.reconnect().await
    }
}
