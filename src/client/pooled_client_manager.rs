use crate::{
    client::{Client, Config, IntoConfig},
    commands::ConnectionCommands,
    Error, Future, Result,
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

    fn connect<'s, 'a>(&'s self) -> Future<'a, Client>
    where
        's: 'a,
        Self: 'a,
    {
        let config = self.config.clone();
        Box::pin(async move { Client::connect(config).await })
    }

    fn is_valid<'s, 'c, 'a>(&'s self, client: &'c mut Client) -> Future<'a, ()>
    where
        's: 'a,
        'c: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            client.ping(Default::default()).await?;
            Ok(())
        })
    }

    fn has_broken(&self, _client: &mut Client) -> bool {
        false
    }
}
