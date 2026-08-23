//! What `close` reports, which is not always "closed".
//!
//! Hermetic: the connection is an in-memory pipe, so a test can hold every clone
//! of one client and watch which of them actually ends the network task.

use crate::{
    Result, TimeoutKind,
    client::{Client, CloseOutcome},
    commands::ConnectionCommands,
    network::{ReconnectReceiver, timeout},
    tests::{
        fake_server::{FakeServer, duplex_config},
        log_try_init,
    },
};
use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};
use tokio::sync::Barrier;

/// Handles racing one shutdown, more than the runtime below has worker threads,
/// so they queue as well as overlap.
const HANDLES: usize = 8;

/// The losing interleaving is a narrow window between one handle being given up
/// and the next reading what is left, so a single round rarely lands in it.
const ROUNDS: usize = 1000;

/// A client connected to a server that answers the handshake only.
async fn connect() -> Result<Client> {
    Client::connect(duplex_config(
        FakeServer::new(),
        Arc::new(AtomicUsize::new(0)),
    ))
    .await
}

/// Fails unless the network task ends, which it does by dropping the last
/// reconnect sender: the receiver then reports the channel closed. A task that
/// outlives every handle keeps its sender alive and this times out, the leak
/// being unobservable otherwise.
async fn assert_the_network_task_ended(mut on_reconnect: ReconnectReceiver) {
    let closed = timeout(
        Duration::from_secs(5),
        TimeoutKind::Command,
        on_reconnect.recv(),
    )
    .await;
    assert!(
        matches!(closed, Ok(Err(_))),
        "the network task must end once no handle is left, got {closed:?}"
    );
}

#[tokio::test]
async fn closing_a_client_whose_clone_is_alive_reports_the_connection_stayed_up() -> Result<()> {
    log_try_init();
    let client = connect().await?;
    let clone = client.clone();

    assert_eq!(CloseOutcome::StillShared, client.close().await?);
    assert!(!clone.is_terminated());

    Ok(())
}

#[tokio::test]
async fn closing_the_last_client_reports_the_connection_closed() -> Result<()> {
    log_try_init();
    let client = connect().await?;
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

/// Every handle of one connection closing at the same time still shuts it down,
/// once.
///
/// Which handle is the last is decided by `Arc::into_inner`, which hands the
/// shared state to exactly one caller whatever the interleaving. Reading the
/// reference count instead — the shape this replaced — lets each racer see a
/// count above one and back off, so no caller closes the send channel and the
/// network task, its socket and its buffers are left with no handle able to reach
/// them. `Closed` is therefore counted rather than asserted on one call: the bug
/// shows up as nobody reporting it, and a double shutdown as two.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_close_of_every_handle_closes_the_connection_once() -> Result<()> {
    log_try_init();

    for _ in 0..ROUNDS {
        let client = connect().await?;
        let on_reconnect = client.on_reconnect();

        // The handle this test holds is given up before the racers start, so the
        // shutdown is one of theirs to take rather than this thread's.
        let barrier = Arc::new(Barrier::new(HANDLES + 1));
        let closes: Vec<_> = (0..HANDLES)
            .map(|_| {
                let handle = client.clone();
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    handle.close().await
                })
            })
            .collect();
        drop(client);
        barrier.wait().await;

        let mut closed = 0;
        for close in closes {
            if CloseOutcome::Closed == close.await.unwrap()? {
                closed += 1;
            }
        }
        assert_eq!(1, closed, "exactly one handle closes the connection");

        assert_the_network_task_ended(on_reconnect).await;
    }

    Ok(())
}

/// A handle closing while another is dropped shuts the connection down too.
///
/// The two paths give a handle up in different ways — `close` awaits the network
/// task, `Drop` does not — over the one reference count that decides who shuts
/// down. Whichever goes last has to, so a `close` racing a `Drop` may well report
/// `StillShared` and still leave nothing running.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_close_racing_a_drop_still_ends_the_connection() -> Result<()> {
    log_try_init();

    for _ in 0..ROUNDS {
        let client = connect().await?;
        let on_reconnect = client.on_reconnect();

        let barrier = Arc::new(Barrier::new(HANDLES + 1));
        let handles: Vec<_> = (0..HANDLES)
            .map(|index| {
                let handle = client.clone();
                let barrier = Arc::clone(&barrier);
                tokio::spawn(async move {
                    barrier.wait().await;
                    if index % 2 == 0 {
                        drop(handle);
                        return Ok(None);
                    }
                    handle.close().await.map(Some)
                })
            })
            .collect();
        drop(client);
        barrier.wait().await;

        let mut closed = 0;
        for handle in handles {
            if Some(CloseOutcome::Closed) == handle.await.unwrap()? {
                closed += 1;
            }
        }
        assert!(closed <= 1, "at most one handle closes the connection");

        assert_the_network_task_ended(on_reconnect).await;
    }

    Ok(())
}
