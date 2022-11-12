use crate::{Result};
#[cfg(feature = "tokio-runtime")]
use crate::Error;
#[cfg(feature = "tls")]
use crate::TlsConfig;
use futures::{Future, FutureExt};
use log::{debug, info};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamReader = tokio::io::ReadHalf<tokio::net::TcpStream>;
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamWriter = tokio::io::WriteHalf<tokio::net::TcpStream>;
#[cfg(feature = "tokio-tls")]
pub(crate) type TcpTlsStreamReader =
    tokio::io::ReadHalf<tokio_native_tls::TlsStream<tokio::net::TcpStream>>;
#[cfg(feature = "tokio-tls")]
pub(crate) type TcpTlsStreamWriter =
    tokio::io::WriteHalf<tokio_native_tls::TlsStream<tokio::net::TcpStream>>;

#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamReader =
    tokio_util::compat::Compat<futures::io::ReadHalf<async_std::net::TcpStream>>;
#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamWriter =
    tokio_util::compat::Compat<futures::io::WriteHalf<async_std::net::TcpStream>>;
#[cfg(feature = "async-std-tls")]
pub(crate) type TcpTlsStreamReader = tokio_util::compat::Compat<
    futures::io::ReadHalf<async_native_tls::TlsStream<async_std::net::TcpStream>>,
>;
#[cfg(feature = "async-std-tls")]
pub(crate) type TcpTlsStreamWriter = tokio_util::compat::Compat<
    futures::io::WriteHalf<async_native_tls::TlsStream<async_std::net::TcpStream>>,
>;

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn tcp_connect(
    host: &str,
    port: u16,
) -> Result<(TcpStreamReader, TcpStreamWriter)> {
    debug!("Connecting to {host}:{port}...");

    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    let (reader, writer) = tokio::io::split(stream);

    info!("Connected to {host}:{port}");

    Ok((reader, writer))
}

#[cfg(feature = "tokio-runtime")]
#[cfg(feature = "tokio-tls")]
#[cfg(feature = "tls")]
pub(crate) async fn tcp_tls_connect(
    host: &str,
    port: u16,
    tls_config: &TlsConfig,
) -> Result<(TcpTlsStreamReader, TcpTlsStreamWriter)> {
    debug!("Connecting to {host}:{port}...");

    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    let builder = tls_config.into_tls_connector_builder();
    let tls_connector: native_tls::TlsConnector = builder.build()?;
    let tls_connector = tokio_native_tls::TlsConnector::from(tls_connector);
    let tls_stream = tls_connector.connect(host, stream).await?;
    let (reader, writer) = tokio::io::split(tls_stream);

    info!("Connected to {host}:{port}");

    Ok((reader, writer))
}

#[cfg(feature = "async-std-runtime")]
pub(crate) async fn tcp_connect(
    host: &str,
    port: u16,
) -> Result<(TcpStreamReader, TcpStreamWriter)> {
    use futures::AsyncReadExt;
    use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

    debug!("Connecting to {host}:{port}...");

    let stream = async_std::net::TcpStream::connect((host, port)).await?;
    let (reader, writer) = stream.split();
    let reader = reader.compat();
    let writer = writer.compat_write();

    info!("Connected to {host}:{port}");

    Ok((reader, writer))
}

#[cfg(feature = "async-std-runtime")]
#[cfg(feature = "async-std-tls")]
#[cfg(feature = "tls")]
pub(crate) async fn tcp_tls_connect(
    host: &str,
    port: u16,
    tls_config: &TlsConfig,
) -> Result<(TcpTlsStreamReader, TcpTlsStreamWriter)> {
    use futures::AsyncReadExt;
    use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

    debug!("Connecting to {host}:{port}...");

    let stream = async_std::net::TcpStream::connect((host, port)).await?;
    let builder = tls_config.into_tls_connector_builder();
    let tls_connector: async_native_tls::TlsConnector = builder.into();
    let tls_stream = tls_connector.connect(host, stream).await?;
    let (reader, writer) = tls_stream.split();
    let reader = reader.compat();
    let writer = writer.compat_write();

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
                Poll::Ready(Err(e)) => Poll::Ready(Err(Error::Client(format!("JoinError: {e}")))),
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

#[cfg(feature = "tokio-runtime")]
pub(crate) fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    JoinHandle::Tokio(tokio::spawn(future))
}

#[cfg(feature = "async-std-runtime")]
pub(crate) fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    JoinHandle::AsyncStd(async_std::task::spawn(future))
}

#[allow(dead_code)]
#[cfg(feature = "tokio-runtime")]
pub(crate) async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[allow(dead_code)]
#[cfg(feature = "async-std-runtime")]
pub(crate) async fn sleep(duration: Duration) {
    async_std::task::sleep(duration).await;
}
