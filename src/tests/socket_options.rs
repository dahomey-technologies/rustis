use crate::{Result, client::Config, network::apply_socket_options};
use std::time::Duration;

/// Both the plain and the TLS connect paths must go through the same socket
/// setup, so a single test on that setup covers both.
#[cfg(feature = "tokio-runtime")]
async fn connected_stream() -> Result<(tokio::net::TcpListener, tokio::net::TcpStream)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let stream = tokio::net::TcpStream::connect(addr).await?;
    Ok((listener, stream))
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn keep_alive_and_no_delay_are_applied() -> Result<()> {
    let (_listener, stream) = connected_stream().await?;

    let config = Config {
        keep_alive: Some(Duration::from_secs(42)),
        no_delay: true,
        ..Default::default()
    };
    apply_socket_options(&stream, &config)?;

    let socket = socket2::SockRef::from(&stream);
    assert!(socket.keepalive()?);
    assert_eq!(Duration::from_secs(42), socket.tcp_keepalive_time()?);
    assert!(stream.nodelay()?);

    Ok(())
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn no_delay_disabled_leaves_nagle_on() -> Result<()> {
    let (_listener, stream) = connected_stream().await?;

    let config = Config {
        keep_alive: None,
        no_delay: false,
        ..Default::default()
    };
    apply_socket_options(&stream, &config)?;

    let socket = socket2::SockRef::from(&stream);
    assert!(!socket.keepalive()?);
    assert!(!stream.nodelay()?);

    Ok(())
}
