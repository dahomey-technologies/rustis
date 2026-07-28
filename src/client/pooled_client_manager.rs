use crate::{
    Error, Result,
    client::{Client, Config, IntoConfig},
    commands::ConnectionCommands,
};
use bb8::ManageConnection;

/// An object which manages a pool of clients, based on [bb8](https://docs.rs/bb8/latest/bb8/)
///
/// # Connection state is not reset between borrows
///
/// What the pool hands out is a [`Client`], which multiplexes commands over one
/// connection — not a fresh connection. Anything a borrower attaches to that
/// connection (`SELECT`, `CLIENT SETNAME`, `CLIENT TRACKING`, subscriptions, an
/// open `WATCH`) is still there for the next borrower, and would equally be there
/// for a concurrent clone of the same client. Callers that need a clean slate
/// must issue [`reset`](crate::commands::ConnectionCommands::reset) themselves;
/// the pool does not do it for them, because a per-borrow round-trip would be
/// paid by every user and would tear down the pub/sub state of the ones using it.
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

    /// A client whose network task has ended can never answer again, so it must
    /// leave the pool instead of being handed to the next borrower.
    fn has_broken(&self, client: &mut Client) -> bool {
        client.is_network_task_finished()
    }
}
