#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::client::TlsConfig;
use crate::{Error, Result, client::Config};
use futures_util::{Future, FutureExt};
use socket2::TcpKeepalive;
#[cfg(feature = "tokio-runtime")]
use std::sync::Arc;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tracing::{debug, info};

// `into_split` gives owned halves that share the `TcpStream` through an `Arc`
// with no per-operation lock, unlike `tokio::io::split`'s `BiLock` which is
// acquired on every read and every write. Plain TCP can use it because the two
// halves are the whole stream; TLS streams have no native split and keep
// `io::split` below.
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamReader = tokio::net::tcp::OwnedReadHalf;
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamWriter = tokio::net::tcp::OwnedWriteHalf;
#[cfg(feature = "tokio-rustls")]
pub(crate) type TcpTlsStreamReader =
    tokio::io::ReadHalf<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>;
#[cfg(feature = "tokio-rustls")]
pub(crate) type TcpTlsStreamWriter =
    tokio::io::WriteHalf<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>;
#[cfg(feature = "tokio-native-tls")]
pub(crate) type TcpTlsStreamReader =
    tokio::io::ReadHalf<tokio_native_tls::TlsStream<tokio::net::TcpStream>>;
#[cfg(feature = "tokio-native-tls")]
pub(crate) type TcpTlsStreamWriter =
    tokio::io::WriteHalf<tokio_native_tls::TlsStream<tokio::net::TcpStream>>;

pub(crate) async fn tcp_connect(
    host: &str,
    port: u16,
    config: &Config,
) -> Result<(TcpStreamReader, TcpStreamWriter)> {
    debug!(
        "Connecting to {host}:{port} with timeout {:?}...",
        config.connect_timeout
    );

    let reader: TcpStreamReader;
    let writer: TcpStreamWriter;

    #[cfg(feature = "tokio-runtime")]
    {
        let stream = timeout(
            config.connect_timeout,
            tokio::net::TcpStream::connect((host, port)),
        )
        .await??;

        if let Some(keep_alive) = config.keep_alive {
            socket2::SockRef::from(&stream)
                .set_tcp_keepalive(&TcpKeepalive::new().with_time(keep_alive))?;
        }

        if config.no_delay {
            stream.set_nodelay(true)?;
        }

        (reader, writer) = stream.into_split();
    }

    info!("Connected to {host}:{port}");

    Ok((reader, writer))
}

#[cfg(any(feature = "native-tls", feature = "rustls"))]
pub(crate) async fn tcp_tls_connect(
    host: &str,
    port: u16,
    tls_config: &TlsConfig,
    connect_timeout: Duration,
) -> Result<(TcpTlsStreamReader, TcpTlsStreamWriter)> {
    debug!("Connecting to {host}:{port} with timeout {connect_timeout:?}...");

    let reader: TcpTlsStreamReader;
    let writer: TcpTlsStreamWriter;

    #[cfg(feature = "tokio-runtime")]
    #[cfg(feature = "tokio-rustls")]
    {
        let stream = timeout(
            connect_timeout,
            tokio::net::TcpStream::connect((host, port)),
        )
        .await??;
        let tls_connector = tokio_rustls::TlsConnector::from(tls_config.rustls_config.clone());
        let server_name = host.to_owned().try_into()?;
        let tls_stream = tls_connector.connect(server_name, stream).await?;
        (reader, writer) = tokio::io::split(tls_stream);
    }
    #[cfg(feature = "tokio-runtime")]
    #[cfg(feature = "tokio-native-tls")]
    {
        let builder = tls_config.into_tls_connector_builder();
        let stream = timeout(
            connect_timeout,
            tokio::net::TcpStream::connect((host, port)),
        )
        .await??;
        let tls_connector: native_tls::TlsConnector = builder.build()?;
        let tls_connector = tokio_native_tls::TlsConnector::from(tls_connector);
        let tls_stream = tls_connector.connect(host, stream).await?;
        (reader, writer) = tokio::io::split(tls_stream);
    }

    info!("Connected to {host}:{port}");

    Ok((reader, writer))
}

pub(crate) enum JoinHandle<T> {
    #[cfg(feature = "tokio-runtime")]
    Tokio(tokio::task::JoinHandle<T>),
}

impl<T> JoinHandle<T> {
    /// Whether the spawned task has already completed. Non-blocking: it does not
    /// poll the task, it only reads what the runtime already knows.
    ///
    /// Only the pool needs this, to evict a client whose network task has ended.
    #[cfg(feature = "pool")]
    pub(crate) fn is_finished(&self) -> bool {
        match self {
            #[cfg(feature = "tokio-runtime")]
            JoinHandle::Tokio(join_handle) => join_handle.is_finished(),
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            #[cfg(feature = "tokio-runtime")]
            JoinHandle::Tokio(join_handle) => match join_handle.poll_unpin(cx) {
                Poll::Ready(Ok(result)) => Poll::Ready(Ok(result)),
                Poll::Ready(Err(e)) => Poll::Ready(Err(Error::TokioJoin(Arc::new(e)))),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

pub(crate) fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    #[cfg(feature = "tokio-runtime")]
    return JoinHandle::Tokio(tokio::spawn(future));
}

#[allow(dead_code)]
pub(crate) async fn sleep(duration: Duration) {
    #[cfg(feature = "tokio-runtime")]
    tokio::time::sleep(duration).await;
}

/// Await on a future for a maximum amount of time before returning an error.
#[allow(dead_code)]
pub(crate) async fn timeout<F: Future>(timeout: Duration, future: F) -> Result<F::Output> {
    #[cfg(feature = "tokio-runtime")]
    {
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| Error::Timeout)
    }
}
