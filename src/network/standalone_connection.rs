use crate::{
    client::{Config, PreparedCommand},
    commands::{ClusterCommands, ConnectionCommands, SentinelCommands, ServerCommands},
    resp::{Command, CommandEncoder, FromValue, ResultValueExt, Value, ValueDecoder},
    tcp_connect, Error, Future, Result, RetryReason, TcpStreamReader, TcpStreamWriter,
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
}

pub struct StandaloneConnection {
    host: String,
    port: u16,
    config: Config,
    streams: Streams,
    buffer: BytesMut,
}

impl StandaloneConnection {
    pub async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        let streams = Streams::connect(host, port, config).await?;

        let mut connection = Self {
            host: host.to_owned(),
            port,
            config: config.clone(),
            streams,
            buffer: BytesMut::new(),
        };

        connection.post_connect().await?;

        Ok(connection)
    }

    pub async fn write(&mut self, command: &Command) -> Result<()> {
        debug!("[{}:{}] Sending {command:?}", self.host, self.port);
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn write_batch(
        &mut self,
        commands: impl Iterator<Item = &Command>,
        _retry_reasons: &[RetryReason],
    ) -> Result<()> {
        self.buffer.clear();

        let command_encoder = match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.encoder_mut(),
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.encoder_mut(),
        };

        for command in commands {
            debug!("[{}:{}] Sending {command:?}", self.host, self.port);
            command_encoder.encode(command, &mut self.buffer)?;
        }

        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.get_mut().write_all(&self.buffer).await?,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => {
                framed_write.get_mut().write_all(&self.buffer).await?
            }
        }

        Ok(())
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = match &mut self.streams {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        } {
            if log_enabled!(Level::Debug) {
                match &value {
                    Ok(Value::Array(array)) => {
                        if array.len() > 100 {
                            debug!(
                                "[{}:{}] Received result Array(Vec([...]))",
                                self.host, self.port
                            );
                        } else {
                            debug!("[{}:{}] Received result {value:?}", self.host, self.port);
                        }
                    }
                    _ => debug!("[{}:{}] Received result {value:?}", self.host, self.port),
                }
            }
            Some(value)
        } else {
            None
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        self.streams = Streams::connect(&self.host, self.port, &self.config).await?;
        self.post_connect().await?;

        Ok(())

        // TODO improve reconnection strategy with multiple retries
    }

    async fn post_connect(&mut self) -> Result<()> {
        // authentication
        if let Some(ref password) = self.config.password {
            self.auth(self.config.username.clone(), password.clone())
                .await?;
        }

        // select database
        if self.config.database != 0 {
            self.select(self.config.database).await?;
        }

        Ok(())
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, StandaloneConnection, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.write(&self.command).await?;

            self.executor
                .read()
                .await
                .ok_or_else(|| Error::Client("Disconnected by peer".to_owned()))?
                .into_result()?
                .into()
        })
    }
}

impl ClusterCommands for StandaloneConnection {}
impl ConnectionCommands for StandaloneConnection {}
impl SentinelCommands for StandaloneConnection {}
impl ServerCommands for StandaloneConnection {}
