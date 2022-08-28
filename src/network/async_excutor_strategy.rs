use crate::{Result};
use futures::Future;

#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamReader = tokio::io::ReadHalf<tokio::net::TcpStream>;
#[cfg(feature = "tokio-runtime")]
pub(crate) type TcpStreamWriter = tokio::io::WriteHalf<tokio::net::TcpStream>;

#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamReader =
    tokio_util::compat::Compat<futures::io::ReadHalf<async_std::net::TcpStream>>;
#[cfg(feature = "async-std-runtime")]
pub(crate) type TcpStreamWriter =
    tokio_util::compat::Compat<futures::io::WriteHalf<async_std::net::TcpStream>>;

#[cfg(feature = "tokio-runtime")]
pub(crate) async fn tcp_connect(addr: &str) -> Result<(TcpStreamReader, TcpStreamWriter)> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    let (reader, writer) = tokio::io::split(stream);

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