use crate::{
    ClusterConnection, ConnectionState, Endpoint, ErrorKind, Future, Result, RetryReason,
    SentinelConnection, StandaloneConnection,
    client::{Config, PreparedCommand, ServerConfig},
    commands::InternalPubSubCommands,
    resp::{Command, RespResponse},
};
use serde::de::DeserializeOwned;
use std::{future::IntoFuture, sync::Arc, task::Poll};

#[allow(clippy::large_enum_variant)]
pub(crate) enum Connection {
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
            #[cfg(unix)]
            ServerConfig::UnixSocket { path } => Ok(Connection::Standalone(
                StandaloneConnection::connect_endpoint(
                    Endpoint::Unix(path.clone()),
                    &config,
                    connection_state,
                )
                .await?,
            )),
            #[cfg(not(unix))]
            ServerConfig::UnixSocket { path: _ } => {
                Err(ErrorKind::Client(crate::ClientError::InvalidConfig(
                    "unix domain sockets are not available on this platform",
                ))
                .into())
            }
            ServerConfig::Custom(transport) => Ok(Connection::Standalone(
                StandaloneConnection::connect_endpoint(
                    Endpoint::Custom(transport.clone()),
                    &config,
                    connection_state,
                )
                .await?,
            )),
        }
    }

    #[inline]
    pub(crate) async fn feed(
        &mut self,
        command: &Command,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.feed(command, retry_reasons).await,
            Connection::Sentinel(connection) => connection.feed(command, retry_reasons).await,
            Connection::Cluster(connection) => connection.feed(command, retry_reasons).await,
        }
    }

    #[inline]
    pub(crate) async fn flush(&mut self) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.flush().await,
            Connection::Sentinel(connection) => connection.flush().await,
            Connection::Cluster(connection) => connection.flush().await,
        }
    }

    #[inline]
    pub(crate) async fn read(&mut self) -> Option<Result<RespResponse>> {
        match self {
            Connection::Standalone(connection) => connection.read().await,
            Connection::Sentinel(connection) => connection.read().await,
            Connection::Cluster(connection) => connection.read().await,
        }
    }

    #[inline]
    pub(crate) fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
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
    pub(crate) async fn send(&mut self, command: &Command) -> Result<RespResponse> {
        self.feed(command, &[]).await?;
        self.flush().await?;
        self.read()
            .await
            .ok_or_else(|| ErrorKind::DisconnectedByPeer)?
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

    /// Whether reconnecting this connection looks the master up again, rather than
    /// redialing the node it already knows.
    ///
    /// Only the sentinel variant does: its `reconnect` polls the sentinels and
    /// accepts a node only once `ROLE` confirms it is the master. So it is the only
    /// one a reconnection can repair when the node turned out to be a replica — a
    /// standalone connection would come back to the same demoted node, and a cluster
    /// one learns where the masters are from the cluster itself.
    pub(crate) fn rediscovers_master_on_reconnect(&self) -> bool {
        match self {
            Connection::Sentinel(_) => true,
            Connection::Standalone(_) | Connection::Cluster(_) => false,
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
