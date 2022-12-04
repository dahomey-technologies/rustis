use crate::{
    client::Client,
    commands::{
        CallBuilder, ClusterCommands, ClusterNodeResult,
        ClusterSetSlotSubCommand::{Importing, Migrating, Node},
        ClusterShardResult, FlushingMode, GenericCommands, MigrateOptions, ScriptingCommands,
        ServerCommands, StringCommands,
    },
    sleep, spawn,
    tests::get_cluster_test_client,
    Error, RedisError, RedisErrorKind, Result,
};
use serial_test::serial;
use std::collections::HashSet;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn no_request_policy_no_response_policy() -> Result<()> {
    let mut client = get_cluster_test_client().await?;

    client.set("key2", "value2").await?;
    let value: String = client.get("key2").await?;
    assert_eq!("value2", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn multi_shard_all_succeeded() -> Result<()> {
    let mut client = get_cluster_test_client().await?;

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
    let mut client = get_cluster_test_client().await?;
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
    let mut client = get_cluster_test_client().await?;
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
            let mut client = get_cluster_test_client().await?;

            let _ = client
                .evalsha::<String>(CallBuilder::sha1(sha1).args("hello"))
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
    let mut client = get_cluster_test_client().await?;
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
    let mut client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client
        .msetnx([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn all_shards_no_response_policy() -> Result<()> {
    let mut client = get_cluster_test_client().await?;
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
    let mut client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let sha1: String = client.script_load("return 12").await?;
    assert!(!sha1.is_empty());

    let value: i64 = client.evalsha(CallBuilder::sha1(sha1)).await?;
    assert_eq!(12, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn moved() -> Result<()> {
    let mut client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let shard_info_list: Vec<ClusterShardResult> = client.cluster_shards().await?;

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
    let mut src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let mut dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // migrate
    dst_client
        .cluster_setslot(
            slot,
            Importing {
                node_id: src_id.clone(),
            },
        )
        .await?;

    src_client
        .cluster_setslot(
            slot,
            Migrating {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    dst_client
        .cluster_setslot(
            slot,
            Node {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    src_client
        .cluster_setslot(
            slot,
            Node {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    // issue command on migrated slot
    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

    // migrate back
    src_client
        .cluster_setslot(
            slot,
            Importing {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    dst_client
        .cluster_setslot(
            slot,
            Migrating {
                node_id: src_id.clone(),
            },
        )
        .await?;

    src_client
        .cluster_setslot(
            slot,
            Node {
                node_id: src_id.clone(),
            },
        )
        .await?;

    dst_client
        .cluster_setslot(
            slot,
            Node {
                node_id: src_id.clone(),
            },
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ask() -> Result<()> {
    let mut client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let shard_info_list: Vec<ClusterShardResult> = client.cluster_shards().await?;

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
    let mut src_client = Client::connect((src_node.ip.clone(), src_node.port.unwrap())).await?;
    let mut dst_client = Client::connect((dst_node.ip.clone(), dst_node.port.unwrap())).await?;

    // set key
    client.set("key", "value").await?;

    // migrate
    dst_client
        .cluster_setslot(
            slot,
            Importing {
                node_id: src_id.clone(),
            },
        )
        .await?;

    src_client
        .cluster_setslot(
            slot,
            Migrating {
                node_id: dst_id.clone(),
            },
        )
        .await?;

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
    dst_client
        .cluster_setslot(
            slot,
            Node {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    src_client
        .cluster_setslot(
            slot,
            Node {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

    // migrate back
    src_client
        .cluster_setslot(
            slot,
            Importing {
                node_id: dst_id.clone(),
            },
        )
        .await?;

    dst_client
        .cluster_setslot(
            slot,
            Migrating {
                node_id: src_id.clone(),
            },
        )
        .await?;

    src_client
        .cluster_setslot(
            slot,
            Node {
                node_id: src_id.clone(),
            },
        )
        .await?;

    dst_client
        .cluster_setslot(
            slot,
            Node {
                node_id: src_id.clone(),
            },
        )
        .await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);
    client.del("key").await?;

    Ok(())
}
