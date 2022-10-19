use crate::{
    resp::{cmd, Array, BulkString, Command, CommandEncoder, ResultValueExt, Value, ValueDecoder},
    sleep, tcp_connect, Config, Error, Future, IntoConfig, Result, RoleResult, SentinelConfig,
    ServerConfig, TcpStreamReader, TcpStreamWriter,
};
#[cfg(feature = "tls")]
use crate::{tcp_tls_connect, TcpTlsStreamReader, TcpTlsStreamWriter};
use futures::{SinkExt, StreamExt};
use log::{debug, error, log_enabled, Level};
use tokio_util::codec::{FramedRead, FramedWrite};

enum Streams {
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

pub struct Connection {
    config: Config,
    streams: Streams,
}

impl Connection {
    pub async fn initialize(config: Config) -> Result<Self> {
        let streams = Self::connect(&config).await?;

        Ok(Self { config, streams })
    }

    pub async fn write(&mut self, command: Command) -> Result<()> {
        debug!("Sending {command:?}");
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = match &mut self.streams {
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

    pub(crate) async fn reconnect(&mut self) -> bool {
        match Self::connect(&self.config).await {
            Ok(streams) => {
                self.streams = streams;
                true
            }
            Err(e) => {
                error!("Failed to reconnect: {:?}", e);
                false
            }
        }

        // TODO improve reconnection strategy with multiple retries
    }

    async fn connect(config: &Config) -> Result<Streams> {
        match &config.server {
            ServerConfig::Single { host, port } => {
                Self::connect_single_server(host, *port, config).await
            }
            ServerConfig::Sentinel(sentinel_config) => {
                Self::connect_with_sentinel(sentinel_config, config).await
            }
        }
    }

    async fn connect_with_addr(host: &str, port: u16) -> Result<Streams> {
        let (reader, writer) = tcp_connect(host, port).await?;
        let framed_read = FramedRead::new(reader, ValueDecoder);
        let framed_write = FramedWrite::new(writer, CommandEncoder);
        Ok(Streams::Tcp(framed_read, framed_write))
    }

    async fn connect_single_server(host: &str, port: u16, _config: &Config) -> Result<Streams> {
        #[cfg(feature = "tls")]
        if let Some(tls_config) = &_config.tls_config {
            let (reader, writer) = tcp_tls_connect(host, port, tls_config).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            Self::connect_with_addr(host, port).await
        }

        #[cfg(not(feature = "tls"))]
        Self::connect_with_addr(host, port).await
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
                    let config_for_sentinel = sentinel_instance.clone().into_config()?;

                    match Connection::initialize(config_for_sentinel).await {
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
                                            let master_config: Config;

                                            #[cfg(feature = "tls")]
                                            {
                                                master_config = Config {
                                                    server: ServerConfig::Single {
                                                        host: master_host,
                                                        port: master_port,
                                                    },
                                                    tls_config: _config.tls_config.clone(),
                                                    ..Default::default()
                                                };
                                            }

                                            #[cfg(not(feature = "tls"))]
                                            {
                                                master_config = Config {
                                                    server: ServerConfig::Single {
                                                        host: master_host,
                                                        port: master_port,
                                                    },
                                                    ..Default::default()
                                                };
                                            }

                                            let mut master_connection =
                                                Self::initialize(master_config).await?;

                                            let role: RoleResult = master_connection.role().await?;

                                            if let RoleResult::Master {
                                                master_replication_offset: _,
                                                replica_infos: _,
                                            } = role
                                            {
                                                return Ok(master_connection.streams);
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

    async fn send(&mut self, command: Command) -> Result<Value> {
        self.write(command).await?;

        self.read()
            .await
            .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
            .into_result()
    }

    async fn role(&mut self) -> Result<RoleResult> {
        self.send(cmd("ROLE")).await?.into()
    }

    async fn sentinel_get_master_addr_by_name<N: Into<BulkString>>(
        &mut self,
        master_name: N,
    ) -> Result<Option<(String, u16)>> {
        self.send(
            cmd("SENTINEL")
                .arg("GET-MASTER-ADDR-BY-NAME")
                .arg(master_name),
        )
        .await?
        .into()
    }
}
