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
    ClientError, Error, ErrorKind, Result,
    client::{BackpressureConfig, Client, Config, IntoConfig, ReconnectionConfig},
    commands::{
        BlockingCommands, ClientTrackingOptions, ClientTrackingStatus, ConnectionCommands,
        GenericCommands, StringCommands,
    },
    network::{QueueMetricsTestHook, timeout},
    resp::cmd,
    spawn,
    tests::{
        fault_injection_proxy::{Action, FaultProxy},
        get_default_addr, get_default_config, get_test_client, log_try_init, resident_bytes,
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

    timeout(Duration::from_secs(60), TimeoutKind::Command, async {
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

    // One filler is larger than the whole budget, and the killed `PING` is still
    // awaiting a reply, so nothing is outstanding-free: the first filler is
    // already refused.
    const BUDGET: usize = 4 * 1024;
    const FILLER_BYTES: usize = 8 * 1024;

    let proxy = FaultProxy::start_multi(get_default_addr(), vec![vec![], vec![Action::Drop]])
        .await
        .unwrap();

    let client = Client::connect(storm_config(proxy.addr, BUDGET)?).await?;
    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;

    let error = timeout(Duration::from_secs(30), TimeoutKind::Command, async {
        while proxy.connections_accepted() < 3 {
            tokio::task::yield_now().await;
        }
        // A retryable command would normally be queued and wait for the link to
        // come back; over budget it must be refused instead, and say why.
        let value = "v".repeat(FILLER_BYTES);
        let result: Result<()> = client
            .send(
                cmd("SET").arg("refused_filler").arg(value.as_str()),
                Some(true),
            )
            .await;
        match result {
            Err(e) => Ok::<Error, Error>(e),
            Ok(()) => panic!("a command offered to a full send queue must not succeed"),
        }
    })
    .await??;

    assert!(
        matches!(error.kind(), ErrorKind::Client(ClientError::SendQueueFull)),
        "a command shed by a full queue must report SendQueueFull, got {error:?}"
    );
    assert_eq!(
        Some("SET"),
        error.command(),
        "a shed command must say what was shed, got {error:?}"
    );
    // The caller is told per command; the counter is what makes the *rate*
    // visible to an operator sizing the budget.
    let stats = client.stats();
    assert_eq!(1, stats.shed_commands, "{stats:?}");
    assert!(stats.queued_bytes_high_water > 0, "{stats:?}");

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
    timeout(Duration::from_secs(30), TimeoutKind::Command, async {
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
        match timeout(Duration::from_secs(30), TimeoutKind::Command, handle).await {
            Ok(Ok(Ok(()))) => accepted += 1,
            Ok(Ok(Err(e))) if matches!(e.kind(), ErrorKind::Client(ClientError::SendQueueFull)) => {
                shed += 1
            }
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

/// Waits for a push feed to go quiet, and answers how many messages it carried.
///
/// The count is read after the feed stops moving rather than against a predicted
/// total: Redis batches invalidations, so the number of messages is not the
/// number of keys written. Quiescence here is a fact and not a guess — the writer
/// has already had a command answered, so the server has nothing left to send.
async fn offered_once_quiet(metrics: &QueueMetricsTestHook) -> usize {
    let mut last = metrics.push_delivered();
    let mut still = 0;
    while still < 500 {
        tokio::task::yield_now().await;
        let now = metrics.push_delivered();
        if now == last {
            still += 1;
        } else {
            still = 0;
            last = now;
        }
    }
    last
}

/// A `MONITOR` stream that is held but not polled must stay within its budget.
///
/// This is the harshest of the three sinks: the feed is filled by other clients'
/// traffic, so it keeps growing even when its holder does nothing wrong beyond
/// reading it more slowly than the server produces it, and `MONITOR` offers no
/// way to push back. Shedding the oldest lines is the only bound available.
#[tokio::test]
#[serial]
async fn a_paused_monitor_is_bounded_by_its_memory_budget() -> Result<()> {
    log_try_init();

    // Large values make each monitored line large, so a small budget is reached
    // with few enough commands to keep the test quick.
    const VALUE_BYTES: usize = 4096;
    const BUDGET: usize = 256 * 1024;
    const OFFERED: usize = 5_000;

    let metrics = QueueMetricsTestHook::new();
    let mut config = get_default_config()?;
    config.queue_metrics_test_hook = Some(metrics.clone());
    config.backpressure.max_push_bytes = BUDGET;
    let monitored = Client::connect(config).await?.into_exclusive()?;
    let writer = get_test_client().await?;

    // Held and never polled: the bound must come from the channel, not from a
    // reader keeping up.
    let held_stream = monitored.monitor().await?;

    let baseline_rss = resident_bytes();
    let value = "v".repeat(VALUE_BYTES);

    timeout(Duration::from_secs(60), TimeoutKind::Command, async {
        for i in 0..OFFERED {
            writer.send_and_forget(
                cmd("SET")
                    .arg(format!("monitor_budget_key_{i}"))
                    .arg(value.as_str()),
                None,
            )?;
            // Bound the writer's own send queue, which is not what is under test.
            if i % 500 == 0 {
                let _: String = writer.send(cmd("PING"), None).await?;
            }
        }
        let _: String = writer.send(cmd("PING"), None).await?;
        while metrics.push_delivered() < OFFERED {
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await??;

    let delivered = offered_once_quiet(&metrics).await;
    let dropped = held_stream.dropped_messages();
    let offered_bytes = metrics.push_delivered_bytes();
    let rss_delta = match (baseline_rss, resident_bytes()) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    // Every line delivered and not evicted is still held by the paused stream.
    let held = delivered.saturating_sub(dropped);
    // A monitored `SET` line quotes the value, so it is at least that long. This
    // under-estimates the per-line charge and therefore over-estimates the
    // permitted count, keeping the assertion conservative. One line of slack:
    // the channel always admits into an empty queue.
    let max_held = BUDGET / VALUE_BYTES + 1;
    let report = format!(
        "delivered={delivered} dropped={dropped} held={held} max_held={max_held} \
         offered={offered_bytes} B budget={BUDGET} B rss_delta={rss_delta:?}"
    );
    println!("paused monitor: {report}");

    assert!(
        delivered >= OFFERED,
        "the feed must carry at least the commands that were issued: {report}"
    );
    assert_eq!(
        0,
        metrics.push_delivery_failed(),
        "a live sink must never have a delivery refused: {report}"
    );
    assert!(
        dropped > 0,
        "far more was monitored than the budget allows, so lines must have been \
         dropped and counted: {report}"
    );
    assert!(
        held <= max_held,
        "the stream must hold no more than its budget allows: {report}"
    );
    // `rss_delta` is reported, not asserted, for the reason given above.

    drop(held_stream);
    Ok(())
}

/// A tracking invalidation stream held by the caller must be bounded too.
///
/// The `Cache` (feature `client-cache`) polls its own stream continuously, but
/// `create_client_tracking_invalidation_stream` is public: a caller can hold one
/// and read it slowly, exactly like a subscriber. Loss there is a correctness
/// matter for whatever cache the caller built, so the counter that reports it
/// must be exact — it is that caller's only signal.
#[tokio::test]
#[serial]
async fn a_paused_invalidation_reader_is_bounded_by_its_memory_budget() -> Result<()> {
    log_try_init();

    // Invalidations are small, so the budget is small too. Long keys give each
    // message a known minimum size.
    const BUDGET: usize = 16 * 1024;
    const KEY_PADDING: usize = 128;
    const OFFERED_KEYS: usize = 5_000;

    let metrics = QueueMetricsTestHook::new();
    let mut config = get_default_config()?;
    config.queue_metrics_test_hook = Some(metrics.clone());
    config.backpressure.max_push_bytes = BUDGET;
    let tracked = Client::connect(config).await?;
    let writer = get_test_client().await?;

    // Held and never polled.
    let held_stream = tracked.create_client_tracking_invalidation_stream()?;

    // Broadcasting on a prefix: every write under it invalidates, with no need
    // for the tracked client to read the keys first.
    tracked
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default()
                .prefix("invalidation_budget_key_")
                .broadcasting(),
        )
        .await?;

    let baseline_rss = resident_bytes();
    let padding = "p".repeat(KEY_PADDING);

    timeout(Duration::from_secs(60), TimeoutKind::Command, async {
        for i in 0..OFFERED_KEYS {
            writer.send_and_forget(
                cmd("SET")
                    .arg(format!("invalidation_budget_key_{padding}_{i}"))
                    .arg("v"),
                None,
            )?;
            if i % 500 == 0 {
                let _: String = writer.send(cmd("PING"), None).await?;
            }
        }
        let _: String = writer.send(cmd("PING"), None).await?;
        while metrics.push_delivered() == 0 {
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await??;

    let delivered = offered_once_quiet(&metrics).await;
    let dropped = held_stream.dropped_messages();
    let offered_bytes = metrics.push_delivered_bytes();
    let rss_delta = match (baseline_rss, resident_bytes()) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    let held = delivered.saturating_sub(dropped);
    // Redis batches invalidations, so a message carries an unpredictable number
    // of keys and its size cannot be derived from the keys written — it is
    // measured instead. Twice the budget's worth of average-sized messages is
    // allowed: the messages still held are the last ones, which may be smaller
    // than average, and a regression to no bound at all would sit orders of
    // magnitude above this.
    let average_bytes = offered_bytes.max(1) / delivered.max(1);
    let max_held = 2 * BUDGET / average_bytes.max(1) + 1;
    let report = format!(
        "delivered={delivered} dropped={dropped} held={held} max_held={max_held} \
         offered={offered_bytes} B average={average_bytes} B budget={BUDGET} B \
         rss_delta={rss_delta:?}"
    );
    println!("paused invalidation reader: {report}");

    assert_eq!(
        0,
        metrics.push_delivery_failed(),
        "a live sink must never have a delivery refused: {report}"
    );
    assert!(
        dropped > 0,
        "far more was invalidated than the budget allows, so invalidations must \
         have been dropped and counted: {report}"
    );
    assert!(
        held <= max_held,
        "the stream must hold no more than its budget allows: {report}"
    );

    drop(held_stream);
    tracked
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;
    Ok(())
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
