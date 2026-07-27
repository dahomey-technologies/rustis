use crate::{
    ClientError, Error, RedisError, RedisErrorKind, Result,
    client::{BatchPreparedCommand, Client, IntoConfig, ReconnectionConfig},
    commands::{
        ClusterCommands, ClusterNodeResult,
        ClusterSetSlotSubCommand::{self, Importing, Migrating, Node},
        ClusterShardResult, ConnectionCommands, FlushingMode, GenericCommands, HelloOptions,
        LegacyClusterNodeResult, LegacyClusterShardResult, MigrateOptions, ScriptingCommands,
        ServerCommands, StringCommands,
    },
    network::{ClusterConnection, ClusterTestHook, Version, timeout},
    resp::cmd,
    sleep, spawn,
    tests::{
        TestClient, get_cluster_test_client, get_cluster_test_client_with_command_timeout,
        get_default_host,
    },
};
use futures_util::try_join;
use serial_test::serial;
use std::{collections::HashSet, future::IntoFuture, time::Duration};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn no_request_policy_no_response_policy() -> Result<()> {
    let client = get_cluster_test_client().await?;

    client.set("key2", "value2").await?;
    let value: String = client.get("key2").await?;
    assert_eq!("value2", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn multi_shard_all_succeeded() -> Result<()> {
    let client = get_cluster_test_client().await?;

    client
        .mset([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;
    let values: Vec<String> = client.mget(["key1", "key2", "key3"]).await?;
    assert_eq!(3, values.len());
    assert_eq!("value1", values[0]);
    assert_eq!("value2", values[1]);
    assert_eq!("value3", values[2]);

    client
        .mset([
            ("key1{1}", "value1"),
            ("key2{2}", "value2"),
            ("key3{1}", "value3"),
        ])
        .await?;
    let values: Vec<String> = client.mget(["key1{1}", "key2{2}", "key3{1}"]).await?;
    assert_eq!(3, values.len());
    assert_eq!("value1", values[0]);
    assert_eq!("value2", values[1]);
    assert_eq!("value3", values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn all_shards_agg_sum() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.set("key1", "value1").await?;
    client.set("key2", "value2").await?;
    client.set("key3", "value3").await?;
    let dbsize = client.dbsize().await?;
    assert_eq!(3, dbsize);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn all_shards_one_succeeded() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.script_kill().await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::NotBusy,
            description: _
        }))
    ));

    let sha1: String = client
        .script_load("while (true) do end return ARGV[1]")
        .await?;

    spawn(async move {
        async fn blocking_script(sha1: String) -> Result<()> {
            let client = get_cluster_test_client().await?;

            let _ = client.evalsha::<String>(sha1, (), "hello").await?;

            Ok(())
        }

        let _ = blocking_script(sha1).await;
    });

    sleep(std::time::Duration::from_millis(100)).await;

    client.script_kill().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn all_shard_agg_logical_and() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let exists = client.script_exists("123456").await?;
    assert_eq!(1, exists.len());
    assert!(!exists[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn multi_shard_agg_min() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.set("key1", "value1").await?;
    let num_replicas = client.wait(1, 1000).await?;
    assert_eq!(1, num_replicas);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn all_shards_no_response_policy() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.set("key1", "value1").await?;
    client.set("key2", "value2").await?;
    client.set("key3", "value3").await?;

    let keys: HashSet<String> = client.keys("*").await?;
    assert_eq!(3, keys.len());
    assert!(keys.contains("key1"));
    assert!(keys.contains("key2"));
    assert!(keys.contains("key3"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn all_nodes_all_succeeded() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let sha1: String = client.script_load("return 12").await?;
    assert!(!sha1.is_empty());

    let value: i64 = client.evalsha(sha1, (), ()).await?;
    assert_eq!(12, value);

    Ok(())
}

/// Hands `slot` over from the shard served by `src_client` to the one served by
/// `dst_client`. The operation is symmetric: calling it with the two sides
/// swapped moves the slot back.
async fn migrate_slot(
    slot: u16,
    src_client: &Client,
    src_id: &str,
    dst_client: &Client,
    dst_id: &str,
) -> Result<()> {
    dst_client.cluster_setslot(slot, Importing(src_id)).await?;
    src_client.cluster_setslot(slot, Migrating(dst_id)).await?;
    dst_client.cluster_setslot(slot, Node(dst_id)).await?;
    src_client.cluster_setslot(slot, Node(dst_id)).await?;
    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn moved() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let hello_result = client.hello(HelloOptions::new(3)).await?;
    let version: Version = hello_result.version.as_str().try_into()?;

    let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
        ClusterConnection::convert_from_legacy_shard_description(client.cluster_slots().await?)
    } else {
        client.cluster_shards().await?
    };

    let slot = client.cluster_keyslot("key").await?;

    let src_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let dst_node = &shard_info_list
        .iter()
        .find(|s| s.slots.iter().all(|s| s.0 > slot || slot > s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    migrate_slot(slot, &src_client, src_id, &dst_client, dst_id).await?;

    // issue command on migrated slot
    let set_result = client.set("key", "value").await;
    let value: Result<String> = client.get("key").await;
    let del_result = client.del("key").await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test, and an early return here would strand the slot outside its
    // range, breaking unrelated tests in a way that is hard to trace back.
    migrate_slot(slot, &dst_client, dst_id, &src_client, src_id).await?;

    set_result?;
    del_result?;
    assert_eq!("value", value?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ask() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let hello_result = client.hello(HelloOptions::new(3)).await?;
    let version: Version = hello_result.version.as_str().try_into()?;

    let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
        ClusterConnection::convert_from_legacy_shard_description(client.cluster_slots().await?)
    } else {
        client.cluster_shards().await?
    };

    tracing::debug!("shard_info_list: {shard_info_list:?}");

    let slot = client.cluster_keyslot("key").await?;

    let src_node: &ClusterNodeResult = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let dst_node: &ClusterNodeResult = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 == 0))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for destination shard");
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // set key
    client.set("key", "value").await?;

    // Leave the slot in migrating/importing state and move the key across, so
    // the source answers ASK for it. This is deliberately only half of a slot
    // hand-over, hence not `migrate_slot`.
    dst_client.cluster_setslot(slot, Importing(src_id)).await?;
    src_client.cluster_setslot(slot, Migrating(dst_id)).await?;
    src_client
        .migrate(
            dst_node.ip.clone(),
            dst_node.port.unwrap(),
            "key",
            0,
            1000,
            MigrateOptions::default(),
        )
        .await?;

    // issue command on migrating slot
    let while_migrating: Result<String> = client.get("key").await;
    let cleanup_migrating = client.del("key").await;

    // finish migration
    dst_client.cluster_setslot(slot, Node(dst_id)).await?;
    src_client.cluster_setslot(slot, Node(dst_id)).await?;

    let set_migrated = client.set("key", "value").await;
    let once_migrated: Result<String> = client.get("key").await;
    let cleanup_migrated = client.del("key").await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test, and an early return here would strand the slot outside its
    // range, breaking unrelated tests in a way that is hard to trace back.
    migrate_slot(slot, &dst_client, dst_id, &src_client, src_id).await?;

    let set_restored = client.set("key", "value").await;
    let once_restored: Result<String> = client.get("key").await;
    let cleanup_restored = client.del("key").await;

    cleanup_migrating?;
    set_migrated?;
    cleanup_migrated?;
    set_restored?;
    cleanup_restored?;
    assert_eq!("value", while_migrating?);
    assert_eq!("value", once_migrated?);
    assert_eq!("value", once_restored?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn commands_to_different_nodes() -> Result<()> {
    // Assume test cluster has following slots split: [0 - 5460], [5461 - 10922], [10923 - 16383]
    let client = get_cluster_test_client_with_command_timeout().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.set("key0", "0").await?; // cluster keyslot key0 = 13252
    client.set("key1", "1").await?; // cluster keyslot key1 = 9189
    client.set("key2", "2").await?; // cluster keyslot key2 = 4998

    let (val0, val1, val2) = try_join!(
        client.get::<String>("key0").into_future(),
        client.get::<String>("key1").into_future(),
        client.get::<String>("key2").into_future(),
    )?;

    assert_eq!("0", val0);
    assert_eq!("1", val1);
    assert_eq!("2", val2);
    Ok(())
}

/// On a cluster reconnect, the in-flight `pending_requests` reference the old
/// per-node connections and can never be fulfilled. If they are not purged, the
/// stale request stuck at the front of the queue blocks every subsequent reply
/// from surfacing (`read()` pops the front only once all its sub-requests are
/// resolved) and every caller hangs. A follow-up command must still complete.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reconnect_purges_pending_requests_so_callers_do_not_hang() -> Result<()> {
    let host = get_default_host();
    let mut config =
        format!("redis+cluster://{host}:7000,{host}:7001,{host}:7002").into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    // Make the in-flight command retryable so it survives the reconnect purge
    // and is replayed, exercising the queue reconciliation.
    config.retry_on_error = true;
    let client = Client::connect(config).await?;

    // Send a command and close its node connection on the next read, before its
    // response is matched, so it is in flight when the cluster reconnect fires.
    client.send_and_forget(
        cmd("GET").arg("clu02_key").kill_connection_on_read(1),
        Some(true),
    )?;

    // Let the reconnection settle.
    sleep(Duration::from_millis(500)).await;

    // A follow-up command must receive its own reply rather than hang behind a
    // stale, never-fulfilled in-flight request.
    let echoed: String = timeout(
        Duration::from_secs(2),
        client.send(cmd("ECHO").arg("clu02_marker"), None),
    )
    .await??;

    assert_eq!(
        "clu02_marker", echoed,
        "the follow-up response must be routed to its own caller"
    );

    Ok(())
}

/// When a topology refresh removes a node while requests are in flight against
/// it, those requests are orphaned: their response can never arrive. Left in
/// the queue, an orphaned request stuck at the front blocks every subsequent
/// reply (`read()` pops the front only once all its sub-requests resolve) and
/// hangs all callers. Orphaned requests must instead surface as a retryable
/// error so the handler replays them against the refreshed topology.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn refresh_removing_a_node_does_not_hang_in_flight_callers() -> Result<()> {
    crate::tests::log_try_init();

    let cluster_hook = ClusterTestHook::new();

    let host = get_default_host();
    let mut config =
        format!("redis+cluster://{host}:7000,{host}:7001,{host}:7002").into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    config.retry_on_error = true;
    config.cluster_test_hook = Some(cluster_hook.clone());
    let client = Client::connect(config).await?;

    // Arm the node removal before issuing anything: it is consumed on the first
    // read that finds an in-flight request, so the command below is guaranteed
    // to be the one orphaned, with no timing assumption.
    cluster_hook.arm_drop_front_pending_node();

    // This command is in flight against the node that owns its key when that
    // node disappears from the topology, so its reply can never arrive.
    client.send_and_forget(cmd("GET").key("clu02_key"), None)?;

    // A follow-up caller must reach a verdict — a reply, or an error if it was
    // itself routed to the removed node — instead of hanging behind an orphaned
    // request that can never be fulfilled. Completing within the timeout is the
    // assertion: without the purge, this call never returns.
    let _: Result<String> = timeout(Duration::from_secs(3), client.send(cmd("PING"), None)).await?;

    Ok(())
}

/// A batch message is fed to the cluster as N independent requests. When one of
/// them is redirected (ASK/MOVED) the whole message is retried — but the
/// requests queued behind it must be discarded too. Otherwise their replies
/// still arrive, get matched FIFO against the retried message, and shift every
/// subsequent response by one.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn mid_batch_redirection_does_not_desync_following_responses() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let hello_result = client.hello(HelloOptions::new(3)).await?;
    let version: Version = hello_result.version.as_str().try_into()?;
    let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
        ClusterConnection::convert_from_legacy_shard_description(client.cluster_slots().await?)
    } else {
        client.cluster_shards().await?
    };

    let slot = client.cluster_keyslot("clu01_moved").await?;
    let src_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let dst_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().all(|s| s.0 > slot || slot > s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for destination shard");
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // Keys whose slots are left untouched by the migration below.
    client.set("clu01_a", "A").await?;
    client.set("clu01_b", "B").await?;

    // Hand the slot over to another shard. The batch client keeps its stale slot
    // map, so a command on that key is answered with a MOVED redirection.
    migrate_slot(slot, &src_client, src_id, &dst_client, dst_id).await?;

    // The redirected key's value must live on its new owner.
    dst_client.set("clu01_moved", "M").await?;

    // A batch whose *middle* command is redirected: the third request is the one
    // whose reply must not leak onto the retried message.
    let results = client
        .internal_send_batch(
            vec![
                cmd("GET").key("clu01_a").into(),
                cmd("GET").key("clu01_moved").into(),
                cmd("GET").key("clu01_b").into(),
            ],
            Some(true),
        )
        .await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test, and an early return here would leave a slot stranded.
    // The key goes first: a node still holding keys for a slot refuses to hand
    // that slot over to another node, and whether it still sees itself as the
    // owner at that instant depends on gossip timing.
    dst_client.del("clu01_moved").await?;
    migrate_slot(slot, &dst_client, dst_id, &src_client, src_id).await?;

    let values = results?
        .iter()
        .map(|response| response.to::<String>())
        .collect::<Result<Vec<_>>>()?;
    assert_eq!(
        vec!["A", "M", "B"],
        values,
        "each command of the batch must receive its own response, in order"
    );

    Ok(())
}

/// A multi-shard command is split into one sub-request per slot, and their replies
/// are aggregated. When a single sub-request is redirected, re-running the whole
/// command double-counts nothing but *under*-counts everything already applied: a
/// replayed `DEL` answers 0 for the keys its first attempt deleted. The caller then
/// receives a total that is silently wrong, reported as a success.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn partial_redirection_keeps_the_sub_results_already_obtained() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let hello_result = client.hello(HelloOptions::new(3)).await?;
    let version: Version = hello_result.version.as_str().try_into()?;
    let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
        ClusterConnection::convert_from_legacy_shard_description(client.cluster_slots().await?)
    } else {
        client.cluster_shards().await?
    };

    let slot = client.cluster_keyslot("clu04_moved").await?;
    let src_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let dst_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().all(|s| s.0 > slot || slot > s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for destination shard");
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // A key whose slot is left untouched by the migration below, so its own
    // sub-request succeeds on the first attempt.
    client.set("clu04_stable", "S").await?;

    // Hand the slot over to another shard. The client keeps its stale slot map,
    // so the sub-request carrying this key is answered with a MOVED redirection.
    migrate_slot(slot, &src_client, src_id, &dst_client, dst_id).await?;
    dst_client.set("clu04_moved", "M").await?;

    // Both keys exist, so both are deleted: the only correct answer is 2.
    let deleted: Result<usize> = client.del(["clu04_stable", "clu04_moved"]).await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test. The key goes first — see the migration test above.
    dst_client.del("clu04_moved").await?;
    migrate_slot(slot, &dst_client, dst_id, &src_client, src_id).await?;

    assert_eq!(
        2, deleted?,
        "a redirected sub-request must not discard the sub-results already obtained"
    );

    Ok(())
}

/// An ASK points at the node currently importing the slot, which the client may
/// never have heard of: unlike a MOVED, an ASK invalidates nothing, so nothing
/// else brings that node into the local topology. Resolving the target among the
/// known nodes only therefore fails the command outright, where the cluster spec
/// requires the redirection to be followed.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ask_to_an_unknown_node_is_followed_instead_of_failing() -> Result<()> {
    let probe = get_cluster_test_client().await?;
    probe.flushall(FlushingMode::Sync).await?;

    let hello_result = probe.hello(HelloOptions::new(3)).await?;
    let version: Version = hello_result.version.as_str().try_into()?;
    let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
        ClusterConnection::convert_from_legacy_shard_description(probe.cluster_slots().await?)
    } else {
        probe.cluster_shards().await?
    };

    let slot = probe.cluster_keyslot("clu05_key").await?;
    let src_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let dst_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().all(|s| s.0 > slot || slot > s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for destination shard");
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // The client under test starts with a topology that ignores the node the
    // slot is about to be imported by — the state a client is in when a node
    // joined, or was learned about, after its own discovery.
    let cluster_hook = ClusterTestHook::new();
    cluster_hook.hide_node_on_initial_discovery(dst_id);

    let host = get_default_host();
    let mut config =
        format!("redis+cluster://{host}:7000,{host}:7001,{host}:7002").into_config()?;
    config.cluster_test_hook = Some(cluster_hook.clone());
    let client = Client::connect(config).await?;

    client.set("clu05_key", "value").await?;

    // Leave the slot in migrating/importing state and move the key across, so
    // the source answers ASK for it. Half a hand-over on purpose, hence not
    // `migrate_slot`.
    dst_client.cluster_setslot(slot, Importing(src_id)).await?;
    src_client.cluster_setslot(slot, Migrating(dst_id)).await?;
    src_client
        .migrate(
            dst_node.ip.clone(),
            dst_node.port.unwrap(),
            "clu05_key",
            0,
            1000,
            MigrateOptions::default(),
        )
        .await?;

    let while_migrating: Result<String> = client.get("clu05_key").await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test. The half hand-over is completed first — the key now lives on
    // the destination, which only serves it once it owns the slot — then the key
    // is dropped, since a node still holding keys for a slot refuses to hand
    // that slot back.
    dst_client.cluster_setslot(slot, Node(dst_id)).await?;
    src_client.cluster_setslot(slot, Node(dst_id)).await?;
    dst_client.del("clu05_key").await?;
    migrate_slot(slot, &dst_client, dst_id, &src_client, src_id).await?;

    assert_eq!(
        "value", while_migrating?,
        "an ASK must be followed even to a node absent from the local topology"
    );

    Ok(())
}

