// Every test in this module drives test-only, `debug_assertions`-gated
// infrastructure (the send-batch hook, `kill_connection_on_read`). In release
// builds that infrastructure is compiled out, so the whole module must be too.
#![cfg(debug_assertions)]

use crate::{
    Result, RetryReason,
    client::{Client, ReconnectionConfig},
    commands::{GenericCommands, StringCommands},
    network::{SendBatchTestHook, sleep, timeout},
    resp::cmd,
    tests::{
        get_default_config, get_default_port, get_test_client, get_test_client_with_config,
        log_try_init,
    },
};
use serial_test::serial;
use std::time::Duration;

/// Retry reasons accumulated for one message must not be applied to the other
/// messages sharing the same send batch: each message must be fed only with
/// its own reasons.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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
