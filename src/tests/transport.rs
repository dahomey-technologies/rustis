//! The transport injection points: a caller-supplied stream, and a Unix socket.
//!
//! Every test here is hermetic — no Redis, and for the duplex ones no socket at
//! all — because the whole point of the feature is that the client no longer
//! needs a TCP endpoint to reach a server.

use crate::{
    Result,
    client::{Client, Config, CustomTransport, ServerConfig},
    commands::{ConnectionCommands, StringCommands},
    resp::cmd,
    tests::{
        fake_server::{FakeServer, duplex_config, duplex_pair},
        log_try_init,
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Serves `server` on a Unix socket in a temporary directory, returning its
/// path. The socket is left behind for the test's lifetime, which is short.
#[cfg(unix)]
fn serve_on_unix_socket(name: &str, server: FakeServer) -> Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("rustis-uds-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("redis.sock");
    let _ = std::fs::remove_file(&path);

    let listener = tokio::net::UnixListener::bind(&path)?;
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let server = server.clone();
            tokio::spawn(async move { server.serve(stream).await });
        }
    });

    Ok(path)
}

#[tokio::test]
async fn a_custom_transport_carries_the_whole_client_stack() -> Result<()> {
    log_try_init();

    let config = duplex_config(
        FakeServer::new().reply("PING", b"+PONG\r\n"),
        Arc::new(AtomicUsize::new(0)),
    );

    let client = Client::connect(config).await?;
    assert_eq!("PONG", client.ping::<String>(()).await?);

    Ok(())
}

#[tokio::test]
async fn a_closure_is_a_transport_factory() -> Result<()> {
    log_try_init();

    let config = Config {
        server: ServerConfig::Custom(CustomTransport::new(|| async {
            Ok(duplex_pair(
                FakeServer::new().reply("GET", b"$5\r\nvalue\r\n"),
            ))
        })),
        ..Default::default()
    };

    let client = Client::connect(config).await?;
    assert_eq!("value", client.get::<String>("key").await?);

    Ok(())
}

#[tokio::test]
async fn the_factory_is_asked_again_on_each_reconnection() -> Result<()> {
    log_try_init();

    let dials = Arc::new(AtomicUsize::new(0));
    let client = Client::connect(duplex_config(
        FakeServer::new().reply("PING", b"+PONG\r\n"),
        Arc::clone(&dials),
    ))
    .await?;

    assert_eq!(1, dials.load(Ordering::SeqCst));
    assert_eq!("PONG", client.ping::<String>(()).await?);

    // A transport handed over once would be spent here: the reconnection has to
    // obtain a second pipe, and the command after it has to go through.
    let mut reconnections = client.on_reconnect();
    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;
    reconnections.recv().await.unwrap();

    assert_eq!(2, dials.load(Ordering::SeqCst));
    assert_eq!("PONG", client.ping::<String>(()).await?);

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn a_unix_socket_reaches_the_server() -> Result<()> {
    log_try_init();

    let path = serve_on_unix_socket("config", FakeServer::new().reply("PING", b"+PONG\r\n"))?;

    let client = Client::connect(Config {
        server: ServerConfig::UnixSocket { path },
        ..Default::default()
    })
    .await?;
    assert_eq!("PONG", client.ping::<String>(()).await?);

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn a_unix_socket_is_reachable_from_its_uri() -> Result<()> {
    log_try_init();

    let path = serve_on_unix_socket("uri", FakeServer::new().reply("PING", b"+PONG\r\n"))?;

    let client = Client::connect(format!("unix://{}", path.display())).await?;
    assert_eq!("PONG", client.ping::<String>(()).await?);

    Ok(())
}