/// A topology discovery that describes no usable node must be rejected, not
/// applied. Applying it empties the node list, and the next node lookup then
/// indexes an empty collection — panicking the network task, which owns all
/// routing state, and leaving the client permanently dead.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn empty_topology_discovery_is_rejected_instead_of_killing_the_client() -> Result<()> {
    let cluster_hook = ClusterTestHook::new();

    let host = get_default_host();
    let mut config =
        format!("redis+cluster://{host}:7000,{host}:7001,{host}:7002").into_config()?;
    config.cluster_test_hook = Some(cluster_hook.clone());
    let client = Client::connect(config).await?;

    let hello_result = client.hello(HelloOptions::new(3)).await?;
    let version: Version = hello_result.version.as_str().try_into()?;
    let shard_info_list: Vec<ClusterShardResult> = if version.major < 7 {
        ClusterConnection::convert_from_legacy_shard_description(client.cluster_slots().await?)
    } else {
        client.cluster_shards().await?
    };

    let slot = client.cluster_keyslot("clu09_key").await?;
    let src_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for source shard");
    let dst_node = shard_info_list
        .iter()
        .find(|s| s.slots.iter().all(|s| s.0 > slot || slot > s.1))
        .and_then(|s| s.nodes.iter().find(|n| n.role == "master"))
        .expect("No master found for destination shard");
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // Hand the slot over so the client, whose slot map is now stale, is answered
    // with a MOVED redirection — the trigger of a topology refresh.
    migrate_slot(slot, &src_client, src_id, &dst_client, dst_id).await?;

    // That refresh discovers an empty cluster.
    cluster_hook.arm_empty_topology_on_refresh();
    let _: Result<String> = client.send(cmd("GET").key("clu09_key"), None).await;

    // The client must still be alive. A keyless command picks a node at random,
    // which is precisely what indexes the node list.
    let pong = timeout(
        Duration::from_secs(3),
        client.send::<String>(cmd("PING"), None),
    )
    .await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test, and an early return here would leave a slot stranded.
    migrate_slot(slot, &dst_client, dst_id, &src_client, src_id).await?;

    assert_eq!(
        "PONG", pong??,
        "an unusable topology must surface as an error, not kill the network task"
    );

    Ok(())
}

