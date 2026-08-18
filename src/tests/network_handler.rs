use crate::{
    ClientError, ErrorKind, Result, RetryReason,
    client::{Client, ClientPreparedCommand, ReconnectionConfig},
    commands::{
        ClientReplyMode, ConnectionCommands, GenericCommands, PubSubCommands, StringCommands,
    },
    network::{QueueMetricsTestHook, SendBatchTestHook, sleep, timeout},
    resp::cmd,
    tests::{
        get_default_config, get_default_port, get_test_client, get_test_client_with_config,
        log_try_init,
    },
};
use serial_test::serial;
use std::future::IntoFuture;
use std::{collections::HashMap, time::Duration};

/// Retry reasons accumulated for one message must not be applied to the other
/// messages sharing the same send batch: each message must be fed only with
/// its own reasons.
#[tokio::test]
#[serial]
async fn retry_reasons_do_not_leak_across_messages_in_a_batch() -> Result<()> {
    log_try_init();

    let hook = SendBatchTestHook::new();
    let mut config = get_default_config()?;
    config.send_batch_test_hook = Some(hook.clone());
    let client = Client::connect(config).await?;

    // Force an ASK retry reason onto the first message of the next send batch.
    hook.push_injection(Some(vec![RetryReason::Ask {
        hash_slot: 0,
        address: ("127.0.0.1".to_owned(), get_default_port()),
    }]));

    // Enqueue two independent messages synchronously so they are drained
    // together in a single send batch. `send_and_forget` performs the enqueue
    // without awaiting, guaranteeing both are queued before the network task
    // drains them.
    client.send_and_forget(cmd("GET").arg("net12_a"), None)?;
    client.send_and_forget(cmd("STRLEN").arg("net12_b"), None)?;

    // Await a follow-up command to ensure the batch has been sent.
    let _: String = client.send(cmd("PING"), None).await?;

    let fed = hook.fed_retry_reasons();

    // Locate the injected first message and the message that follows it in the
    // same batch.
    let first_idx = fed
        .iter()
        .position(|(name, _)| name == "GET")
        .expect("GET should have been fed");
    assert_eq!(
        1, fed[first_idx].1,
        "the first message should carry the injected reason"
    );

    let (second_name, second_reasons) = &fed[first_idx + 1];
    assert_eq!("STRLEN", second_name, "the second message should follow");
    assert_eq!(
        0, *second_reasons,
        "a following message must not inherit the previous message's retry reasons"
    );

    Ok(())
}

/// The queue-depth marks are high-water marks: they must report the peak a
/// queue reached even after it has been fully drained. Without that, no test can
/// measure a transient queue, because a measurement always arrives after the
/// drain.
#[tokio::test]
#[serial]
async fn the_queue_depth_marks_survive_the_drain() -> Result<()> {
    log_try_init();

    let metrics = QueueMetricsTestHook::new();
    let mut config = get_default_config()?;
    config.queue_metrics_test_hook = Some(metrics.clone());
    let client = Client::connect(config).await?;

    // Enqueue many independent messages without awaiting, so they pile up in the
    // channel before the network task gets a turn. `send_and_forget` is
    // synchronous, so this loop never yields and the whole batch is queued.
    for i in 0..100 {
        client.send_and_forget(cmd("GET").arg(format!("depth_mark_{i}")), None)?;
    }

    // Await a follow-up command: by the time it answers, every queued message
    // has been sent and both queues are empty again.
    let _: String = client.send(cmd("PING"), None).await?;

    let send_peak = metrics.messages_to_send_high_water();
    let receive_peak = metrics.messages_to_receive_high_water();

    assert!(
        send_peak > 1,
        "the send-queue mark should hold the peak reached before the drain, got {send_peak}"
    );
    assert!(
        receive_peak > 1,
        "the receive-queue mark should hold the peak reached before the drain, got {receive_peak}"
    );

    Ok(())
}

/// On reconnect, a non-retryable message sitting behind a retryable one must
/// be failed, not replayed: replaying it double-executes a command whose
/// caller explicitly opted out of retries.
#[tokio::test]
#[serial]
async fn non_retryable_message_behind_retryable_is_not_replayed_on_reconnect() -> Result<()> {
    log_try_init();

    let control = get_test_client().await?;
    control.del("net01_counter").await?;

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = get_test_client_with_config(config).await?;

    // Enqueue two messages in a single batch: a retryable head, then a
    // non-retryable command with an observable side effect. Both reach and are
    // executed by the server; the connection is then closed on read, before
    // any response is matched, forcing a reconnect while both are in flight.
    client.send_and_forget(cmd("GET").arg("net01_dummy"), Some(true))?;
    client.send_and_forget(
        cmd("INCR").arg("net01_counter").kill_connection_on_read(1),
        Some(false),
    )?;

    // Let the batch be sent, the connection be closed, and the reconnection
    // and any replays settle.
    sleep(Duration::from_millis(500)).await;

    // The non-retryable INCR must have executed exactly once.
    let counter: i64 = control.get("net01_counter").await?;
    assert_eq!(
        1, counter,
        "the non-retryable command must not be replayed on reconnect"
    );

    control.del("net01_counter").await?;
    Ok(())
}

