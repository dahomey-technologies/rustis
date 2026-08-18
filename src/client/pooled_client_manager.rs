use crate::{
    Error, Result, TimeoutKind,
    client::{Config, ExclusiveClient, IntoConfig},
    commands::ConnectionCommands,
    network::timeout,
};
use bb8::ManageConnection;
use std::future::IntoFuture;

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

    /// A health check must answer, or the borrower waits on it.
    ///
    /// The ping carries its own deadline because
    /// [`command_timeout`](Config::command_timeout) defaults to none: a server
    /// that accepts the socket and then goes silent would otherwise park the
    /// check for good, and with it every caller waiting for a connection. The
    /// budget is `command_timeout` when it is set, and
    /// [`connect_timeout`](Config::connect_timeout) otherwise, that being the
    /// time the same configuration already allows for making a usable
    /// connection. Both set to zero is an explicit opt-out and is honoured.
    ///
    /// Expiry is a [`TimeoutKind::Connect`], even though the budget is usually
    /// `command_timeout` and the wait is a `PING`: the caller never issued that
    /// command, and what the failure tells them is that this pooled connection
    /// did not become usable -- it is discarded, and another is tried. The
    /// deadline's size and its meaning are separate questions.
    async fn is_valid(&self, client: &mut ExclusiveClient) -> Result<()> {
        let budget = if self.config.command_timeout.is_zero() {
            self.config.connect_timeout
        } else {
            self.config.command_timeout
        };

        if budget.is_zero() {
            client.ping::<()>(()).await?;
        } else {
            timeout(
                budget,
                TimeoutKind::Connect,
                client.ping::<()>(()).into_future(),
            )
            .await??;
        }

        Ok(())
    }

    /// A client whose network task has ended can never answer again, so it must
    /// leave the pool instead of being handed to the next borrower.
    fn has_broken(&self, client: &mut ExclusiveClient) -> bool {
        client.inner().is_terminated()
    }
}
