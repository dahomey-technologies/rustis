use crate::{
    resp::{cmd, Array, BulkString, CommandArgs, ResultValueExt, Value},
    spawn, Config, Connection, Error, Message, Result,
};
use futures::{
    channel::{mpsc, oneshot},
    select, FutureExt, SinkExt, StreamExt,
};
use std::collections::{HashMap, VecDeque};

pub(crate) type MsgSender = mpsc::UnboundedSender<Message>;
pub(crate) type MsgReceiver = mpsc::UnboundedReceiver<Message>;
pub(crate) type ValueSender = oneshot::Sender<Result<Value>>;
pub(crate) type ValueReceiver = oneshot::Receiver<Result<Value>>;
pub(crate) type PubSubSender = mpsc::UnboundedSender<Result<Value>>;
pub(crate) type PubSubReceiver = mpsc::UnboundedReceiver<Result<Value>>;

enum Status {
    Disconnected,
    Connected,
    Subscribing,
    Subscribed,
}

pub(crate) struct NetworkHandler {
    config: Config,
    status: Status,
    connection: Connection,
    msg_receiver: MsgReceiver,
    value_senders: VecDeque<Option<ValueSender>>,
    pending_subscriptions: HashMap<Vec<u8>, PubSubSender>,
    /// for each UNSUBSCRIBE/PUNSUBSCRIBE message, the number of channels/patterns to unscribe from
    pending_unsubscriptions: VecDeque<usize>,
    subscriptions: HashMap<Vec<u8>, PubSubSender>,
    is_reply_on: bool,
}

impl NetworkHandler {
    pub async fn connect(config: Config) -> Result<MsgSender> {
        let connection = Connection::initialize(config.clone()).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) = mpsc::unbounded();
        let value_senders = VecDeque::new();

        let mut network_handler = NetworkHandler {
            config,
            status: Status::Connected,
            connection,
            msg_receiver,
            value_senders,
            pending_subscriptions: HashMap::new(),
            pending_unsubscriptions: VecDeque::new(),
            subscriptions: HashMap::new(),
            is_reply_on: true,
        };

        spawn(async move {
            if let Err(e) = network_handler.network_loop().await {
                eprintln!("{}", e);
            }
        });

