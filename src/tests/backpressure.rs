//! What the client holds on behalf of a consumer that has stopped keeping up,
//! and the budgets that bound it.
//!
//! The pub/sub half of the question lives with the pub/sub suite, next to the
//! subscription API it exercises. This module holds the send-queue half, which
//! needs a sustained outage and therefore the fault proxy, and the two push
//! sinks — the `MONITOR` feed and the tracking invalidation stream.
//!
//! The push sinks share the send path and the bounded channel with pub/sub, but
//! not its exposure, which is why they get their own scenarios. A `MONITOR`
//! stream is fed by every *other* client's traffic, so it grows without its
//! holder doing anything unusual, and the server offers no way to slow the feed
//! down. An invalidation stream carries a correctness signal rather than data:
//! discarding one leaves a key stale, so what has to be proven is not only the
//! bound but that the loss is counted and acted upon.

use crate::TimeoutKind;
use crate::{
    ClientError, ErrorKind, Result,
    client::{BackpressureConfig, Client, Config, IntoConfig, ReconnectionConfig},
    network::timeout,
    resp::cmd,
    tests::log_try_init,
};
use serial_test::serial;
use std::time::Duration;

/// Bytes charged per queued message on top of its command buffers. Mirrors the
/// crate-internal allowance so the tests can predict the bound.
const MESSAGE_OVERHEAD: usize = 1024;

/// Builds a client whose reconnection never gives up and whose send queue is
/// capped at `max_queued_bytes`, pointed at `addr`.
///
/// The reconnection cap stays at `0`: a non-zero one ends the network task for
/// good, which is a different failure from the one under test.
fn storm_config(addr: std::net::SocketAddr, max_queued_bytes: usize) -> Result<Config> {
    let mut config = format!("redis://{addr}").into_config()?;
    config.retry_on_error = true;
    config.reconnection = ReconnectionConfig::new_constant(0, 50);
    config.connect_timeout = Duration::from_millis(200);
    config.command_timeout = Duration::ZERO;
    config.backpressure = BackpressureConfig {
        max_queued_bytes,
        ..Default::default()
    };
    Ok(config)
}

/// The budget must bound what is *in flight*, not only what is waiting to be
/// written. A connection that accepts every byte and answers none leaves each
/// message in `messages_to_receive`, where the charge used to be released the
/// moment the command was written — so the documented "bound memory with
/// `BackpressureConfig`" story had a hole exactly the size of one keep-alive
/// interval.
#[tokio::test]
#[serial]
async fn the_budget_bounds_the_replies_still_awaited() -> Result<()> {
    use crate::tests::fake_server::HELLO_REPLY;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    log_try_init();

    const VALUE_BYTES: usize = 1024;
    const BUDGET: usize = 64 * 1024;

    // Answers the handshake, then reads everything and replies to none of it.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let Ok((mut stream, _)) = listener.accept().await else {
            return;
        };
        let mut chunk = [0u8; 4096];
        if stream.read(&mut chunk).await.is_err() {
            return;
        }
        if stream.write_all(HELLO_REPLY).await.is_err() {
            return;
        }
        while stream.read(&mut chunk).await.is_ok_and(|n| n > 0) {}
    });

    let mut config = storm_config(addr, BUDGET)?;
    // The keep-alive would eventually break the socket and end the scenario; the
    // point is what the budget does before that.
    config.keep_alive = None;
    let client = Client::connect(config).await?;

    // Enough to pass the budget several times over, all of it awaiting a reply
    // that never comes. The fill is fire-and-forget because awaiting would park
    // on a server that never answers; a shed fire-and-forget has no caller to
    // report to, so the refusal is read on the awaited command that follows.
    let value = "v".repeat(VALUE_BYTES);
    let fill = (BUDGET / (VALUE_BYTES + MESSAGE_OVERHEAD)) * 4;
    for i in 0..fill {
        client.send_and_forget(
            cmd("SET").arg(format!("budget_{i}")).arg(value.as_str()),
            None,
        )?;
    }

    // Let the network task write them all, which is what used to release their
    // charge.
    crate::network::sleep(Duration::from_millis(200)).await;

    let refused: Result<()> = timeout(
        Duration::from_secs(2),
        TimeoutKind::Command,
        client.send(cmd("SET").arg("budget_probe").arg(value.as_str()), None),
    )
    .await?;

    server.abort();

    let error = refused.unwrap_err();
    assert!(
        matches!(error.kind(), ErrorKind::Client(ClientError::SendQueueFull)),
        "a reply still awaited must keep holding its share of the budget, got {error:?}"
    );

    Ok(())
}
