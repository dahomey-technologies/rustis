use crate::{
    Result,
    client::Client,
    network::SendBatchTestHook,
    resp::cmd,
    tests::{get_default_config, get_default_port, log_try_init},
    RetryReason,
};
use serial_test::serial;

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
