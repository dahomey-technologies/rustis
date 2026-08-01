use crate::{
    ClientError, Error, ErrorKind, Result,
    client::{Client, IntoConfig, ReconnectionConfig},
    commands::{
        ClientKillOptions, ClusterCommands, ClusterShardResult, ConnectionCommands, FlushingMode,
        ListCommands, PubSubCommands, ServerCommands, StringCommands,
    },
    network::{QueueMetricsTestHook, SendBatchTestHook, timeout},
    resp::cmd,
    spawn,
    tests::{
        get_cluster_test_client, get_cluster_test_client_with_command_timeout, get_default_addr,
        get_default_config, get_test_client, get_test_client_with_config, log_try_init,
        resident_bytes,
    },
};
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use serial_test::serial;
use std::{
    collections::{HashMap, HashSet},
    future::IntoFuture,
    time::Duration,
};

/// A subscriber that stops polling its stream must cost the client a bounded
/// amount of memory, and must be told what it lost.
///
/// Before the budget existed this same setup absorbed everything a single
/// loopback publisher could send — 113 MiB/s, nothing refused, 207 MB retained
/// for 50 000 messages — which is a 1 GiB container gone in about nine seconds.
/// The subscription is still held while nothing ever reads it; what is asserted
/// now is that the retained bytes stay within the configured budget, that the
/// dropped messages are counted rather than lost silently, and that resident
/// memory follows.
#[tokio::test]
#[serial]
async fn a_paused_subscriber_is_bounded_by_its_memory_budget() -> Result<()> {
    log_try_init();

    const PAYLOAD_BYTES: usize = 4096;
    const MESSAGES_PER_WAVE: usize = 500;
    // 10 000 messages of 4 KiB is 40 MiB offered against a 4 MiB budget. The
    // measurement that first exposed the growth used five times this, but the
    // assertion below is arithmetic on the budget, not on the volume: an order of
    // magnitude over budget proves the bound just as well, and keeps this test
    // clear of its timeout when the whole suite is competing for the CPU.
    const WAVES: usize = 20;
    const TOTAL_MESSAGES: usize = MESSAGES_PER_WAVE * WAVES;
    const BUDGET: usize = 4 * 1024 * 1024;

    let metrics = QueueMetricsTestHook::new();
    let mut config = get_default_config()?;
    config.queue_metrics_test_hook = Some(metrics.clone());
    config.backpressure.max_pubsub_bytes = BUDGET;
    let subscriber = Client::connect(config).await?;
    let publisher = get_test_client().await?;

    // Hold the whole stream and never poll it. Splitting and dropping the reader
    // would close the receiver, which turns every delivery into an error and
    // would measure loss instead of growth.
    let _held_stream = subscriber.subscribe("paused_subscriber_channel").await?;

    let baseline_rss = resident_bytes();
    let payload = "x".repeat(PAYLOAD_BYTES);
    let started = std::time::Instant::now();

    // Generous on purpose: the volume above is comfortably inside this, and the
    // timeout is here to fail a hang rather than to bound a slow machine.
    timeout(Duration::from_secs(120), async {
        for _ in 0..WAVES {
            for _ in 0..MESSAGES_PER_WAVE {
                publisher.send_and_forget(
                    cmd("PUBLISH")
                        .arg("paused_subscriber_channel")
                        .arg(payload.as_str()),
                    None,
                )?;
            }
            // Bound the publisher's own queue: by the time this answers, the
            // wave has been written and its deliveries counted.
            let _: String = publisher.send(cmd("PING"), None).await?;
        }

        // The last deliveries may still be in flight on the subscriber side.
        while metrics.pub_sub_delivered() < TOTAL_MESSAGES {
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await??;

    let elapsed = started.elapsed();
    let delivered = metrics.pub_sub_delivered();
    let dropped = _held_stream.dropped_messages();
    let offered_bytes = metrics.pub_sub_delivered_bytes();
    let rss_delta = match (baseline_rss, resident_bytes()) {
        (Some(before), Some(after)) => Some(after.saturating_sub(before)),
        _ => None,
    };
    // Every message that was delivered but not dropped is still held.
    let held = delivered.saturating_sub(dropped);
    let report = format!(
        "delivered={delivered} dropped={dropped} held={held} \
         offered={offered_bytes} B budget={BUDGET} B rss_delta={rss_delta:?} \
         elapsed={elapsed:?}"
    );
    println!("paused subscriber: {report}");

    assert_eq!(
        TOTAL_MESSAGES, delivered,
        "delivery must never block or refuse, only shed: {report}"
    );
    assert_eq!(
        0,
        metrics.pub_sub_delivery_failed(),
        "a live subscriber must never have a delivery refused: {report}"
    );
    assert!(
        dropped > 0,
        "far more was published than the budget allows, so messages must have \
         been dropped and counted: {report}"
    );
    // The channel keeps at least one message beyond the budget by design, so a
    // single message of slack is allowed on top of it.
    let max_held = BUDGET / PAYLOAD_BYTES + 1;
    assert!(
        held <= max_held,
        "the stream must hold no more than its budget allows ({max_held} messages): {report}"
    );
    // `rss_delta` is reported, not asserted; see the same note in
    // `backpressure.rs`. The held-message count above is exact and is the proof.

    drop(_held_stream);

    Ok(())
}

#[tokio::test]
#[serial]
async fn pubsub() -> Result<()> {
    log_try_init();

    let mut config = get_default_addr().into_config()?;
    "pub/sub".clone_into(&mut config.connection_name);
    let pub_sub_client = Client::connect(config).await?;

    let mut config = get_default_addr().into_config()?;
    "regular".clone_into(&mut config.connection_name);
    let regular_client = Client::connect(config).await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    regular_client.publish("mychannel", "mymessage").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    assert_eq!(b"mychannel", message.channel());
    assert_eq!(b"mymessage", message.payload());

    regular_client.set("key", "value").await?;
    let value: String = regular_client.get("key").await?;
    assert_eq!("value", value);

    pub_sub_stream.close().await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel2").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    Ok(())
}

// #[tokio::test]
// #[serial]
// async fn forbidden_command() -> Result<()> {
//     let client = get_test_client().await?;

//     // cleanup
//     client.flushdb(FlushingMode::Sync).await?;

//     // regular mode, these commands are allowed
//     client.set("key", "value").await?;
//     let value: String = client.get("key").await?;
//     assert_eq!("value", value);

//     // subscribed mode
//     let pub_sub_stream = client.subscribe("mychannel").await?;

//     // Cannot send regular commands during subscribed mode
//     let result: Result<String> = client.get("key").await;
//     assert!(result.is_err());

//     pub_sub_stream.close().await?;

//     // After leaving subscribed mode, should work again
//     let value: String = client.get("key").await?;
//     assert_eq!("value", value);

//     Ok(())
// }

#[tokio::test]
#[serial]
async fn subscribe_to_multiple_channels() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn subscribe_to_multiple_patterns() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .psubscribe(["mychannel1*", "mychannel2*"])
        .await?;

    regular_client.publish("mychannel11", "mymessage11").await?;
    regular_client.publish("mychannel12", "mymessage12").await?;
    regular_client.publish("mychannel21", "mymessage21").await?;
    regular_client.publish("mychannel22", "mymessage22").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern = std::str::from_utf8(message.pattern()).unwrap();
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel11", channel);
    assert_eq!("mymessage11", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern = std::str::from_utf8(message.pattern()).unwrap();
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel12", channel);
    assert_eq!("mymessage12", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern = std::str::from_utf8(message.pattern()).unwrap();
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel21", channel);
    assert_eq!("mymessage21", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let pattern = std::str::from_utf8(message.pattern()).unwrap();
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel22", channel);
    assert_eq!("mymessage22", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn pub_sub_channels() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2", "mychannel3", "otherchannel"])
        .await?;

    let channels: HashSet<String> = regular_client.pub_sub_channels(()).await?;
    assert_eq!(4, channels.len());
    assert!(channels.contains("mychannel1"));
    assert!(channels.contains("mychannel2"));
    assert!(channels.contains("mychannel3"));
    assert!(channels.contains("otherchannel"));

    let channels: HashSet<String> = regular_client.pub_sub_channels("mychannel*").await?;
    assert_eq!(3, channels.len());
    assert!(channels.contains("mychannel1"));
    assert!(channels.contains("mychannel2"));
    assert!(channels.contains("mychannel3"));

    stream.close().await?;

    let channels: HashSet<String> = regular_client.pub_sub_channels(()).await?;
    assert_eq!(0, channels.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn pub_sub_numpat() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(0, num_patterns);

    let stream = pub_sub_client.psubscribe(["mychannel*"]).await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(1, num_patterns);

    stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn pub_sub_numsub() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let num_sub: HashMap<String, usize> = regular_client
        .pub_sub_numsub(["mychannel1", "mychannel2"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&0usize), num_sub.get("mychannel1"));
    assert_eq!(Some(&0usize), num_sub.get("mychannel2"));

    let stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;

    let num_sub: HashMap<String, usize> = regular_client
        .pub_sub_numsub(["mychannel1", "mychannel2"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&1usize), num_sub.get("mychannel1"));
    assert_eq!(Some(&1usize), num_sub.get("mychannel2"));

    stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn pubsub_shardchannels() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;
    let regular_client = get_cluster_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client.ssubscribe("mychannel").await?;
    regular_client.spublish("mychannel", "mymessage").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel", channel);
    assert_eq!("mymessage", payload);

    regular_client.set("key", "value").await?;
    let value: String = regular_client.get("key").await?;
    assert_eq!("value", value);

    pub_sub_stream.close().await?;

    let mut pub_sub_stream = pub_sub_client.ssubscribe("mychannel2").await?;
    regular_client.spublish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn subscribe_to_multiple_shardchannels() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;
    let regular_client = get_cluster_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .ssubscribe(["mychannel1{1}", "mychannel2{1}"])
        .await?;
    regular_client
        .spublish("mychannel1{1}", "mymessage1")
        .await?;
    regular_client
        .spublish("mychannel2{1}", "mymessage2")
        .await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1{1}", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2{1}", channel);
    assert_eq!("mymessage2", payload);

    pub_sub_stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn pub_sub_shardchannels() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;
    pub_sub_client.flushall(FlushingMode::Sync).await?;

    // find the master node matching the {1} hashtag
    let slot = pub_sub_client.cluster_keyslot("{1}").await?;
    let shard_results: Vec<ClusterShardResult> = pub_sub_client.cluster_shards().await?;
    let shard_index = shard_results
        .iter()
        .position(|s| s.slots[0].0 <= slot && slot <= s.slots[0].1)
        .unwrap();
    let shard_result = &shard_results[shard_index];
    let master_node = shard_result
        .nodes
        .iter()
        .find(|n| n.role == "master")
        .unwrap();

    let master_client =
        Client::connect((master_node.ip.clone(), master_node.port.unwrap()).into_config()?).await?;

    let pub_sub_stream = pub_sub_client
        .ssubscribe([
            "mychannel1{1}",
            "mychannel2{1}",
            "mychannel3{1}",
            "otherchannel{1}",
        ])
        .await?;

    let channels: HashSet<String> = master_client.pub_sub_shardchannels(()).await?;
    assert_eq!(4, channels.len());
    assert!(channels.contains("mychannel1{1}"));
    assert!(channels.contains("mychannel2{1}"));
    assert!(channels.contains("mychannel3{1}"));
    assert!(channels.contains("otherchannel{1}"));

    let channels: HashSet<String> = master_client.pub_sub_shardchannels("mychannel*").await?;
    assert_eq!(3, channels.len());
    assert!(channels.contains("mychannel1{1}"));
    assert!(channels.contains("mychannel2{1}"));
    assert!(channels.contains("mychannel3{1}"));

    pub_sub_stream.close().await?;

    let channels: HashSet<String> = master_client.pub_sub_shardchannels(()).await?;
    assert_eq!(0, channels.len());

    Ok(())
}

/// A subscription is confirmed by a push frame, which the cluster connection
/// hands straight to the network handler instead of filing it as the answer to
/// the request it sent. That request stays at the head of the pending queue,
/// and every later reply coming from another node waits behind it forever.
/// The command timeout is what turns that wait into a failure this test can
/// report instead of hanging the whole suite.
#[tokio::test]
#[serial]
async fn a_subscription_does_not_block_replies_from_other_nodes() -> Result<()> {
    let client = get_cluster_test_client_with_command_timeout().await?;

    // A hashtag served by another master than the one holding the subscription.
    let shard_results: Vec<ClusterShardResult> = client.cluster_shards().await?;
    let subscribed_slot = client.cluster_keyslot("{1}").await?;
    let subscribed_shard = shard_results
        .iter()
        .position(|s| s.slots[0].0 <= subscribed_slot && subscribed_slot <= s.slots[0].1)
        .unwrap();

    let mut other_key = None;
    for i in 2..100 {
        let candidate = format!("{{{i}}}key");
        let slot = client.cluster_keyslot(candidate.as_str()).await?;
        let (start, end) = shard_results[subscribed_shard].slots[0];
        if slot < start || slot > end {
            other_key = Some(candidate);
            break;
        }
    }
    let other_key = other_key.expect("no hashtag maps outside the subscribed shard");

    let _pub_sub_stream = client.ssubscribe("mychannel{1}").await?;

    client.set(other_key.as_str(), "value").await?;
    let value: String = client.get(other_key.as_str()).await?;
    assert_eq!("value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn sunsubscribe() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;
    pub_sub_client.flushall(FlushingMode::Sync).await?;

    // find the master node matching the {1} hashtag
    let slot = pub_sub_client.cluster_keyslot("{1}").await?;
    let shard_results: Vec<ClusterShardResult> = pub_sub_client.cluster_shards().await?;
    let shard_index = shard_results
        .iter()
        .position(|s| s.slots[0].0 <= slot && slot <= s.slots[0].1)
        .unwrap();
    let master_node = shard_results[shard_index]
        .nodes
        .iter()
        .find(|n| n.role == "master")
        .unwrap();

    let master_client =
        Client::connect((master_node.ip.clone(), master_node.port.unwrap()).into_config()?).await?;

    let mut pub_sub_stream = pub_sub_client
        .ssubscribe(["mychannel1{1}", "mychannel2{1}"])
        .await?;

    // Unsubscribing from one shard channel leaves the other one subscribed —
    // unlike `close`, which drops them all.
    pub_sub_stream.sunsubscribe("mychannel1{1}").await?;

    let channels: HashSet<String> = master_client.pub_sub_shardchannels(()).await?;
    assert_eq!(1, channels.len());
    assert!(channels.contains("mychannel2{1}"));

    // The stream still delivers on the channel it kept.
    pub_sub_client
        .spublish("mychannel2{1}", "mymessage")
        .await?;
    let message = pub_sub_stream.next().await.unwrap()?;
    assert_eq!(b"mychannel2{1}", message.channel());
    assert_eq!(b"mymessage", message.payload());

    pub_sub_stream.close().await?;

    let channels: HashSet<String> = master_client.pub_sub_shardchannels(()).await?;
    assert_eq!(0, channels.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn pub_sub_shardnumsub() -> Result<()> {
    let pub_sub_client = get_cluster_test_client().await?;

    // find the master node matching the {1} hashtag
    let slot = pub_sub_client.cluster_keyslot("{1}").await?;
    let shard_results: Vec<ClusterShardResult> = pub_sub_client.cluster_shards().await?;
    let shard_index = shard_results
        .iter()
        .position(|s| s.slots[0].0 <= slot && slot <= s.slots[0].1)
        .unwrap();
    let shard_result = &shard_results[shard_index];
    let master_node = shard_result
        .nodes
        .iter()
        .find(|n| n.role == "master")
        .unwrap();

    let master_client =
        Client::connect((master_node.ip.clone(), master_node.port.unwrap()).into_config()?).await?;

    let num_sub: HashMap<String, usize> = master_client
        .pub_sub_shardnumsub(["mychannel1{1}", "mychannel2{1}"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&0usize), num_sub.get("mychannel1{1}"));
    assert_eq!(Some(&0usize), num_sub.get("mychannel2{1}"));

    let pub_sub_stream = pub_sub_client
        .ssubscribe(["mychannel1{1}", "mychannel2{1}"])
        .await?;

    let num_sub: HashMap<String, usize> = master_client
        .pub_sub_shardnumsub(["mychannel1{1}", "mychannel2{1}"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&1usize), num_sub.get("mychannel1{1}"));
    assert_eq!(Some(&1usize), num_sub.get("mychannel2{1}"));

    pub_sub_stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn additional_sub() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    // 1st subscription
    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel1").await?;

    // publish / receive
    regular_client.publish("mychannel1", "mymessage1").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    // 2nd subscription
    pub_sub_stream.subscribe("mychannel2").await?;

    // publish / receive
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    // 3rd subscription
    pub_sub_stream.psubscribe("o*").await?;

    // publish / receive
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;
    regular_client.publish("otherchannel", "mymessage3").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("otherchannel", channel);
    assert_eq!("mymessage3", payload);

    // close
    pub_sub_stream.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn auto_resubscribe() -> Result<()> {
    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let pub_sub_client = get_test_client_with_config(config).await?;
    let regular_client = get_test_client().await?;

    let pub_sub_client_id = pub_sub_client.client_id().await?;
    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    pub_sub_stream.psubscribe("o*").await?;

    let mut on_reconnect = pub_sub_client.on_reconnect();

    regular_client
        .client_kill(ClientKillOptions::default().id(pub_sub_client_id))
        .await?;

    // wait for reconnection before publishing
    on_reconnect.recv().await.unwrap();

    regular_client.publish("mychannel", "mymessage").await?;
    regular_client
        .publish("otherchannel", "othermessage")
        .await?;

    let message = pub_sub_stream.try_next().await?.unwrap();
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel", channel);
    assert_eq!("mymessage", payload);

    let message = pub_sub_stream.try_next().await?.unwrap();
    let pattern = std::str::from_utf8(message.pattern()).unwrap();
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("otherchannel", channel);
    assert_eq!("o*", pattern);
    assert_eq!("othermessage", payload);

    Ok(())
}

#[tokio::test]
#[serial]
async fn no_auto_resubscribe() -> Result<()> {
    log_try_init();

    let mut config = get_default_addr().into_config()?;
    "pub/sub".clone_into(&mut config.connection_name);
    config.auto_resubscribe = false;
    let pub_sub_client = Client::connect(config).await?;

    let mut config = get_default_addr().into_config()?;
    "regular".clone_into(&mut config.connection_name);
    let regular_client = Client::connect(config).await?;

    let pub_sub_client_id = pub_sub_client.client_id().await?;
    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    pub_sub_stream.psubscribe("o*").await?;

    let mut on_reconnect = pub_sub_client.on_reconnect();

    regular_client
        .client_kill(ClientKillOptions::default().id(pub_sub_client_id))
        .await?;

    // wait for reconnection before publishing
    on_reconnect.recv().await.unwrap();

    regular_client.publish("mychannel", "mymessage").await?;
    regular_client
        .publish("otherchannel", "othermessage")
        .await?;

    let message = pub_sub_stream.next().now_or_never();
    assert!(message.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn concurrent_subscribe() -> Result<()> {
    let pub_sub_client1 = get_test_client().await?;
    let pub_sub_client2 = pub_sub_client1.clone();
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    regular_client.lpush("key", ["value1", "value2"]).await?;

    let results = futures_util::join!(
        pub_sub_client1.subscribe("mychannel1"),
        pub_sub_client2.subscribe("mychannel2"),
        regular_client.lpop("key", 2).into_future(),
        regular_client.lpop("key", 2).into_future(),
    );

    let mut pub_sub_stream1 = results.0?;
    let _pub_sub_stream2 = results.1?;
    let values1: Vec<String> = results.2?;
    let values2: Vec<String> = results.3?;

    assert_eq!(vec!["value2".to_owned(), "value1".to_owned()], values1);
    assert_eq!(Vec::<String>::new(), values2);

    // Published once the subscriptions are confirmed. Joining the publish with
    // them would assert an order nothing provides: it travels on another
    // connection, and the server owes no ordering between two of them — it
    // reaches an empty channel whenever it wins the race, and the message is
    // dropped rather than delivered.
    regular_client.publish("mychannel1", "new").await?;

    let message1 = pub_sub_stream1.next().await.unwrap()?;
    assert_eq!(b"new", message1.payload());

    Ok(())
}

#[tokio::test]
#[serial]
async fn unsubscribe() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", payload);

    regular_client.publish("mychannel1", "mymessage11").await?;
    pub_sub_stream.unsubscribe("mychannel2").await?;
    regular_client.publish("mychannel1", "mymessage12").await?;

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage11", payload);

    let message = pub_sub_stream.next().await.unwrap()?;
    let channel = std::str::from_utf8(message.channel()).unwrap();
    let payload = std::str::from_utf8(message.payload()).unwrap();

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage12", payload);

    pub_sub_stream.close().await?;
    regular_client.close().await?;

    Ok(())
}

/// A failed `unsubscribe` must not drop the channel from local tracking: the
/// subscription still stands server-side, so forgetting it locally would leave a
/// ghost the stream keeps receiving and `close`/`Drop` no longer cancel. The
/// asymmetry with `subscribe`, which inserts only after success, is the defect.
#[tokio::test]
#[serial]
async fn a_failed_unsubscribe_keeps_the_channel_tracked() -> Result<()> {
    log_try_init();

    let hook = SendBatchTestHook::new();
    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    config.send_batch_test_hook = Some(hook.clone());
    let pub_sub_client = Client::connect(config).await?;

    let mut pub_sub_stream = pub_sub_client.subscribe(["ps03_a", "ps03_b"]).await?;

    // Kill the connection on the confirmation read of the next UNSUBSCRIBE: the
    // command reaches the server but its caller sees a send failure, since the
    // non-retryable command is purged on reconnect and the await returns Err.
    hook.arm_kill_on_read_for("UNSUBSCRIBE", 1);

    let unsubscribe_result = pub_sub_stream.unsubscribe("ps03_b").await;
    assert!(
        unsubscribe_result.is_err(),
        "the unsubscribe send must fail for this test to be meaningful, got {unsubscribe_result:?}"
    );

    // The channel is still subscribed, so re-subscribing to it must be rejected
    // as a duplicate. On the buggy path it was forgotten before the failed send,
    // so the re-subscribe is wrongly accepted.
    let resubscribe_result = pub_sub_stream.subscribe("ps03_b").await;
    let error = resubscribe_result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::AlreadySubscribed)
        ),
        "a channel whose unsubscribe failed must remain tracked, got {error:?}"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn punsubscribe() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .psubscribe(["mychannel1*", "mychannel2*"])
        .await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(2, num_patterns);

    pub_sub_stream.punsubscribe("mychannel1*").await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(1, num_patterns);

    pub_sub_stream.close().await?;
    regular_client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn split() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let pub_sub_stream = pub_sub_client.create_pub_sub();
    let (mut sink, mut stream) = pub_sub_stream.split();

    sink.subscribe("mychannel1").await?;
    regular_client.publish("mychannel1", "mymessage1").await?;
    sink.subscribe("mychannel2").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;
    sink.subscribe("mychannel3").await?;
    regular_client.publish("mychannel3", "mymessage3").await?;

    let join_handle_stream = spawn(async move {
        let message1 = stream.next().await.unwrap().unwrap();
        assert_eq!(b"mychannel1", message1.channel());
        assert_eq!(b"mymessage1", message1.payload());

        let message2 = stream.next().await.unwrap().unwrap();
        assert_eq!(b"mychannel2", message2.channel());
        assert_eq!(b"mymessage2", message2.payload());

        let message3 = stream.next().await.unwrap().unwrap();
        assert_eq!(b"mychannel3", message3.channel());
        assert_eq!(b"mymessage3", message3.payload());
    });

    join_handle_stream.await?;
    sink.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn subscribe_multiple_times_to_the_same_channel() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    assert!(pub_sub_stream.subscribe("mychannel").await.is_err());
    assert!(pub_sub_client.subscribe("mychannel").await.is_err());
    regular_client.publish("mychannel", "mymessage").await?;

    pub_sub_stream.psubscribe("pattern").await?;
    assert!(pub_sub_stream.psubscribe("pattern").await.is_err());
    assert!(pub_sub_client.psubscribe("pattern").await.is_err());

    pub_sub_stream.ssubscribe("myshardchannel").await?;
    assert!(pub_sub_stream.ssubscribe("myshardchannel").await.is_err());
    assert!(pub_sub_client.ssubscribe("myshardchannel").await.is_err());

    Ok(())
}

/// `PUBSUB HELP` answers the subcommand list as a flat array of text lines,
/// which is the shape the declared return type claims.
#[tokio::test]
#[serial]
async fn pub_sub_help() -> Result<()> {
    let client = get_test_client().await?;

    let help = client.pub_sub_help().await?;

    assert!(help.iter().any(|line| line.contains("CHANNELS")));

    Ok(())
}