        Ok(msg_sender)
    }

    async fn post_connect(&mut self) -> Result<()> {
        self.authenticate().await?;
        self.select().await?;

        Ok(())
    }

    async fn authenticate(&mut self) -> Result<()> {
        if let Some(password) = &self.config.password {
            let mut command = cmd("AUTH");

            if let Some(username) = &self.config.username {
                command = command.arg(username.clone());
            }

            command = command.arg(password.clone());

            self.connection.write(command).await?;
            self.connection
                .read()
                .await
                .ok_or(Error::Internal("Disconnected".to_owned()))?
                .into_result()?;
        }

        Ok(())
    }

    /// Select default database
    async fn select(&mut self) -> Result<()> {
        if self.config.database != 0 {
            let command = cmd("SELECT").arg(self.config.database);

            self.connection.write(command).await?;
            self.connection
                .read()
                .await
                .ok_or(Error::Internal("Disconnected".to_owned()))?
                .into_result()?;
        }

        Ok(())
    }

    async fn network_loop(&mut self) -> Result<()> {
        self.post_connect().await?;

        loop {
            select! {
                msg = self.msg_receiver.next().fuse() => if !self.handle_message(msg).await { break; },
                value = self.connection.read().fuse() => self.handle_result(value).await?,
            }
        }

        println!("end of network loop");
        Ok(())
    }

    async fn handle_message(&mut self, msg: Option<Message>) -> bool {
        if let Some(mut msg) = msg {
            let pub_sub_senders = msg.pub_sub_senders.take();
            if let Some(pub_sub_senders) = pub_sub_senders {
                self.pending_subscriptions.extend(pub_sub_senders);
            }

            match &self.status {
                Status::Connected => {
                    match msg.command.name {
                        "SUBSCRIBE" | "PSUBSCRIBE" => {
                            self.status = Status::Subscribing;
                        }
                        "CLIENT" => match &msg.command.args {
                            CommandArgs::Array2(args)
                                if args[0].as_bytes() == b"REPLY"
                                    && (args[1].as_bytes() == b"OFF"
                                        || args[1].as_bytes() == b"SKIP") =>
                            {
                                self.is_reply_on = false
                            }
                            CommandArgs::Array2(args)
                                if args[0].as_bytes() == b"REPLY"
                                    && args[1].as_bytes() == b"ON" =>
                            {
                                self.is_reply_on = true
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                    self.send_message(msg).await;
                }
                Status::Subscribing => {
                    self.send_message(msg).await;
                }
                Status::Subscribed => {
                    if let "UNSUBSCRIBE" | "PUNSUBSCRIBE" = msg.command.name {
                        self.pending_unsubscriptions
                            .push_back(msg.command.args.len());
                    }
                    self.send_message(msg).await;
                }
                Status::Disconnected => {
                    let value_sender = msg.value_sender;
                    if let Some(value_sender) = value_sender {
                        let _result = value_sender
                            .send(Err(Error::Network("Disconnected from server".to_string())));
                    }
                }
            }
            true
        } else {
            false
        }
    }

    async fn send_message(&mut self, msg: Message) {
        let command = msg.command;
        let value_sender = msg.value_sender;
        match (self.is_reply_on, self.connection.write(command).await) {
            (true, _) => self.value_senders.push_back(value_sender),
            (false, _) => (),
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
            },
            // disconnection
            None => {
                self.status = Status::Disconnected;
                // reconnect
                println!("reconnecting");
                if self.reconnect().await {
                    self.status = Status::Connected;
                    println!("reconnected!");
                }
            }
        }

        Ok(())
    }

    fn receive_result(&mut self, value: Result<Value>) {
        match self.value_senders.pop_front() {
            Some(Some(value_sender)) => {
                let _result = value_sender.send(value);
            }
            Some(None) => {
                println!("forget value {value:?}"); // fire & forget
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
            Ok(Value::Array(Array::Vec(ref items))) => match &items[..] {
                [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(channel)), _] =>
                {
                    match command.as_slice() {
                        b"message" => true,
                        b"subscribe" | b"psubscribe" => {
                            if let Some(pub_sub_sender) = self.pending_subscriptions.remove(channel)
                            {
                                self.subscriptions.insert(channel.clone(), pub_sub_sender);
                            }
                            if !self.pending_subscriptions.is_empty() {
                                return Ok(None);
                            }
                            false
                        }
                        b"unsubscribe" | b"punsubscribe" => {
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
                [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(_pattern)), Value::BulkString(BulkString::Binary(_channel)), Value::BulkString(BulkString::Binary(_payload))] => {
                    command.as_slice() == b"pmessage"
                }
                _ => false,
            },
            _ => false,
        };

        // because value is not consumed we can send it back to the caller
        // if it is not a PubSub message
        if !is_pub_sub_message {
            return Ok(Some(value));
        }

        // second pass, move payload into pub_sub_sender by consuming received value
        if let Ok(Value::Array(Array::Vec(items))) = value {
            let mut iter = items.into_iter();
            match (
                iter.next(),
                iter.next(),
                iter.next(),
                iter.next(),
                iter.next(),
            ) {
                // message
                (
                    Some(Value::BulkString(BulkString::Binary(_command))),
                    Some(Value::BulkString(BulkString::Binary(channel))),
                    Some(payload),
                    None,
                    None,
                ) => match self.subscriptions.get_mut(&channel) {
                    Some(pub_sub_sender) => {
                        pub_sub_sender
                            .send(Ok(Value::Array(Array::Vec(vec![
                                Value::BulkString(BulkString::Binary(channel)),
                                payload,
                            ]))))
                            .await?;
                        return Ok(None);
                    }
                    None => {
                        return Err(Error::Internal(format!(
                            "Unexpected message on channel: {:?}",
                            String::from_utf8(channel).unwrap()
                        )));
                    }
                },
                // pmessage
                (
                    Some(Value::BulkString(BulkString::Binary(_command))),
                    Some(Value::BulkString(BulkString::Binary(pattern))),
                    Some(channel),
                    Some(payload),
                    None,
                ) => match self.subscriptions.get_mut(&pattern) {
                    Some(pub_sub_sender) => {
                        pub_sub_sender
                            .send(Ok(Value::Array(Array::Vec(vec![
                                Value::BulkString(BulkString::Binary(pattern)),
                                channel,
                                payload,
                            ]))))
                            .await?;
                        return Ok(None);
                    }
                    None => {
                        return Err(Error::Internal(format!(
                            "Unexpected pmessage on channel: {:?}",
                            String::from_utf8(pattern).unwrap()
                        )));
                    }
                },
                _ => (),
            }
        }

        panic!("Should not reach this point");
    }

    pub(crate) async fn reconnect(&mut self) -> bool {
        while let Some(value_sender) = self.value_senders.pop_front() {
            if let Some(value_sender) = value_sender {
                let _result =
                    value_sender.send(Err(Error::Network("Disconnected from server".to_string())));
            }
        }

        if self.connection.reconnect().await {
            self.post_connect().await.is_ok()
        } else {
            false
        }
    }
}
