use crate::{
    ClientError, Error, Result, RetryReason,
    client::{Client, ClientPreparedCommand, ReconnectionConfig},
    commands::{
        ClientReplyMode, ConnectionCommands, GenericCommands, PubSubCommands, StringCommands,
    },
    network::{SendBatchTestHook, sleep, timeout},
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

    assert!(
        matches!(
            result,
            Err(Error::Client(ClientError::MaxCommandAttemptsReached))
        ),
        "expected MaxCommandAttemptsReached, got {result:?}"
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
