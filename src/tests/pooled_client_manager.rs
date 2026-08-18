use crate::{
    Result, client::PooledClientManager, commands::StringCommands, tests::get_default_addr,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn pooled_client_manager() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr())?;
    let pool = crate::bb8::Pool::builder().build(manager).await?;
    let client = pool.get().await.unwrap();

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

/// The pool asks the manager whether a connection is broken before handing it
/// out again. A client whose network task has ended can no longer answer
/// anything, so the pool must drop it instead of recycling it forever.
#[cfg(feature = "tokio-runtime")]
#[tokio::test]
#[serial]
async fn a_client_whose_network_task_ended_is_reported_broken() -> Result<()> {
    use crate::{
        client::{Config, IntoConfig, ReconnectionConfig},
        tests::fault_injection_proxy::FaultProxy,
    };
    use bb8::ManageConnection;

    // Front the server with a proxy so the connection can be made unrecoverable:
    // once the proxy is gone, nothing listens on that port any more.
    let proxy = FaultProxy::start(get_default_addr(), vec![]).await?;
    let mut config: Config = format!("redis://{}", proxy.addr).into_config()?;
    // A single reconnection attempt, so the network task gives up promptly.
    config.reconnection = ReconnectionConfig::new_constant(1, 10);

    let manager = PooledClientManager::new(config)?;
    let mut client = manager.connect().await?;
    client.set("pool_probe", "value").await?;

    drop(proxy);

    // Give the network task the time to fail its single reconnection attempt and end.
    crate::network::sleep(std::time::Duration::from_millis(500)).await;

    assert!(
        manager.has_broken(&mut client),
        "a client whose network task has ended must be reported broken"
    );

    Ok(())
}

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
    let result = timeout(Duration::from_secs(5), manager.is_valid(&mut client)).await;
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
