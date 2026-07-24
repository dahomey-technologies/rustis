use crate::{
    Error, RedisError, RedisErrorKind, Result,
    client::{BatchPreparedCommand, Client, IntoConfig, ReconnectionConfig},
    commands::{
        ClusterCommands, ClusterNodeResult,
        ClusterSetSlotSubCommand::{self, Importing, Migrating, Node},
        ClusterShardResult, ConnectionCommands, FlushingMode, GenericCommands, HelloOptions,
        MigrateOptions, ScriptingCommands, ServerCommands, StringCommands,
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
use smallvec::smallvec;
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

    log::debug!("shard_info_list: {shard_info_list:?}");

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

/// test reconnection to replica when master is stopped
/// master stop is not automated but must be done manually
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
#[ignore]
async fn get_loop() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.set("key", "value").await?;

    for _ in 0..1000 {
        let _value: Result<String> = client.get("key").await;
        sleep(Duration::from_secs(1)).await;
    }

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
            smallvec![
                cmd("GET").key("clu01_a").into(),
                cmd("GET").key("clu01_moved").into(),
                cmd("GET").key("clu01_b").into(),
            ],
            Some(true),
        )
        .await;

    // Restore the topology before asserting: the cluster is shared with every
    // other test, and an early return here would leave a slot stranded.
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
