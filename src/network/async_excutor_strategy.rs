use crate::Result;
#[cfg(feature = "tls")]
use crate::TlsConfig;
use futures::Future;
use log::{debug,info};

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