/// On reconnect, an in-flight UNSUBSCRIBE that is replayed must have its
/// pub/sub bookkeeping (`pending_unsubscriptions`) rebuilt. Otherwise its
/// confirmation push arrives with nothing to match, the stale message keeps
/// its slot in the receive queue, and it consumes the reply of the next
/// command — shifting every subsequent response by one, permanently.
#[tokio::test]
#[serial]
async fn inflight_unsubscribe_does_not_desync_responses_after_reconnect() -> Result<()> {
    log_try_init();

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    // Make the in-flight UNSUBSCRIBE retryable so it survives the reconnect
    // purge and is replayed: that replay is what arms the desync.
    config.retry_on_error = true;
    let client = get_test_client_with_config(config).await?;

    // Send an UNSUBSCRIBE and close the connection on the next read, before its
    // confirmation is matched, so it is in flight when the reconnect happens.
    client.send_and_forget(
        cmd("UNSUBSCRIBE")
            .arg("net03_chan")
            .kill_connection_on_read(1),
        None,
    )?;

    // Let the reconnection complete and the UNSUBSCRIBE be replayed.
    sleep(Duration::from_millis(500)).await;

    // A follow-up command must receive its own reply. With the desync its
    // reply is consumed by the stale UNSUBSCRIBE slot and the call hangs.
    let echoed: String = timeout(
        Duration::from_secs(2),
        client.send(cmd("ECHO").arg("net03_marker"), None),
    )
    .await??;

    assert_eq!(
        "net03_marker", echoed,
        "the follow-up response must be routed to its own caller"
    );

    Ok(())
}

/// On reconnect, an in-flight UNSUBSCRIBE must not be turned into a SUBSCRIBE by
/// `auto_resubscribe`. The correct action for a pending unsubscription on a
/// fresh connection is to emit nothing and drop it: resubscribing would leave
/// the server subscribed to a channel the client no longer tracks.
#[tokio::test]
#[serial]
async fn inflight_unsubscribe_is_not_turned_into_subscribe_on_reconnect() -> Result<()> {
    log_try_init();

    let control = get_test_client().await?;

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    // Keep the default `auto_resubscribe = true` so the reconnection runs the
    // resubscribe pass that this test exercises.
    let client = get_test_client_with_config(config).await?;

    // Send a non-retryable UNSUBSCRIBE (so it is purged rather than replayed)
    // and close the connection on the next read. Its entry then sits in the
    // handler's pending-unsubscriptions bookkeeping when the reconnection fires
    // `auto_resubscribe`.
    client.send_and_forget(
        cmd("UNSUBSCRIBE")
            .arg("net02_chan")
            .kill_connection_on_read(1),
        None,
    )?;

    // Let the reconnection and its resubscribe pass run.
    sleep(Duration::from_millis(500)).await;

    // The client must not have subscribed the server to the channel.
    let num_sub: HashMap<String, usize> = control.pub_sub_numsub(["net02_chan"]).await?;
    assert_eq!(
        Some(&0usize),
        num_sub.get("net02_chan"),
        "an in-flight unsubscription must not be resubscribed on reconnect"
    );

    Ok(())
}

/// A retryable command must be given up after `Config::max_command_attempts`
/// replays and failed with a distinct error rather than replayed further. With a
/// cap of 1, the single reconnection replay this test forces already reaches the
/// budget, so the command is failed instead of retried.
#[tokio::test]
#[serial]
async fn retryable_command_fails_after_max_command_attempts() -> Result<()> {
    log_try_init();

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 10);
    config.retry_on_error = true;
    // One attempt allowed: the first replay exhausts the budget.
    config.max_command_attempts = 1;
    let client = get_test_client_with_config(config).await?;

    // The command tears the socket down before its response is matched, forcing a
    // reconnect that would replay it — which the cap turns into a failure.
    let result: Result<String> = timeout(
        Duration::from_secs(5),
        client.send(cmd("PING").kill_connection_on_read(1), Some(true)),
    )
    .await?;

    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::MaxCommandAttemptsReached)
        ),
        "expected MaxCommandAttemptsReached, got {error:?}"
    );

    Ok(())
}

