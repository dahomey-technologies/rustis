use crate::{Result, TimeoutKind, client::PooledClientManager};
use serial_test::serial;

/// The pool pings a connection before handing it out. Under the default
/// `command_timeout = 0` that ping has no deadline of its own, so a server that
/// answers the handshake and then goes silent — a half-open socket, a black-hole
/// firewall rule — must not park the health check, and with it the borrower.
#[cfg(feature = "tokio-runtime")]
#[tokio::test]
#[serial]
async fn the_health_check_gives_up_on_a_silent_server() -> Result<()> {
    use crate::{
        client::{Config, IntoConfig},
        network::timeout,
        tests::fake_server::HELLO_REPLY,
    };
    use bb8::ManageConnection;
    use std::time::{Duration, Instant};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // A server that answers the handshake and never answers anything else. The
    // `FakeServer` cannot do this: an unscripted command gets an error reply,
    // which `is_valid` would report as a failure whatever its deadline.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else {
            return;
        };
        let mut chunk = [0u8; 1024];
        if stream.read(&mut chunk).await.is_err() {
            return;
        }
        if stream.write_all(HELLO_REPLY).await.is_err() {
            return;
        }
        // Read the rest forever, reply to none of it.
        while stream.read(&mut chunk).await.is_ok_and(|n| n > 0) {}
    });

    let mut config: Config = format!("redis://{addr}").into_config()?;
    // The default, spelled out: the health check must bound itself.
    config.command_timeout = Duration::ZERO;
    config.connect_timeout = Duration::from_millis(300);

    let manager = PooledClientManager::new(config)?;
    let mut client = manager.connect().await?;

    let start = Instant::now();
    let result = timeout(
        Duration::from_secs(5),
        TimeoutKind::Command,
        manager.is_valid(&mut client),
    )
    .await;
    server.abort();

    assert!(
        matches!(result, Ok(Err(_))),
        "the health check must fail rather than park: {result:?}"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "the health check took {:?}",
        start.elapsed()
    );

    Ok(())
}
