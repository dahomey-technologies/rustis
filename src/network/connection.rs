use crate::{
    resp::{Array, Command, CommandEncoder, FromValue, ResultValueExt, Value, ValueDecoder},
    sleep, tcp_connect, Cluster, ClusterCommands, Config, ConnectionCommands, Error, Future,
    PreparedCommand, Result, RoleResult, SentinelCommands, SentinelConfig, ServerCommands,
    ServerConfig, TcpStreamReader, TcpStreamWriter,
};
#[cfg(feature = "tls")]
use crate::{tcp_tls_connect, TcpTlsStreamReader, TcpTlsStreamWriter};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use log::{debug, log_enabled, Level};
use std::future::IntoFuture;
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{Encoder, FramedRead, FramedWrite};

pub(crate) enum Streams {
    Tcp(
        FramedRead<TcpStreamReader, ValueDecoder>,
        FramedWrite<TcpStreamWriter, CommandEncoder>,
    ),
    #[cfg(feature = "tls")]
    TcpTls(
        FramedRead<TcpTlsStreamReader, ValueDecoder>,
        FramedWrite<TcpTlsStreamWriter, CommandEncoder>,
    ),
}

impl Streams {
    pub async fn connect(host: &str, port: u16, _config: &Config) -> Result<Self> {
        #[cfg(feature = "tls")]
        if let Some(tls_config) = &_config.tls_config {
            let (reader, writer) = tcp_tls_connect(host, port, tls_config).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            Self::connect_non_secure(host, port).await
        }

        #[cfg(not(feature = "tls"))]
        Self::connect_non_secure(host, port).await
    }

    pub async fn connect_non_secure(host: &str, port: u16) -> Result<Self> {
        let (reader, writer) = tcp_connect(host, port).await?;
        let framed_read = FramedRead::new(reader, ValueDecoder);
        let framed_write = FramedWrite::new(writer, CommandEncoder);
        Ok(Streams::Tcp(framed_read, framed_write))
    }

    pub async fn write(&mut self, command: Command) -> Result<()> {
        debug!("Sending {command:?}");
        match self {
            Streams::Tcp(_, framed_write) => framed_write.send(&command).await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn write_batch(
        &mut self,
        buffer: &mut BytesMut,
        commands: impl Iterator<Item = Command>,
    ) -> Result<()> {
        let command_encoder = match self {
            Streams::Tcp(_, framed_write) => framed_write.encoder_mut(),
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.encoder_mut(),
        };

        for command in commands {
            debug!("Sending {command:?}");
            command_encoder.encode(&command, buffer)?;
        }

        match self {
            Streams::Tcp(_, framed_write) => framed_write.get_mut().write_all(buffer).await?,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.get_mut().write_all(buffer).await?,
        }

        Ok(())
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = match self {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        } {
            if log_enabled!(Level::Debug) {
                match &value {
                    Ok(Value::Array(Array::Vec(array))) => {
                        if array.len() > 100 {
                            debug!("Received result Array(Vec([...]))");
                        } else {
                            debug!("Received result {value:?}");
                        }
                    }
                    _ => debug!("Received result {value:?}"),
                }
            }
            Some(value)
        } else {
            None
        }
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, Streams, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.write(self.command).await?;

            self.executor
                .read()
                .await
                .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
                .into_result()?
                .into()
        })
    }
}

impl ServerCommands for Streams {}
impl SentinelCommands for Streams {}
impl ConnectionCommands for Streams {}

enum InnerConnection {
    Streams { streams: Streams, buffer: BytesMut },
    Cluster(Cluster),
}

pub struct Connection {
    config: Config,
    inner_connection: InnerConnection,
}

impl Connection {
    pub async fn initialize(config: Config) -> Result<Self> {
        let inner_connection = Self::connect(&config).await?;

        let mut connection = Self {
            config,
            inner_connection,
        };

        connection.post_connect().await?;

        Ok(connection)
    }

    pub async fn write(&mut self, command: Command) -> Result<()> {
        match &mut self.inner_connection {
            InnerConnection::Streams { streams, buffer: _ } => {
                streams.write(command).await?
            }
            InnerConnection::Cluster(_) => unimplemented!(),
        }

        Ok(())
    }

