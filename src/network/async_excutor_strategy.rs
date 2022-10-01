use crate::Result;
use futures::Future;
use std::time::Duration;

#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamReader = tokio::io::ReadHalf<tokio::net::TcpStream>;
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamWriter = tokio::io::WriteHalf<tokio::net::TcpStream>;
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpTlsStreamReader =
    tokio::io::ReadHalf<tokio_native_tls::TlsStream<tokio::net::TcpStream>>;
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpTlsStreamWriter =
    tokio::io::WriteHalf<tokio_native_tls::TlsStream<tokio::net::TcpStream>>;

#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamReader =
    tokio_util::compat::Compat<futures::io::ReadHalf<async_std::net::TcpStream>>;
#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamWriter =
    tokio_util::compat::Compat<futures::io::WriteHalf<async_std::net::TcpStream>>;

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn tcp_connect(
    host: &str,
    port: u16,
) -> Result<(TcpStreamReader, TcpStreamWriter)> {
    println!("Connecting to {host}:{port}...");
    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    let (reader, writer) = tokio::io::split(stream);
    println!("Connected to {host}:{port}");

    Ok((reader, writer))
}

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn tcp_tls_connect(
    host: &str,
    port: u16,
    tls_connector: tokio_native_tls::native_tls::TlsConnector,
) -> Result<(TcpTlsStreamReader, TcpTlsStreamWriter)> {
    println!("Connecting to {host}:{port}...");
    let stream = tokio::net::TcpStream::connect((host, port)).await?;

    let tls_connector = tokio_native_tls::TlsConnector::from(tls_connector);
    let tls_stream = tls_connector.connect(host, stream).await?;
    let (reader, writer) = tokio::io::split(tls_stream);
    println!("Connected to {host}:{port}");

    Ok((reader, writer))
}

#[cfg(feature = "async-std-runtime")]
pub(crate) async fn tcp_connect(addr: &str) -> Result<(TcpStreamReader, TcpStreamWriter)> {
    use futures::AsyncReadExt;
    use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

    let stream = async_std::net::TcpStream::connect(addr).await?;
    let (reader, writer) = stream.split();
    let reader = reader.compat();
    let writer = writer.compat_write();

    Ok((reader, writer))
}

#[cfg(feature = "tokio-runtime")]
pub(crate) fn spawn<F, T>(future: F)
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tokio::spawn(future);
}

#[cfg(feature = "async-std-runtime")]
pub(crate) fn spawn<F, T>(future: F)
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    async_std::task::spawn(future);
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
