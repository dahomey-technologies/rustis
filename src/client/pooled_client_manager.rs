use crate::{
    Error, Result,
    client::{Config, ExclusiveClient, IntoConfig},
    commands::ConnectionCommands,
};
use bb8::ManageConnection;

/// An object which manages a pool of clients, based on [bb8](https://docs.rs/bb8/latest/bb8/)
///
/// What the pool hands out is an [`ExclusiveClient`]: a borrower holds its
/// connection alone until it gives it back, so blocking commands and
/// [`watch`](crate::commands::TransactionCommands::watch) are legitimate here —
/// the block stays confined to the one borrowed connection.
///
/// # Connection state is not reset between borrows
///
/// A borrow is a connection out of the pool, not a fresh connection. Anything a
/// borrower attaches to it (`SELECT`, `CLIENT SETNAME`, `CLIENT TRACKING`,
/// subscriptions, an open `WATCH`) is still there for the next borrower.
/// Callers that need a clean slate must issue
/// [`reset`](crate::commands::ConnectionCommands::reset) themselves; the pool
/// does not do it for them, because a per-borrow round-trip would be paid by
/// every user and would tear down the pub/sub state of the ones using it.
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
    type Connection = ExclusiveClient;
    type Error = Error;

    async fn connect(&self) -> Result<ExclusiveClient> {
        let config = self.config.clone();
        ExclusiveClient::connect(config).await
    }

    async fn is_valid(&self, client: &mut ExclusiveClient) -> Result<()> {
        client.ping::<()>(()).await?;
        Ok(())
    }

    /// A client whose network task has ended can never answer again, so it must
    /// leave the pool instead of being handed to the next borrower.
    fn has_broken(&self, client: &mut ExclusiveClient) -> bool {
        client.inner().is_network_task_finished()
    }
}
