//! What the client holds on behalf of a consumer that has stopped keeping up,
//! and the budgets that bound it.
//!
//! The pub/sub half of the question lives with the pub/sub suite, next to the
//! subscription API it exercises. This module holds the send-queue half, which
//! needs a sustained outage and therefore the fault proxy.

use crate::{
    ClientError, Error, Result,
    client::{BackpressureConfig, Client, Config, IntoConfig, ReconnectionConfig},
    commands::{GenericCommands, StringCommands},
    network::{QueueMetricsTestHook, timeout},
    resp::cmd,
    spawn,
    tests::{
        fault_injection_proxy::{Action, FaultProxy},
        get_default_addr, log_try_init, resident_bytes,
    },
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

/// The budget must actually bound the send queue during an outage — this is the
/// growth that was measured as unbounded before it existed.
///
/// The bound is asserted arithmetically rather than by a round number: with a
/// known budget and a known per-message charge, the peak depth the queue may
/// reach is computable, and the queue is allowed exactly one message of overrun
/// because an empty queue always admits whatever is offered.
#[tokio::test]
#[serial]
async fn the_send_queue_stops_growing_at_its_memory_budget() -> Result<()> {
    log_try_init();

    const VALUE_BYTES: usize = 1024;
    const BUDGET: usize = 1024 * 1024;
    const OFFERED: usize = 50_000;

    let proxy = FaultProxy::start_multi(get_default_addr(), vec![vec![], vec![Action::Drop]])
        .await
        .unwrap();

    let metrics = QueueMetricsTestHook::new();
    let mut config = storm_config(proxy.addr, BUDGET)?;
    config.queue_metrics_test_hook = Some(metrics.clone());
    let client = Client::connect(config).await?;

    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;

    let baseline_rss = resident_bytes();
    let value = "v".repeat(VALUE_BYTES);

    timeout(Duration::from_secs(60), async {
        while proxy.connections_accepted() < 3 {
            tokio::task::yield_now().await;
        }
        for i in 0..OFFERED {
            client.send_and_forget(
                cmd("SET")
                    .arg(format!("budget_key_{i}"))
                    .arg(value.as_str()),
                Some(true),
            )?;
            if i % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }
        // Let the network task drain the channel into the send queue, which is
        // where the budget applies.
        for _ in 0..1000 {
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await??;

    let peak = metrics.messages_to_send_high_water();
    let rss_delta = match (baseline_rss, resident_bytes()) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    // A `SET key <1 KiB value>` buffer is at least the value itself, so this
    // under-estimates the per-message charge and therefore over-estimates the
    // permitted depth — the assertion stays conservative.
    let min_charge = VALUE_BYTES + MESSAGE_OVERHEAD;
    let max_depth = BUDGET / min_charge + 1;
    let report =
        format!("peak={peak} max_depth={max_depth} offered={OFFERED} rss_delta={rss_delta:?}");
    println!("send queue budget: {report}");

    assert!(
        peak <= max_depth,
        "the send queue must stay within its budget plus one message: {report}"
    );
    // `rss_delta` is reported, not asserted. Resident memory is a process-wide
    // figure: the shed commands are allocated and freed, and the allocator keeps
    // the pages, so the delta reflects arena behaviour as much as retention. It
    // was the right corroboration for a 220 MiB growth against a few MiB of
    // noise; it cannot prove a bound of that same order. The queue depth above
    // is exact and is the proof.

    Ok(())
}

/// A command shed by a full send queue must say so, with an error distinct from
/// the two other ways a command dies during an outage.
#[tokio::test]
#[serial]
async fn a_command_refused_by_a_full_send_queue_reports_it() -> Result<()> {
    log_try_init();

    // One filler is larger than the whole budget, so the queue is over budget
    // from the first one and every later command is refused whatever its size.
    const BUDGET: usize = 4 * 1024;
    const FILLER_BYTES: usize = 8 * 1024;

    let proxy = FaultProxy::start_multi(get_default_addr(), vec![vec![], vec![Action::Drop]])
        .await
        .unwrap();

    let client = Client::connect(storm_config(proxy.addr, BUDGET)?).await?;
    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;

    let error = timeout(Duration::from_secs(30), async {
        while proxy.connections_accepted() < 3 {
            tokio::task::yield_now().await;
        }
        let value = "v".repeat(FILLER_BYTES);
        for _ in 0..10 {
            client.send_and_forget(
                cmd("SET").arg("refused_filler").arg(value.as_str()),
                Some(true),
            )?;
        }
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }

        // A retryable command would normally be queued and wait for the link to
        // come back; over budget it must be refused instead, and say why.
        let result: Result<String> = client.send(cmd("PING"), Some(true)).await;
        match result {
            Err(e) => Ok::<Error, Error>(e),
            Ok(_) => panic!("a command offered to a full send queue must not succeed"),
        }
    })
    .await??;

    assert!(
        matches!(error, Error::Client(ClientError::SendQueueFull)),
        "a command shed by a full queue must report SendQueueFull, got {error:?}"
    );

    Ok(())
}

/// The invariant that makes the budget safe: it sheds only *incoming* commands.
///
/// A command that was accepted, then replayed by a reconnection, must never be
/// refused by the budget — otherwise the client would drop a command its caller
/// had been told was on its way. Here the budget is far smaller than the queue
/// the outage builds, so every replayed command is offered to a queue that is
/// already over budget.
#[tokio::test]
#[serial]
async fn a_command_already_queued_survives_the_reconnection_that_replays_it() -> Result<()> {
    log_try_init();

    const BUDGET: usize = 4 * 1024;
    const COMMANDS: usize = 20;

    // The outage lasts two failed reconnections, then the link comes back.
    let proxy = FaultProxy::start_multi(
        get_default_addr(),
        vec![vec![], vec![Action::Drop], vec![Action::Drop], vec![]],
    )
    .await
    .unwrap();

    let client = Client::connect(storm_config(proxy.addr, BUDGET)?).await?;

    let control = Client::connect(get_default_addr()).await?;
    for i in 0..COMMANDS {
        control.del(format!("replayed_key_{i}")).await?;
    }

    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;

    // Every command that is *accepted* must eventually reach the server. Ones
    // refused up front are counted separately: shedding a new command is the
    // budget working, losing an accepted one is the bug this guards against.
    let mut handles = Vec::new();
    timeout(Duration::from_secs(30), async {
        while proxy.connections_accepted() < 2 {
            tokio::task::yield_now().await;
        }
        for i in 0..COMMANDS {
            let client = client.clone();
            handles.push(spawn(async move {
                let result: Result<String> = client
                    .send(
                        cmd("SET").arg(format!("replayed_key_{i}")).arg(i),
                        Some(true),
                    )
                    .await;
                result.map(|_| ())
            }));
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await??;

    let mut accepted = 0usize;
    let mut shed = 0usize;
    for handle in handles {
        match timeout(Duration::from_secs(30), handle).await {
            Ok(Ok(Ok(()))) => accepted += 1,
            Ok(Ok(Err(Error::Client(ClientError::SendQueueFull)))) => shed += 1,
            other => panic!("unexpected outcome for a queued command: {other:?}"),
        }
    }

    let mut stored = 0usize;
    for i in 0..COMMANDS {
        let value: Option<usize> = control.get(format!("replayed_key_{i}")).await?;
        if value.is_some() {
            stored += 1;
        }
    }

    let report = format!("accepted={accepted} shed={shed} stored={stored} of {COMMANDS}");
    println!("replay invariant: {report}");

    assert!(
        accepted > 0,
        "the test proves nothing unless some commands were accepted: {report}"
    );
    assert_eq!(
        accepted, stored,
        "every accepted command must have reached the server: {report}"
    );

    Ok(())
}

/// The byte accounting must come back down as the queue drains, otherwise the
/// budget would latch and refuse commands forever on a healthy connection.
#[tokio::test]
#[serial]
async fn the_send_queue_budget_is_released_when_the_queue_drains() -> Result<()> {
    log_try_init();

    const BUDGET: usize = 16 * 1024;

    let mut config = get_default_addr().into_config()?;
    config.backpressure = BackpressureConfig {
        max_queued_bytes: BUDGET,
        ..Default::default()
    };
    let client = Client::connect(config).await?;

    // Far more traffic than the budget, but sent in awaited waves so the queue
    // drains between them. Nothing may be refused.
    let value = "v".repeat(4096);
    for wave in 0..20 {
        for i in 0..2 {
            client
                .set(format!("drain_key_{wave}_{i}"), value.as_str())
                .await?;
        }
        let _: String = client.send(cmd("PING"), None).await?;
    }

    let stored: String = client.get("drain_key_19_1").await?;
    assert_eq!(
        value, stored,
        "a drained queue must keep accepting commands"
    );

    Ok(())
}
