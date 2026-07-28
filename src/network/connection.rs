use crate::{
    ClusterConnection, ConnectionState, Error, Future, Result, RetryReason, SentinelConnection,
    StandaloneConnection,
    client::{Config, PreparedCommand, ServerConfig},
    commands::InternalPubSubCommands,
    resp::{Command, RespResponse},
};
use serde::de::DeserializeOwned;
use std::{future::IntoFuture, sync::Arc, task::Poll};

#[allow(clippy::large_enum_variant)]
pub enum Connection {
    Standalone(StandaloneConnection),
    Sentinel(SentinelConnection),
    Cluster(ClusterConnection),
}

impl Connection {
    #[inline]
    pub(crate) async fn connect(
        config: Config,
        connection_state: &mut ConnectionState,
    ) -> Result<Self> {
        match &config.server {
            ServerConfig::Standalone { host, port } => Ok(Connection::Standalone(
                StandaloneConnection::connect(host, *port, &config, connection_state).await?,
            )),
            ServerConfig::Sentinel(sentinel_config) => Ok(Connection::Sentinel(
                SentinelConnection::connect(sentinel_config, &config, connection_state).await?,
            )),
            ServerConfig::Cluster(cluster_config) => Ok(Connection::Cluster(
                ClusterConnection::connect(cluster_config, &config, connection_state).await?,
            )),
        }
    }

    #[inline]
    pub async fn feed(&mut self, command: &Command, retry_reasons: &[RetryReason]) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.feed(command, retry_reasons).await,
            Connection::Sentinel(connection) => connection.feed(command, retry_reasons).await,
            Connection::Cluster(connection) => connection.feed(command, retry_reasons).await,
        }
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.flush().await,
            Connection::Sentinel(connection) => connection.flush().await,
            Connection::Cluster(connection) => connection.flush().await,
        }
    }

    #[inline]
    pub async fn read(&mut self) -> Option<Result<RespResponse>> {
        match self {
            Connection::Standalone(connection) => connection.read().await,
            Connection::Sentinel(connection) => connection.read().await,
            Connection::Cluster(connection) => connection.read().await,
        }
    }

    #[inline]
    pub fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        match self {
            Connection::Standalone(connection) => connection.try_read(),
            Connection::Sentinel(connection) => connection.try_read(),
            Connection::Cluster(connection) => connection.try_read(),
        }
    }

    #[inline]
    pub(crate) async fn reconnect(&mut self, connection_state: &mut ConnectionState) -> Result<()> {
        match self {
            Connection::Standalone(connection) => {
                connection.reconnect(Some(connection_state)).await
            }
            Connection::Sentinel(connection) => connection.reconnect(connection_state).await,
            Connection::Cluster(connection) => connection.reconnect(connection_state).await,
        }
    }

    #[inline]
    pub async fn send(&mut self, command: &Command) -> Result<RespResponse> {
        self.feed(command, &[]).await?;
        self.flush().await?;
        self.read().await.ok_or_else(|| Error::DisconnectedByPeer)?
    }

    /// Hands the cluster variant a fresh copy of the connection-state registry, for
    /// the nodes a topology change creates from inside `feed` / `read` — paths the
    /// handler cannot lend its registry to. A no-op for the other variants, which
    /// receive it directly at (re)connect.
    pub(crate) fn sync_connection_state(&mut self, connection_state: &ConnectionState) {
        if let Connection::Cluster(connection) = self {
            connection.sync_connection_state(connection_state);
        }
    }

    pub(crate) fn tag(&self) -> Arc<str> {
        match self {
            Connection::Standalone(connection) => connection.tag(),
            Connection::Sentinel(connection) => connection.tag(),
            Connection::Cluster(connection) => connection.tag(),
        }
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, &'a mut Connection, R>
where
    R: DeserializeOwned + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    #[inline]
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let result = self.executor.send(&self.command).await?;
            result.to()
        })
    }
}

impl<'a> InternalPubSubCommands<'a> for &'a mut Connection {}
