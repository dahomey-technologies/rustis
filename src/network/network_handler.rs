use crate::{
    resp::{Array, BulkString, Value},
    spawn, Connection, Error, Message, Result,
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
    status: Status,
    connection: Connection,
    msg_receiver: MsgReceiver,
    value_senders: VecDeque<Option<ValueSender>>,
    pending_pub_sub_sender: Option<PubSubSender>,
    subscriptions: HashMap<Vec<u8>, PubSubSender>,
}

impl NetworkHandler {
    pub async fn connect(addr: impl Into<String>) -> Result<MsgSender> {
        let connection = Connection::initialize(addr).await?;
        let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) = mpsc::unbounded();
        let value_senders = VecDeque::new();

        let mut network_handler = NetworkHandler {
            status: Status::Connected,
            connection,
            msg_receiver,
            value_senders,
            pending_pub_sub_sender: None,
            subscriptions: HashMap::new(),
        };

        spawn(async move {
            if let Err(e) = network_handler.network_loop().await {
                eprintln!("{}", e);
            }
        });

        Ok(msg_sender)
    }

    pub async fn network_loop(&mut self) -> Result<()> {
        loop {
            select! {
                msg = self.msg_receiver.next().fuse() => if !self.handle_message(msg).await { break; },
                value = self.connection.read().fuse() => self.handle_result(value).await?,
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, msg: Option<Message>) -> bool {
        if let Some(mut msg) = msg {
            let mut pub_sub_sender: Option<PubSubSender> = None;
            std::mem::swap(&mut pub_sub_sender, &mut msg.pub_sub_sender);
            self.pending_pub_sub_sender = pub_sub_sender;

            match &self.status {
                Status::Connected => {
                    if let "SUBSCRIBE" | "SSUBSCRIBE" | "PSUBSCRIBE" = msg.command.name {
                        self.status = Status::Subscribing;
                    }
                    self.send_message(msg).await;
                }
                Status::Subscribing | Status::Subscribed => match msg.command.name {
                    "SUBSCRIBE" | "SSUBSCRIBE" | "PSUBSCRIBE" | "UNSUBSCRIBE" | "SUNSUBSCRIBE"
                    | "PUNSUBSCRIBE" | "PING" | "RESET" | "QUIT" => {
                        self.send_message(msg).await;
                    }
                    _ => {
                        let value_sender = msg.value_sender;
                        if let Some(value_sender) = value_sender {
                            let _result = value_sender.send(Err(Error::Internal(format!(
                                "Command {} not allowed when connection is in subscribed state",
                                msg.command.name
                            ))));
                        }
                    }
                },
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
        match self.connection.write(command).await {
            Ok(()) => self.value_senders.push_back(value_sender),
            Err(_e) => self.value_senders.push_back(value_sender),
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
        //println!("Received result {value:?}");

        match self.value_senders.pop_front() {
            Some(Some(value_sender)) => {
                let _result = value_sender.send(value);
            }
            Some(None) => (), // fire & forget
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
                [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(channel)), _] => {
                    match command.as_slice() {
                        b"message" => true,
                        b"subscribe" => {
                            let mut pub_sub_sender: Option<PubSubSender> = None;
                            std::mem::swap(&mut pub_sub_sender, &mut self.pending_pub_sub_sender);

                            if let Some(pub_sub_sender) = pub_sub_sender {
                                self.subscriptions.insert(channel.clone(), pub_sub_sender);
                            }
                            false
                        }
                        b"unsubscribe" => {
                            self.subscriptions.remove(channel);
                            false
                        }
                        _ => false,
                    }
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
        if let Ok(Value::Array(Array::Vec(mut items))) = value {
            if let (
                Some(payload),
                Some(Value::BulkString(BulkString::Binary(channel))),
                Some(Value::BulkString(BulkString::Binary(_command))),
            ) = (items.pop(), items.pop(), items.pop())
            {
                match self.subscriptions.get_mut(&channel) {
                    Some(pub_sub_sender) => {
                        pub_sub_sender.send(Ok(payload)).await?;
                        return Ok(None);
                    }
                    None => {
                        return Err(Error::Internal(format!(
                            "Unexpected message on channel: {:?}",
                            String::from_utf8(channel).unwrap()
                        )));
                    }
                }
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

        self.connection.reconnect().await
    }
}
