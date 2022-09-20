use crate::{
    cmd,
    resp::{BulkString, Value, ResultValueExt},
    Command, Database, Message, PubSub, PubSubReceiver, PubSubSender, PubSubStream,
    Result, ServerEndPoint, ValueReceiver, ValueSender,
};
use futures::channel::{mpsc, oneshot};

#[derive(Clone)]
pub struct ConnectionMultiplexer {
    server_end_point: ServerEndPoint,
}

impl ConnectionMultiplexer {
    pub async fn connect(addr: impl Into<String>) -> Result<ConnectionMultiplexer> {
        let server_end_point = ServerEndPoint::connect(addr).await?;

        println!("Connected to {}", server_end_point.get_addr());
        Ok(ConnectionMultiplexer { server_end_point })
    }

    pub fn get_database(&self, db: usize) -> Database {
        Database::new(self.clone(), db)
    }

    pub fn get_default_database(&self) -> Database {
        Database::new(self.clone(), 0)
    }

    pub fn get_pub_sub(&self) -> PubSub {
        PubSub::new(self.clone())
    }

    pub(crate) async fn send(&self, database: usize, command: Command) -> Result<Value> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();

        let message = Message::new(command)
            .database(database)
            .value_sender(value_sender);

        self.server_end_point.send(message)?;

        let value = value_receiver.await?;
        value.into_result()
    }

    pub(crate) async fn send_and_forget(&self, database: usize, command: Command) -> Result<()> {
        let message = Message::new(command)
            .database(database);

        self.server_end_point.send(message)?;

        Ok(())
    }   

    pub(crate) async fn subscribe(&self, channel: BulkString) -> Result<PubSubStream> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) = mpsc::unbounded();

        let channel_name = channel.to_string();
        let message = Message::new(cmd("SUBSCRIBE").arg(channel))
            .value_sender(value_sender)
            .pub_sub_sender(pub_sub_sender);

        self.server_end_point.send(message)?;

        let value = value_receiver.await?;
        value.map_into_result(|_| PubSubStream::new(channel_name, pub_sub_receiver, self.clone()))
    }

    pub(crate) fn unsubscribe(&self, channel: BulkString) -> Result<()> {
        let message = Message::new(cmd("UNSUBSCRIBE").arg(channel));
        self.server_end_point.send(message)?;
        Ok(())
    }
}
