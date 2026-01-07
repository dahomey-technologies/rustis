use crate::{
    Error, RedisError, RedisErrorKind, Result,
    client::Client,
    commands::{
        CallBuilder, ClusterCommands, ClusterNodeResult,
        ClusterSetSlotSubCommand::{self, Importing, Migrating, Node},
        ClusterShardResult, ConnectionCommands, FlushingMode, GenericCommands, HelloOptions,
        MigrateOptions, ScriptingCommands, ServerCommands, StringCommands,
    },
    network::{ClusterConnection, Version},
    sleep, spawn,
    tests::{TestClient, get_cluster_test_client, get_cluster_test_client_with_command_timeout},
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

            let _ = client
                .evalsha::<String>(CallBuilder::sha1(&sha1).args("hello"))
                .await?;

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

    let value: i64 = client.evalsha(CallBuilder::sha1(&sha1)).await?;
    assert_eq!(12, value);

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

    let src_node = &shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .unwrap()
        .nodes[0];
    let dst_node = &shard_info_list
        .iter()
        .find(|s| s.slots.iter().all(|s| s.0 > slot || slot > s.1))
        .unwrap()
        .nodes[0];
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // migrate
    dst_client.cluster_setslot(slot, Importing(src_id)).await?;

    src_client.cluster_setslot(slot, Migrating(dst_id)).await?;

    dst_client.cluster_setslot(slot, Node(dst_id)).await?;

    src_client.cluster_setslot(slot, Node(dst_id)).await?;

    // issue command on migrated slot
    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

    // migrate back
    src_client.cluster_setslot(slot, Importing(dst_id)).await?;

    dst_client.cluster_setslot(slot, Migrating(src_id)).await?;

    src_client.cluster_setslot(slot, Node(src_id)).await?;

    dst_client.cluster_setslot(slot, Node(src_id)).await?;

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

    let src_node: &ClusterNodeResult = &shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 <= slot && slot <= s.1))
        .unwrap()
        .nodes[0];
    let dst_node: &ClusterNodeResult = &shard_info_list
        .iter()
        .find(|s| s.slots.iter().any(|s| s.0 == 0))
        .unwrap()
        .nodes[0];
    let src_id = &src_node.id;
    let dst_id = &dst_node.id;
    let src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // set key
    client.set("key", "value").await?;

    // migrate
    dst_client.cluster_setslot(slot, Importing(src_id)).await?;

    src_client.cluster_setslot(slot, Migrating(dst_id)).await?;

    // migrate key
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
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

    // finish migration
    dst_client.cluster_setslot(slot, Node(dst_id)).await?;

    src_client.cluster_setslot(slot, Node(dst_id)).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

    // migrate back
    src_client.cluster_setslot(slot, Importing(dst_id)).await?;

    dst_client.cluster_setslot(slot, Migrating(src_id)).await?;

    src_client.cluster_setslot(slot, Node(src_id)).await?;

    dst_client.cluster_setslot(slot, Node(src_id)).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

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

#[test]
fn cluster_selslot_command() {
    let cmd = TestClient.cluster_setslot(12539, ClusterSetSlotSubCommand::Migrating("37618c7eec0dd58e946e1ef0df02d8c5a9a14235")).command;
    assert_eq!("CLUSTER SETSLOT 12539 MIGRATING 37618c7eec0dd58e946e1ef0df02d8c5a9a14235", cmd.to_string());
}
