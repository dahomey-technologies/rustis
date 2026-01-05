use crate::{
    Error, Result,
    client::{Client, Config, IntoConfig},
    commands::ConnectionCommands,
};
use bb8::ManageConnection;

/// An object which manages a pool of clients, based on [bb8](https://docs.rs/bb8/latest/bb8/)
pub struct PooledClientManager {
    config: Config,
}

impl PooledClientManager {
    pub fn new(config: impl IntoConfig) -> Result<Self> {
        Ok(Self {
            config: config.into_config()?,
        })
    }
}

impl ManageConnection for PooledClientManager {
    type Connection = Client;
    type Error = Error;

    async fn connect(&self) -> Result<Client> {
        let config = self.config.clone();
        Client::connect(config).await
    }

    async fn is_valid(&self, client: &mut Client) -> Result<()> {
        client.ping::<()>(()).await?;
        Ok(())
    }

    fn has_broken(&self, _client: &mut Client) -> bool {
        false
    }
}
