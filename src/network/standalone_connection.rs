use crate::{
    Error, Future, Result, RetryReason, TcpStreamReader, TcpStreamWriter,
    client::{Config, PreparedCommand},
    commands::{
        ClusterCommands, ConnectionCommands, HelloOptions, SentinelCommands, ServerCommands,
    },
    resp::{BufferDecoder, CommandEncoder, NetworkCommand, RespBuf},
    tcp_connect,
};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::{TcpTlsStreamReader, TcpTlsStreamWriter, tcp_tls_connect};
use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use log::{Level, debug, log_enabled};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::future::IntoFuture;
use tokio::io::AsyncWriteExt;
use tokio_util::codec::{Encoder, FramedRead, FramedWrite};

pub(crate) enum Streams {
    Tcp(
        FramedRead<TcpStreamReader, BufferDecoder>,
        FramedWrite<TcpStreamWriter, CommandEncoder>,
    ),
    #[cfg(any(feature = "native-tls", feature = "rustls"))]
    TcpTls(
        FramedRead<TcpTlsStreamReader, BufferDecoder>,
        FramedWrite<TcpTlsStreamWriter, CommandEncoder>,
    ),
}

impl Streams {
    pub async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        if let Some(tls_config) = &config.tls_config {
            let (reader, writer) =
                tcp_tls_connect(host, port, tls_config, config.connect_timeout).await?;
            let framed_read = FramedRead::new(reader, BufferDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            Self::connect_non_secure(host, port, config).await
        }

        #[cfg(not(any(feature = "native-tls", feature = "rustls")))]
        Self::connect_non_secure(host, port, config).await
    }

    pub async fn connect_non_secure(host: &str, port: u16, config: &Config) -> Result<Self> {
        let (reader, writer) = tcp_connect(host, port, config).await?;
        let framed_read = FramedRead::new(reader, BufferDecoder);
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
    version: String,
    tag: String,
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
            version: String::new(),
            tag: if config.connection_name.is_empty() {
                format!("{host}:{port}")
            } else {
                format!("{}:{}:{}", config.connection_name, host, port)
            },
        };

        connection.post_connect().await?;

        Ok(connection)
    }

    pub async fn write(&mut self, command: &NetworkCommand) -> Result<()> {
        if log_enabled!(Level::Debug) {
            debug!("[{}] Sending command: {command}", self.tag);
        }
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn write_batch(
        &mut self,
        commands: SmallVec<[&mut NetworkCommand; 10]>,
        _retry_reasons: &[RetryReason],
    ) -> Result<()> {
        self.buffer.clear();

        let command_encoder = match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.encoder_mut(),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.encoder_mut(),
        };

        #[cfg(debug_assertions)]
        let mut kill_connection = false;

        for command in commands {
            if log_enabled!(Level::Debug) {
                debug!("[{}] Sending command: {command}", self.tag);
            }

            #[cfg(debug_assertions)]
            if command.kill_connection_on_write > 0 {
                kill_connection = true;
                command.kill_connection_on_write -= 1;
            }

            command_encoder.encode(command, &mut self.buffer)?;
        }

        #[cfg(debug_assertions)]
        if kill_connection {
            let client_id = self.client_id().await?;
            let mut config = self.config.clone();
            "killer".clone_into(&mut config.connection_name);
            let mut connection =
                StandaloneConnection::connect(&self.host, self.port, &config).await?;
            connection
                .client_kill(crate::commands::ClientKillOptions::default().id(client_id))
                .await?;
        }

        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.get_mut().write_all(&self.buffer).await?,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => {
                framed_write.get_mut().write_all(&self.buffer).await?
            }
        }

        Ok(())
    }

    pub async fn read(&mut self) -> Option<Result<RespBuf>> {
        if let Some(result) = match &mut self.streams {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        } {
            if log_enabled!(Level::Debug) {
                match &result {
                    Ok(bytes) => debug!("[{}] Received result {bytes}", self.tag),
                    Err(err) => debug!("[{}] Received result {err:?}", self.tag),
                }
            }
            Some(result)
        } else {
            debug!("[{}] Socked is closed", self.tag);
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
        // RESP3
        let mut hello_options = HelloOptions::new(3);

        let config_username = self.config.username.clone();
        let config_password = self.config.password.clone();
        let config_connection_name = self.config.connection_name.clone();

        // authentication
        if let Some(password) = &config_password {
            hello_options = hello_options.auth(
                match &config_username {
                    Some(username) => username,
                    None => "default",
                },
                password,
            );
        }

        // connection name
        if !config_connection_name.is_empty() {
            hello_options = hello_options.set_name(&config_connection_name);
        }

        let hello_result = self.hello(hello_options).await?;
        self.version = hello_result.version;

        // select database
        if self.config.database != 0 {
            self.select(self.config.database).await?;
        }

        Ok(())
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub(crate) fn tag(&self) -> &str {
        &self.tag
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, &'a mut StandaloneConnection, R>
where
    R: DeserializeOwned + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.write(&self.command.into()).await?;

            let resp_buf = self.executor.read().await.ok_or_else(|| {
                Error::Client(format!("[{}] disconnected by peer", self.executor.tag()))
            })??;

            resp_buf.to()
        })
    }
}

impl<'a> ClusterCommands<'a> for &'a mut StandaloneConnection {}
impl<'a> ConnectionCommands<'a> for &'a mut StandaloneConnection {}
impl<'a> SentinelCommands<'a> for &'a mut StandaloneConnection {}
impl<'a> ServerCommands<'a> for &'a mut StandaloneConnection {}
