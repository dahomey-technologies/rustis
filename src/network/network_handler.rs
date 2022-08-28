use crate::{
    message::Message,
    resp::{Value, ValueDecoder},
    CommandEncoder, ConnectionFactory, ConnectionType, Error, MsgReceiver, Result, TcpStreamReader,
    TcpStreamWriter, ValueSender,
};
use futures::SinkExt;
use std::collections::VecDeque;
use tokio_util::codec::{FramedRead, FramedWrite};

pub(crate) struct NetworkHandler {
    pub connection_factory: ConnectionFactory,
    pub connection_type: ConnectionType,
    pub msg_receiver: MsgReceiver,
    pub value_senders: VecDeque<Option<ValueSender>>,
    pub framed_read: FramedRead<TcpStreamReader, ValueDecoder>,
    pub framed_write: FramedWrite<TcpStreamWriter, CommandEncoder>,
}

impl NetworkHandler {
    pub async fn initialize(
        connection_factory: ConnectionFactory,
        connection_type: ConnectionType,
        msg_receiver: MsgReceiver,
    ) -> Result<Self> {
        let value_senders = VecDeque::new();
        let (reader, writer) = connection_factory.get_connection().await?;
        let framed_read = FramedRead::new(reader, ValueDecoder);
        let framed_write = FramedWrite::new(writer, CommandEncoder);

        Ok(Self {
            connection_factory,
            connection_type,
            msg_receiver,
            value_senders,
            framed_read,
            framed_write,
        })
    }

    pub async fn send_message(&mut self, msg: Message) {
        let command = msg.command;
        let value_sender = msg.value_sender;
        println!("[{:?}] Sending {:?}", self.connection_type, command);
        match self.framed_write.send(command).await {
            Ok(()) => self.value_senders.push_back(value_sender),
            Err(_e) => self.value_senders.push_back(value_sender),
        }
    }

    pub fn receive_result(&mut self, value: Result<Value>) {
        println!("[{:?}] Received result {:?}", self.connection_type, value);

        match self.value_senders.pop_front() {
            Some(Some(value_sender)) => {
                let _ = value_sender.send(value);
            }
            Some(None) => (), // fire & forget
            None => {
                // disconnection errors could end here but ok values should match a value_sender instance
                if value.is_ok() {
                    panic!(
                        "[{:?}] Received unexpected message: {:?}",
                        self.connection_type, value
                    );
                }
            }
        }
    }

    pub(crate) async fn reconnect(&mut self) -> bool {
        while let Some(value_sender) = self.value_senders.pop_front() {
            if let Some(value_sender) = value_sender {
                let _ =
                    value_sender.send(Err(Error::Network("Disconnected from server".to_string())));
            }
        }

        // reconnect
        match self.connection_factory.get_connection().await {
            Ok((reader, writer)) => {
                self.framed_read = FramedRead::new(reader, ValueDecoder);
                self.framed_write = FramedWrite::new(writer, CommandEncoder);
                true
            }
            Err(e) => {
                println!("Failed to reconnect: {:?}", e);
                false
            }
        }
    }
}
