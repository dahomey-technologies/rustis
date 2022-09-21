use crate::{
    Connection, ConnectionFactory, ConnectionType, Error, Message, MsgReceiver, MsgSender,
    NetworkHandler, Result, cmd,
};
use futures::{select, FutureExt, StreamExt};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct InteractiveConnection {
    msg_sender: Arc<MsgSender>,
}

impl Connection for InteractiveConnection {
    fn get_msg_sender(&self) -> &MsgSender {
        &self.msg_sender
    }
}

impl InteractiveConnection {
    pub async fn connect(connection_factory: &ConnectionFactory) -> Result<InteractiveConnection> {
        let msg_sender = super::connect(connection_factory, Self::network_loop).await?;
        Ok(InteractiveConnection {
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
        let mut current_database = 0usize;

        loop {
            select! {
                msg = network_handler.msg_receiver.next().fuse() => match msg {
                    Some(msg) => {
                        if connected {
                            Self::send_message(msg, &mut network_handler, &mut current_database).await;
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
                    Some(value) => network_handler.receive_result(value),
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
        current_database: &mut usize,
    ) {
        if *current_database != msg.database {
            *current_database = msg.database;
            let select_msg = Self::create_select_database_message(msg.database);
            network_handler.send_message(select_msg).await;
        }

        network_handler.send_message(msg).await;
    }

    fn create_select_database_message(database: usize) -> Message {
        Message::new(cmd("SELECT").arg(database))
    }
}
