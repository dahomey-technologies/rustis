use crate::{Client, Config, ConnectionCommands, Error, Future, IntoConfig, Result};
use bb8::ManageConnection;

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
