use crate::{
    Error, Future, Result, RetryReason, TcpStreamReader, TcpStreamWriter,
    client::{Config, PreparedCommand},
    commands::{
        ClusterCommands, ConnectionCommands, HelloOptions, SentinelCommands, ServerCommands,
    },
    resp::{BufferDecoder, Command, CommandEncoder, RespBuf},
    tcp_connect,
};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::{TcpTlsStreamReader, TcpTlsStreamWriter, tcp_tls_connect};
use futures_util::{SinkExt, Stream, StreamExt, task::noop_waker_ref};
use log::{Level, debug, log_enabled};
use serde::de::DeserializeOwned;
use std::{
    future::IntoFuture,
    pin::Pin,
    task::{Context, Poll},
};
use tokio_util::codec::{FramedRead, FramedWrite};

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

    pub async fn write(&mut self, command: &Command) -> Result<()> {
        if log_enabled!(Level::Debug) {
            debug!("[{}] Sending command: {command}", self.tag);
        }
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn feed(&mut self, command: &Command, _retry_reasons: &[RetryReason]) -> Result<()> {
        if log_enabled!(Level::Debug) {
            debug!("[{}] Sending command: {command}", self.tag);
        }
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.feed(command).await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.feed(command).await,
        }
    }

    pub async fn flush(&mut self) -> Result<()> {
        if log_enabled!(Level::Debug) {
            debug!("[{}] Flushing...", self.tag);
        }
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.flush().await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.flush().await,
        }
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

    pub fn try_read(&mut self) -> Option<Result<RespBuf>> {
        let waker = noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let poll_result = match &mut self.streams {
            Streams::Tcp(framed_read, _) => Pin::new(framed_read).poll_next(&mut cx),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(framed_read, _) => Pin::new(framed_read).poll_next(&mut cx),
        };

        match poll_result {
            Poll::Ready(Some(result)) => {
                if log_enabled!(Level::Debug) {
                    match &result {
                        Ok(bytes) => debug!("[{}] (try_read) Received result {bytes}", self.tag),
                        Err(err) => debug!("[{}] (try_read) Received result {err:?}", self.tag),
                    }
                }
                Some(result)
            }
            Poll::Ready(None) => {
                debug!("[{}] Socket is closed", self.tag);
                None
            }
            Poll::Pending => None, // Nothing to read right now
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        self.streams = Streams::connect(&self.host, self.port, &self.config).await?;
        self.post_connect().await?;

        Ok(())
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
            self.executor.write(&self.command).await?;

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
