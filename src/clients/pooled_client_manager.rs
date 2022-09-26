use crate::{Client, Error, Future, ConnectionCommands, ClientCommandResult};
use bb8::ManageConnection;

pub struct PooledClientManager {
    addr: String,
}

impl PooledClientManager {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
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
        let addr = self.addr.clone();
        Box::pin(async move { Client::connect(addr).await })
    }

    fn is_valid<'s, 'c, 'a>(&'s self, client: &'c mut Client) -> Future<'a, ()>
    where
        's: 'a,
        'c: 'a,
        Self: 'a,
    {
        Box::pin(async move { 
            client.ping(Default::default()).send().await?;
            Ok(())
        })
    }

    fn has_broken(&self, _client: &mut Client) -> bool {
        false
    }
}
