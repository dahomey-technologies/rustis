//! What `close` reports, which is not always "closed".
//!
//! Hermetic: the connection is an in-memory pipe, so a test can hold two clones
//! of one client and watch which of them actually ends the network task.

use crate::{
    Result,
    client::{Client, CloseOutcome},
    commands::ConnectionCommands,
    tests::{
        fake_server::{FakeServer, duplex_config},
        log_try_init,
    },
};
use std::sync::{Arc, atomic::AtomicUsize};

#[tokio::test]
async fn closing_a_client_whose_clone_is_alive_reports_the_connection_stayed_up() -> Result<()> {
    log_try_init();
    let client = Client::connect(duplex_config(
        FakeServer::new(),
        Arc::new(AtomicUsize::new(0)),
    ))
    .await?;
    let clone = client.clone();

    assert_eq!(CloseOutcome::StillShared, client.close().await?);
    assert!(!clone.is_terminated());

    Ok(())
}

#[tokio::test]
async fn closing_the_last_client_reports_the_connection_closed() -> Result<()> {
    log_try_init();
    let client = Client::connect(duplex_config(
        FakeServer::new(),
        Arc::new(AtomicUsize::new(0)),
    ))
    .await?;
    let clone = client.clone();

    assert_eq!(CloseOutcome::StillShared, client.close().await?);
    assert_eq!(CloseOutcome::Closed, clone.close().await?);

    Ok(())
}

/// A `StillShared` close leaves the connection up, so the surviving handle must
/// still be able to send on it.
///
/// The outcome alone does not say that, nor does `is_terminated`, which reads
/// the network task and not the channel the closing clone let go of.
#[tokio::test]
async fn a_surviving_clone_still_sends_after_another_handle_closed() -> Result<()> {
    log_try_init();
    let client = Client::connect(duplex_config(
        FakeServer::new().reply("PING", b"+PONG\r\n"),
        Arc::new(AtomicUsize::new(0)),
    ))
    .await?;
    let clone = client.clone();

    assert_eq!(CloseOutcome::StillShared, client.close().await?);
    assert_eq!("PONG", clone.ping::<String>(()).await?);

    Ok(())
}
