use crate::{
    ClusterConnection, Error, Future, Result, RetryReason, SentinelConnection,
    StandaloneConnection,
    client::{Config, PreparedCommand, ServerConfig},
    commands::InternalPubSubCommands,
    resp::{Command, RespBuf},
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
    pub async fn connect(config: Config) -> Result<Self> {
        match &config.server {
            ServerConfig::Standalone { host, port } => Ok(Connection::Standalone(
                StandaloneConnection::connect(host, *port, &config).await?,
            )),
            ServerConfig::Sentinel(sentinel_config) => Ok(Connection::Sentinel(
                SentinelConnection::connect(sentinel_config, &config).await?,
            )),
            ServerConfig::Cluster(cluster_config) => Ok(Connection::Cluster(
                ClusterConnection::connect(cluster_config, &config).await?,
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
    pub async fn read(&mut self) -> Option<Result<RespBuf>> {
        match self {
            Connection::Standalone(connection) => connection.read().await,
            Connection::Sentinel(connection) => connection.read().await,
            Connection::Cluster(connection) => connection.read().await,
        }
    }

    #[inline]
    pub fn try_read(&mut self) -> Poll<Option<Result<RespBuf>>> {
        match self {
            Connection::Standalone(connection) => connection.try_read(),
            Connection::Sentinel(connection) => connection.try_read(),
            Connection::Cluster(connection) => connection.try_read(),
        }
    }

    #[inline]
    pub async fn reconnect(&mut self) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.reconnect().await,
            Connection::Sentinel(connection) => connection.reconnect().await,
            Connection::Cluster(connection) => connection.reconnect().await,
        }
    }

    #[inline]
    pub async fn send(&mut self, command: &Command) -> Result<RespBuf> {
        self.feed(command, &[]).await?;
        self.flush().await?;
        self.read()
            .await
            .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
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
