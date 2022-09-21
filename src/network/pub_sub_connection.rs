use crate::{
    resp::{Array, BulkString, Value},
    Connection, ConnectionFactory, ConnectionType, Error, Message, MsgReceiver, MsgSender,
    NetworkHandler, PubSubSender, Result,
};
use futures::{select, stream::StreamExt, FutureExt, SinkExt};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct PubSubConnection {
    msg_sender: Arc<MsgSender>,
}

impl Connection for PubSubConnection {
    fn get_msg_sender(&self) -> &MsgSender {
        &self.msg_sender
    }
}

impl PubSubConnection {
    pub async fn connect(connection_factory: &ConnectionFactory) -> Result<PubSubConnection> {
        let msg_sender = super::connect(connection_factory, Self::network_loop).await?;
        Ok(PubSubConnection {
            msg_sender: Arc::new(msg_sender),
        })
    }

    async fn network_loop(
        connection_factory: ConnectionFactory,
        msg_receiver: MsgReceiver,
    ) -> Result<()> {
        let mut network_handler = NetworkHandler::initialize(
            connection_factory,
            ConnectionType::Interactive,
            msg_receiver,
        )
        .await?;
        let mut connected = true;
        let mut subscriptions: HashMap<Vec<u8>, PubSubSender> = HashMap::new();

        loop {
            select! {
                msg = network_handler.msg_receiver.next().fuse() => match msg {
                    Some(msg) => {
                        if connected {
                            Self::send_message(msg, &mut network_handler, &mut subscriptions).await;
                        } else {
                            let value_sender = msg.value_sender;
                            if let Some(value_sender) = value_sender {
                                let _result = value_sender.send(Err(Error::Network("Disconnected from server".to_string())));
                            }
                        }
                    },
                    None => break,
                },
                value = network_handler.framed_read.next().fuse() => match value {
                    Some(value) => Self::receive_result(value, &mut network_handler, &mut subscriptions).await?,
                    // disconnection
                    None => {
                        connected = false;
                        while let Some(value_sender) = network_handler.value_senders.pop_front() {
                            if let Some(value_sender) = value_sender {
                                let _result = value_sender.send(Err(Error::Network("Disconnected from server".to_string())));
                            }
                        }

                        // reconnect
                        println!("reconnecting");
                        if network_handler.reconnect().await {
                            connected = true;
                            println!("reconnected!");
                        }
                    },
                },
            }
        }

        Ok(())
    }

    pub(crate) async fn send_message(
        msg: Message,
        network_handler: &mut NetworkHandler,
        subscriptions: &mut HashMap<Vec<u8>, PubSubSender>,
    ) {
        if msg.command.name == "SUBSCRIBE" {
            let channel = msg.command.args.first().as_bytes().to_vec();
            subscriptions.insert(channel, msg.pub_sub_sender.unwrap());
        }

        let value_sender = msg.value_sender;
        let mut msg = Message::new(msg.command).database(msg.database);
        if let Some(value_sender) = value_sender {
            msg = msg.value_sender(value_sender);
        }

        network_handler.send_message(msg).await;
    }

    async fn receive_result(
        value: Result<Value>,
        network_handler: &mut NetworkHandler,
        subscriptions: &mut HashMap<Vec<u8>, PubSubSender>,
    ) -> Result<()> {
        if let Some(v) = Self::try_match_pubsub_message(value, subscriptions).await? {
            network_handler.receive_result(v);
        }

        Ok(())
    }

    async fn try_match_pubsub_message(
        value: Result<Value>,
        subscriptions: &mut HashMap<Vec<u8>, PubSubSender>,
    ) -> Result<Option<Result<Value>>> {
        // first pass check if received value if a PubSub message with matching on references
        let is_pub_sub_message = match value {
            Ok(Value::Array(Array::Vec(ref items))) => match &items[..] {
                [Value::BulkString(BulkString::Binary(command)), Value::BulkString(BulkString::Binary(_)), Value::BulkString(BulkString::Binary(_))] => {
                    command.as_slice() == b"message"
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

        println!("[{:?}] Received result {:?}", ConnectionType::PubSub, value);

        // second pass, move payload into pub_sub_sender by consuming received value
        if let Ok(Value::Array(Array::Vec(mut items))) = value {
            if let (
                Some(Value::BulkString(payload)),
                Some(Value::BulkString(BulkString::Binary(channel))),
                Some(Value::BulkString(BulkString::Binary(_command))),
            ) = (items.pop(), items.pop(), items.pop())
            {
                match subscriptions.get_mut(&channel) {
                    Some(pub_sub_sender) => {
                        pub_sub_sender.send(Ok(payload)).await?;
                        return Ok(None);
                    }
                    None => {
                        return Err(Error::Internal(format!(
                            "Unexpected message on channel: {:?}",
                            channel
                        )));
                    }
                }
            }
        }

        Err(Error::Internal("Should not reach this point".to_string()))
    }
}