/// A command carrying redirection reasons is routed by them, twice in a row.
///
/// This covers the *send* path only: the reasons are attached before the write,
/// so the command is routed as a redirection asks and never replayed. What spends
/// the attempt budget is a redirection arriving as a **reply**, which
/// `a_redirection_spends_the_attempt_budget` in the cluster suite covers.
#[tokio::test]
#[serial]
async fn a_command_is_routed_by_two_successive_redirection_reasons() -> Result<()> {
    log_try_init();

    let hook = SendBatchTestHook::new();
    let mut config = get_default_config()?;
    config.send_batch_test_hook = Some(hook.clone());
    let client = get_test_client_with_config(config.clone()).await?;

    // Two successive redirections, as a slot migration that finishes midway
    // would produce, then the command is left alone and must succeed.
    let address = ("127.0.0.1".to_owned(), get_default_port());
    hook.push_injection(Some(vec![RetryReason::Ask {
        hash_slot: 0,
        address: address.clone(),
    }]));
    hook.push_injection(Some(vec![RetryReason::Moved {
        hash_slot: 0,
        address,
    }]));

    let result: Result<String> =
        timeout(Duration::from_secs(5), client.send(cmd("PING"), Some(true))).await?;

    assert_eq!(
        "PONG", result?,
        "a command redirected twice must still be answered"
    );

    Ok(())
}

/// `CLIENT REPLY OFF` is connection state, and the handler mirrors it to know how
/// many responses each command it writes will produce. A reconnection must leave
/// the two in agreement: if the socket comes back answering while the mirror still
/// says it is silent, every unexpected reply shifts the following responses by one.
#[tokio::test]
#[serial]
async fn reply_mode_is_restored_after_reconnect() -> Result<()> {
    log_try_init();

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = get_test_client_with_config(config).await?;

    client.del(["reply_a", "reply_b"]).await?;

    let mut on_reconnect = client.on_reconnect();

    // Silence the connection, then lose it while it is silent.
    client.client_reply(ClientReplyMode::Off).forget()?;
    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;

    on_reconnect
        .recv()
        .await
        .expect("the client should have reconnected");

    // Two writes over the reconnected socket, then speech again.
    client.set("reply_a", "a").forget()?;
    client.set("reply_b", "b").forget()?;
    timeout(
        Duration::from_secs(5),
        client.client_reply(ClientReplyMode::On).into_future(),
    )
    .await??;

    // Each response must reach the caller that asked for it.
    let a: String = timeout(Duration::from_secs(5), client.get("reply_a").into_future()).await??;
    let b: String = timeout(Duration::from_secs(5), client.get("reply_b").into_future()).await??;
    assert_eq!("a", a, "responses must not be shifted after a reconnection");
    assert_eq!("b", b, "responses must not be shifted after a reconnection");

    client.del(["reply_a", "reply_b"]).await?;
    Ok(())
}

/// `CLIENT REPLY SKIP` silences the next command and nothing beyond it. Treating
/// it as a sticky `OFF` makes the handler expect one reply where the server sends
/// several, and the surplus shifts every response that follows.
#[tokio::test]
#[serial]
async fn reply_skip_silences_only_the_next_command() -> Result<()> {
    log_try_init();

    let client = get_test_client().await?;
    client.del(["skip_a", "skip_b"]).await?;

    // SKIP suppresses the reply of `skip_a` only; `skip_b` is answered.
    client.client_reply(ClientReplyMode::Skip).forget()?;
    client.set("skip_a", "a").forget()?;
    client.set("skip_b", "b").forget()?;

    let a: String = timeout(Duration::from_secs(5), client.get("skip_a").into_future()).await??;
    let b: String = timeout(Duration::from_secs(5), client.get("skip_b").into_future()).await??;
    assert_eq!("a", a, "responses must not be shifted after a SKIP");
    assert_eq!("b", b, "responses must not be shifted after a SKIP");

    client.del(["skip_a", "skip_b"]).await?;
    Ok(())
}

/// `RESET` restores every per-connection default server-side, reply mode
/// included. A client that keeps mirroring the pre-`RESET` mode stops accounting
/// for the replies the server is once again sending.
#[tokio::test]
#[serial]
async fn reset_restores_the_reply_mode_the_server_restored() -> Result<()> {
    log_try_init();

    let client = get_test_client().await?;

    client.client_reply(ClientReplyMode::Off).forget()?;
    client.send_and_forget(cmd("RESET"), None)?;

    let pong: String = timeout(Duration::from_secs(5), client.send(cmd("PING"), None)).await??;
    assert_eq!(
        "PONG", pong,
        "RESET turns replies back on, and the client must expect them again"
    );

    Ok(())
}

