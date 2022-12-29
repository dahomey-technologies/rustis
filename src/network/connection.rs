use crate::{
    client::{Config, PreparedCommand, ServerConfig},
    commands::InternalPubSubCommands,
    resp::{Command, FromValue, ResultValueExt, Value},
    ClusterConnection, Error, Future, Result, RetryReason, SentinelConnection,
    StandaloneConnection,
};
use std::future::IntoFuture;

pub enum Connection {
    Standalone(StandaloneConnection),
    Sentinel(SentinelConnection),
    Cluster(ClusterConnection),
}

impl Connection {
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

    pub async fn write(&mut self, command: &Command) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.write(command).await,
            Connection::Sentinel(connection) => connection.write(command).await,
            Connection::Cluster(connection) => connection.write(command).await,
        }
    }

    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &mut Command>,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        match self {
            Connection::Standalone(connection) => {
                connection.write_batch(commands, retry_reasons).await
            }
            Connection::Sentinel(connection) => {
                connection.write_batch(commands, retry_reasons).await
            }
            Connection::Cluster(connection) => {
                connection.write_batch(commands, retry_reasons).await
            }
        }
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        match self {
            Connection::Standalone(connection) => connection.read().await,
            Connection::Sentinel(connection) => connection.read().await,
            Connection::Cluster(connection) => connection.read().await,
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        match self {
            Connection::Standalone(connection) => connection.reconnect().await,
            Connection::Sentinel(connection) => connection.reconnect().await,
            Connection::Cluster(connection) => connection.reconnect().await,
        }
    }

    pub async fn send(&mut self, command: &Command) -> Result<Value> {
        self.write(command).await?;
        self.read()
            .await
            .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
            .into_result()
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, Connection, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.executor.send(&self.command).await?.into() })
    }
}

impl InternalPubSubCommands for Connection {}
