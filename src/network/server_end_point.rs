use crate::{Command, Connection, InteractiveConnection, Message, PubSubConnection, Result, ConnectionFactory};

#[derive(Debug, Copy, Clone)]
pub(crate) enum ConnectionType {
    Interactive,
    PubSub,
}

#[derive(Clone)]
pub(crate) struct ServerEndPoint {
    interactive: InteractiveConnection,
    pubsub: PubSubConnection,
}

impl ServerEndPoint {
    pub async fn connect() -> Result<Self> {
        let connection_factory = ConnectionFactory::initialize("127.0.0.1:6379").await?;
        let interactive = InteractiveConnection::connect(&connection_factory).await?;
        let pubsub = PubSubConnection::connect(&connection_factory).await?;

        Ok(Self {
            interactive,
            pubsub,
        })
    }

    fn get_connection(&self, command: &Command) -> &dyn Connection {
        match command.name {
            "SUBSCRIBE" => &self.pubsub,
            "UNSUBSCRIBE" => &self.pubsub,
            "PSUBSCRIBE" => &self.pubsub,
            "PUNSUBSCRIBE" => &self.pubsub,
            _ => &self.interactive,
        }
    }

    pub fn send(&self, message: Message) -> Result<()> {
        let connection = self.get_connection(&message.command);
        connection.send(message)?;
        Ok(())
    }
}