#[test]
fn cluster_selslot_command() {
    let cmd = TestClient
        .cluster_setslot(
            12539,
            ClusterSetSlotSubCommand::Migrating("37618c7eec0dd58e946e1ef0df02d8c5a9a14235"),
        )
        .command;
    assert_eq!(
        "CLUSTER SETSLOT 12539 MIGRATING 37618c7eec0dd58e946e1ef0df02d8c5a9a14235",
        cmd.to_string()
    );
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_transaction() -> Result<()> {
    let client = get_cluster_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1{1}", "value1").forget();
    transaction.set("key2{1}", "value2").forget();
    transaction.get::<()>("key1{1}").queue();
    transaction.get::<()>("key2{1}").queue();
    let (value1, value2): (String, String) = transaction.execute().await?;

    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    let mut transaction = client.create_transaction();

    transaction.set("key{1}", "value").forget();
    transaction.get::<()>("key{1}").queue();
    let value: String = transaction.execute().await?;

    assert_eq!("value", value);

    Ok(())
}

/// A multi-shard command whose shards do not all succeed must surface that shard's
/// error to the caller. Reporting it as a disconnection instead makes the handler
/// reconnect the whole cluster and replay in-flight work, turning a routine
/// per-shard error into topology churn.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn per_shard_error_surfaces_to_the_caller_without_reconnecting() -> Result<()> {
    let admin = get_cluster_test_client().await?;

    // A user allowed to read one slot only. `clu03_a{1}` and `clu03_b{3}` are served
    // by two different masters, so MGET is split in two sub-requests of which exactly
    // one comes back as a NOPERM error frame.
    admin
        .acl_setuser(
            "clu03_user",
            ["reset", "on", ">clu03_pwd", "+@all", "%R~clu03_a{1}"],
        )
        .await?;
    admin.set("clu03_a{1}", "value").await?;

    let host = get_default_host();
    let client = Client::connect(format!(
        "redis+cluster://clu03_user:clu03_pwd@{host}:7000,{host}:7001,{host}:7002"
    ))
    .await?;
    let mut on_reconnect = client.on_reconnect();

    let result: Result<Vec<Option<String>>> = client.mget(["clu03_a{1}", "clu03_b{3}"]).await;

    // Restore the shared server state before asserting.
    admin.acl_deluser("clu03_user").await?;
    admin.del("clu03_a{1}").await?;

    assert!(
        matches!(&result, Err(Error::Redis(e)) if e.kind == RedisErrorKind::NoPerm),
        "the failing shard's error must reach the caller, got {result:?}"
    );
    assert!(
        on_reconnect.try_recv().is_err(),
        "a per-shard error must not trigger a cluster reconnection"
    );

    Ok(())
}