    pub async fn write_batch(&mut self, commands: impl Iterator<Item = Command>) -> Result<()> {
        match &mut self.inner_connection {
            InnerConnection::Streams { streams, buffer } => {
                buffer.clear();
                streams.write_batch(buffer, commands).await?;
                buffer.clear();
            }
            InnerConnection::Cluster(cluster) => cluster.write_batch(commands).await?,
        }

        Ok(())
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        match &mut self.inner_connection {
            InnerConnection::Streams { streams, buffer: _ } => streams.read().await,
            InnerConnection::Cluster(cluster) => cluster.read().await,
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        self.inner_connection = Self::connect(&self.config).await?;
        self.post_connect().await?;

        Ok(())

        // TODO improve reconnection strategy with multiple retries
    }

    async fn connect(config: &Config) -> Result<InnerConnection> {
        match &config.server {
            ServerConfig::Standalone { host, port } => Ok(InnerConnection::Streams {
                streams: Streams::connect(host, *port, config).await?,
                buffer: BytesMut::new(),
            }),
            ServerConfig::Sentinel(sentinel_config) => Ok(InnerConnection::Streams {
                streams: Self::connect_with_sentinel(sentinel_config, config).await?,
                buffer: BytesMut::new(),
            }),
            ServerConfig::Cluster(cluster_config) => Ok(InnerConnection::Cluster(
                Cluster::connect(cluster_config).await?,
            )),
        }
    }

    async fn post_connect(&mut self) -> Result<()> {
        match &mut self.inner_connection {
            InnerConnection::Streams { streams, buffer: _ } => {
                // authentication
                if let Some(ref password) = self.config.password {
                    streams
                        .auth(self.config.username.clone(), password.clone())
                        .await?;
                }

                // select database
                if self.config.database != 0 {
                    streams.select(self.config.database).await?;
                }
            }
            InnerConnection::Cluster(_) => (),
        }

        Ok(())
    }

    /// Follow `Redis service discovery via Sentinel` documentation
    /// #See <https://redis.io/docs/reference/sentinel-clients/#redis-service-discovery-via-sentinel>
    ///
    /// # Remark
    /// this function must be desugared because of async recursion:
    /// <https://doc.rust-lang.org/error-index.html#E0733>
    fn connect_with_sentinel<'a>(
        sentinel_config: &'a SentinelConfig,
        _config: &'a Config,
    ) -> Future<'a, Streams> {
        Box::pin(async move {
            let mut restart = false;
            let mut unreachable_sentinel = true;

            loop {
                for sentinel_instance in &sentinel_config.instances {
                    // Step 1: connecting to Sentinel
                    let (host, port) = sentinel_instance;

                    match Streams::connect_non_secure(host, *port).await {
                        Ok(mut sentinel_connection) => {
                            // Step 2: ask for master address
                            let result: Result<Option<(String, u16)>> = sentinel_connection
                                .sentinel_get_master_addr_by_name(
                                    sentinel_config.service_name.clone(),
                                )
                                .await;

                            match result {
                                Ok(result) => {
                                    match result {
                                        Some((master_host, master_port)) => {
                                            // Step 3: call the ROLE command in the target instance
                                            let mut master_streams = Streams::connect(
                                                &master_host,
                                                master_port,
                                                _config,
                                            )
                                            .await?;

                                            let role: RoleResult = master_streams.role().await?;

                                            if let RoleResult::Master {
                                                master_replication_offset: _,
                                                replica_infos: _,
                                            } = role
                                            {
                                                return Ok(master_streams);
                                            } else {
                                                sleep(sentinel_config.wait_beetween_failures).await;
                                                // restart from the beginning
                                                restart = true;
                                                break;
                                            }
                                        }
                                        None => {
                                            debug!(
                                                "Sentinel {}:{} does not know master `{}`",
                                                *host, *port, sentinel_config.service_name
                                            );
                                            unreachable_sentinel = false;
                                            continue;
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Cannot execute command `SENTINEL get-master-addr-by-name` with Sentinel {}:{}: {}", *host, *port, e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            debug!("Cannot connect to Sentinel {}:{} : {}", *host, *port, e);
                            continue;
                        }
                    }
                }

                if !restart {
                    break;
                } else {
                    restart = false;
                }
            }

            if unreachable_sentinel {
                Err(Error::Sentinel(
                    "All Sentinel instances are unreachable".to_owned(),
                ))
            } else {
                Err(Error::Sentinel(format!(
                    "master {} is unknown by all Sentinel instances",
                    sentinel_config.service_name
                )))
            }
        })
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, Connection, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.write(self.command).await?;

            self.executor
                .read()
                .await
                .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
                .into_result()?
                .into()
        })
    }
}

impl ServerCommands for Connection {}
impl ClusterCommands for Connection {}
