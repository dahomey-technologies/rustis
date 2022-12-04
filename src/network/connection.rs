use crate::{
    client::{Config, ServerConfig},
    resp::{Command, Value},
    ClusterConnection, Result, RetryReason, SentinelConnection, StandaloneConnection,
};

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

    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &Command>,
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
}