/// Redis Cluster only supports transactions whose keys all live in the same slot.
/// Commands are routed per key, so a cross-slot transaction would be split across
/// nodes: the ones outside the pinned node execute immediately, outside any MULTI.
/// That must be refused up front rather than reported as a successful transaction.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cross_slot_transaction_is_rejected_instead_of_losing_atomicity() -> Result<()> {
    let client = get_cluster_test_client().await?;

    client.del(["api01_a{1}", "api01_b{3}"]).await?;

    let mut transaction = client.create_transaction();
    transaction.set("api01_a{1}", "value1").forget();
    transaction.set("api01_b{3}", "value2").forget();
    let result: Result<()> = transaction.execute().await;

    assert!(
        matches!(result, Err(Error::Client(ClientError::CrossSlot))),
        "a cross-slot transaction must be refused, got {result:?}"
    );

    // Refused before sending: neither half may have been executed.
    let values: Vec<Option<String>> = client.mget(["api01_a{1}"]).await?;
    assert_eq!(vec![None], values);
    let values: Vec<Option<String>> = client.mget(["api01_b{3}"]).await?;
    assert_eq!(vec![None], values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cluster_pipeline() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1{1}", "value1").forget();
    pipeline.set("key2{1}", "value2").forget();
    pipeline.get::<()>("key1{1}").queue();
    pipeline.get::<()>("key2{1}").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}