/// A caller that gives up on a reply — a `command_timeout`, a losing `select!`
/// branch, a dropped future — leaves the network task holding a reply nobody
/// awaits. That is the documented contract, not a fault, so it must not be
/// reported at `warn!`: a service with deadlines would flood its logs precisely
/// when Redis is slow. The event still names the command, which is the one thing
/// a multiplexed caller cannot work out for itself.
#[tokio::test]
#[serial]
async fn a_reply_nobody_awaits_is_logged_at_debug_with_its_command() -> Result<()> {
    use crate::{commands::DebugCommands, tests::LogCapture};

    let mut config = get_default_config()?;
    config.command_timeout = Duration::from_millis(50);
    let client = Client::connect(config).await?;

    let capture = LogCapture::start();

    // The server sleeps well past the deadline, so the caller times out and
    // drops its receiver before the reply is written.
    let result = client.debug_sleep(Duration::from_millis(300)).await;
    assert!(
        result.unwrap_err().is_timeout(),
        "the command must fail on its deadline"
    );

    // Let the reply arrive, so the task finds no receiver for it.
    sleep(Duration::from_millis(500)).await;
    let events = capture.events();
    drop(capture);

    let abandoned: Vec<_> = events
        .iter()
        .filter(|(_, message)| message.contains("receiver"))
        .collect();
    assert!(
        !abandoned.is_empty(),
        "the abandoned reply must be logged: {events:?}"
    );
    for (level, message) in &abandoned {
        assert_eq!(
            log::Level::Debug,
            *level,
            "an abandoned reply is routine, not a warning: {message}"
        );
        assert!(
            message.contains("DEBUG"),
            "the event must name the command: {message}"
        );
    }

    Ok(())
}

/// `retry_on_error` governs the reconnection replay, and that alone. A command
/// left on the stock `false` is failed by the lost connection rather than
/// replayed, so it never reaches the attempt budget — which is what makes the
/// budget look inert on a default configuration, and is why the default says so.
#[tokio::test]
#[serial]
async fn a_default_command_is_failed_by_a_lost_connection_rather_than_replayed() -> Result<()> {
    log_try_init();

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 10);
    assert!(
        !config.retry_on_error,
        "this test is about the stock default; update it deliberately if it changes"
    );
    config.max_command_attempts = 1;
    let client = get_test_client_with_config(config).await?;

    // The socket is torn down before the reply is matched. With the replay off,
    // the command is failed on the spot; the same command with `Some(true)` is
    // replayed and reaches the budget instead — see
    // `retryable_command_fails_after_max_command_attempts`.
    let result: Result<String> = timeout(
        Duration::from_secs(5),
        client.send(cmd("PING").kill_connection_on_read(1), None),
    )
    .await?;

    let error = result.unwrap_err();
    assert!(
        matches!(error.kind(), ErrorKind::DisconnectedByPeer),
        "expected DisconnectedByPeer, got {error:?}"
    );

    Ok(())
}

/// Neither branch of the network loop may drain without bound.
///
/// The `select!` gives the two directions one task between them, so a side that
/// keeps consuming until its source runs dry starves the other: a caller flooding
/// the channel delays every reply, and a firehose of replies delays every send.
/// Both waves are cut at `max_messages_per_wave`, which returns control to the
/// `select!` rather than to nothing in particular.
#[tokio::test]
#[serial]
async fn neither_side_of_the_loop_drains_without_bound() -> Result<()> {
    log_try_init();

    let hook = QueueMetricsTestHook::new();
    let mut config = get_default_config()?;
    config.queue_metrics_test_hook = Some(hook.clone());
    let cap = config.max_messages_per_wave;
    let client = get_test_client_with_config(config).await?;

    // Flood the channel synchronously, so the whole burst is waiting when the
    // network task next polls it.
    const BURST: usize = 2_000;
    assert!(
        BURST > cap * 4,
        "the burst must be large enough to be cut several times"
    );
    for i in 0..BURST {
        client.send_and_forget(cmd("PING").arg(i.to_string()), None)?;
    }

    // A round trip that only completes once the burst has been written and
    // answered, so both waves have been exercised by the time it returns.
    let _: String = timeout(Duration::from_secs(30), client.send(cmd("PING"), None)).await??;

    assert!(
        hook.write_wave_high_water() <= cap,
        "a send wave took {} messages, above the {cap} cap",
        hook.write_wave_high_water()
    );
    assert!(
        hook.read_wave_high_water() <= cap,
        "a read wave took {} replies, above the {cap} cap",
        hook.read_wave_high_water()
    );

    Ok(())
}
