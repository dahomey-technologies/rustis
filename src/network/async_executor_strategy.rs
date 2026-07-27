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

#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamReader =
    tokio_util::compat::Compat<futures_util::io::ReadHalf<async_std::net::TcpStream>>;
#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamWriter =
    tokio_util::compat::Compat<futures_util::io::WriteHalf<async_std::net::TcpStream>>;
#[cfg(feature = "async-std-native-tls")]
pub(crate) type TcpTlsStreamReader = tokio_util::compat::Compat<
    futures_util::io::ReadHalf<async_native_tls::TlsStream<async_std::net::TcpStream>>,
>;
#[cfg(feature = "async-std-native-tls")]
pub(crate) type TcpTlsStreamWriter = tokio_util::compat::Compat<
    futures_util::io::WriteHalf<async_native_tls::TlsStream<async_std::net::TcpStream>>,
>;

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
    #[cfg(feature = "async-std-runtime")]
    {
        use async_std::net::TcpStream;
        use futures_util::AsyncReadExt;
        use socket2::{Domain, Protocol, Socket, Type};
        use std::net::ToSocketAddrs;
        use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

        // Bring this path to parity with the tokio one. The previous version issued
        // a synchronous socket2 `connect` from async context (stalling the executor
        // thread up to the OS TCP timeout), ignored `connect_timeout`, hardcoded a
        // 60 s keepalive, and tried only the first resolved address. The crate
        // forbids `unsafe`, so keepalive cannot be set on an `async_std::TcpStream`
        // (it exposes no `AsFd`); instead the blocking connect — which *can* set
        // keepalive and iterate every address safely — runs on a blocking thread so
        // it no longer stalls the executor, bounded by `connect_timeout`.
        let host = host.to_owned();
        let keep_alive = config.keep_alive;
        let std_stream: std::net::TcpStream = timeout(
            config.connect_timeout,
            async_std::task::spawn_blocking(move || {
                let addrs = (host.as_str(), port).to_socket_addrs()?;
                let mut last_err = None;
                for addr in addrs {
                    let socket =
                        Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
                    if let Some(keep_alive) = keep_alive {
                        socket.set_tcp_keepalive(&TcpKeepalive::new().with_time(keep_alive))?;
                    }
                    match socket.connect(&addr.into()) {
                        Ok(()) => return Ok(socket.into()),
                        Err(e) => last_err = Some(e),
                    }
                }
                Err(last_err.unwrap_or_else(|| std::io::Error::other("No address found")))
            }),
        )
        .await??;

        let stream = TcpStream::from(std_stream);

        if config.no_delay {
            stream.set_nodelay(true)?;
        }

        let (r, w) = stream.split();
        reader = r.compat();
        writer = w.compat_write();
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
    #[cfg(feature = "async-std-runtime")]
    #[cfg(feature = "async-std-native-tls")]
    {
        use futures_util::AsyncReadExt;
        use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

        let stream = timeout(
            connect_timeout,
            async_std::net::TcpStream::connect((host, port)),
        )
        .await??;
        let builder = tls_config.into_tls_connector_builder();
        let tls_connector: async_native_tls::TlsConnector = builder.into();
        let tls_stream = tls_connector.connect(host, stream).await?;
        let (r, w) = tls_stream.split();
        reader = r.compat();
        writer = w.compat_write();
    }

    info!("Connected to {host}:{port}");

    Ok((reader, writer))
}

pub enum JoinHandle<T> {
    #[cfg(feature = "tokio-runtime")]
    Tokio(tokio::task::JoinHandle<T>),
    #[cfg(feature = "async-std-runtime")]
    AsyncStd(async_std::task::JoinHandle<T>),
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
            #[cfg(feature = "async-std-runtime")]
            JoinHandle::AsyncStd(join_handle) => match join_handle.poll_unpin(cx) {
                Poll::Ready(result) => Poll::Ready(Ok(result)),
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
    #[cfg(feature = "async-std-runtime")]
    return JoinHandle::AsyncStd(async_std::task::spawn(future));
}

#[allow(dead_code)]
pub(crate) async fn sleep(duration: Duration) {
    #[cfg(feature = "tokio-runtime")]
    tokio::time::sleep(duration).await;
    #[cfg(feature = "async-std-runtime")]
    async_std::task::sleep(duration).await;
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
    #[cfg(feature = "async-std-runtime")]
    {
        // This avoids a panic on async-std when the provided duration is too large.
        // See: https://github.com/async-rs/async-std/issues/1037.
        if timeout == Duration::MAX {
            Ok(future.await)
        } else {
            async_std::future::timeout(timeout, future)
                .await
                .map_err(|_| Error::Timeout)
        }
    }
}