/// Builds a `CLUSTER SLOTS` node entry; only the id matters to the conversion.
fn legacy_node(id: &str, port: u16) -> LegacyClusterNodeResult {
    LegacyClusterNodeResult {
        id: id.to_owned(),
        preferred_endpoint: "127.0.0.1".to_owned(),
        ip: "127.0.0.1".to_owned(),
        hostname: None,
        port,
    }
}

#[test]
fn a_legacy_shard_without_any_node_is_skipped_rather_than_indexed() {
    // A `CLUSTER SLOTS` entry that lists no node describes nothing routable. The
    // conversion reads each entry's first node to group slots by master, both
    // while sorting and while grouping — on the network task, where a panic
    // would take the whole client down with it.
    let converted = ClusterConnection::convert_from_legacy_shard_description(vec![
        LegacyClusterShardResult {
            slot: (0, 100),
            nodes: vec![],
        },
        LegacyClusterShardResult {
            slot: (101, 200),
            nodes: vec![legacy_node("node-a", 7000)],
        },
    ]);

    assert_eq!(1, converted.len());
    let shard = &converted[0];
    assert_eq!(vec![(101, 200)], shard.slots);
    assert_eq!("node-a", shard.nodes[0].id);
    assert_eq!("master", shard.nodes[0].role);
}

#[test]
fn legacy_shards_sharing_a_master_are_merged_into_one_shard() {
    // The grouping the skip above must not disturb: consecutive entries with the
    // same master accumulate their slot ranges, and the first node of each entry
    // is the master while the rest are replicas.
    let converted = ClusterConnection::convert_from_legacy_shard_description(vec![
        LegacyClusterShardResult {
            slot: (0, 100),
            nodes: vec![legacy_node("node-a", 7000), legacy_node("node-b", 7001)],
        },
        LegacyClusterShardResult {
            slot: (101, 200),
            nodes: vec![legacy_node("node-a", 7000)],
        },
        LegacyClusterShardResult {
            slot: (201, 300),
            nodes: vec![legacy_node("node-c", 7002)],
        },
    ]);

    assert_eq!(2, converted.len());
    assert_eq!(vec![(0, 100), (101, 200)], converted[0].slots);
    assert_eq!("node-a", converted[0].nodes[0].id);
    assert_eq!("master", converted[0].nodes[0].role);
    assert_eq!("replica", converted[0].nodes[1].role);
    assert_eq!(vec![(201, 300)], converted[1].slots);
    assert_eq!("node-c", converted[1].nodes[0].id);
}
