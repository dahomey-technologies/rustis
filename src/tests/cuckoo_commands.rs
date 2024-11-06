use crate::{
    commands::{
        CfInsertOptions, CfReserveOptions, CfScanDumpResult, CuckooCommands, FlushingMode,
        ServerCommands, StringCommands,
    },
    tests::get_redis_stack_test_client,
    Error, RedisError, RedisErrorKind, Result,
};
use serial_test::serial;
use std::collections::VecDeque;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_add() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cf_add("key", "item1").await?;

    // Cuckoo filters can contain the same item multiple times
    client.cf_add("key", "item1").await?;

    client.cf_add("key", "item2").await?;

    client.set("key", "item").await?;
    let result = client.cf_add("key", "item").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::WrongType,
            description: _
        }))
    ));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_addnx() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.cf_addnx("key", "item").await?;
    assert!(result);

    let result = client.cf_addnx("key", "item").await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_count() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cf_add("key", "item1").await?;
    client.cf_add("key", "item1").await?;
    client.cf_add("key", "item1").await?;

    let count = client.cf_count("key", "item1").await?;
    assert_eq!(3, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_del() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cf_add("key", "item1").await?;
    client.cf_add("key", "item1").await?;
    client.cf_add("key", "item1").await?;

    let deleted = client.cf_del("key", "item1").await?;
    assert!(deleted);
    let deleted = client.cf_del("key", "item1").await?;
    assert!(deleted);
    let deleted = client.cf_del("key", "item1").await?;
    assert!(deleted);
    let deleted = client.cf_del("key", "item1").await?;
    assert!(!deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_exists() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let exists = client.cf_exists("key", "item1").await?;
    assert!(!exists);

    client.cf_add("key", "item1").await?;

    let exists = client.cf_exists("key", "item1").await?;
    assert!(exists);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_info() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cf_add("key", "item1").await?;
    client.cf_add("key", "item2").await?;
    client.cf_add("key", "item3").await?;
    client.cf_add("key", "item4").await?;
    client.cf_del("key", "item1").await?;

    let info = client.cf_info("key").await?;
    log::debug!("info: {info:?}");
    assert_eq!(3, info.num_items_inserted);
    assert_eq!(1, info.num_items_deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_insert() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .cf_insert(
            "key",
            CfInsertOptions::default().capacity(2048),
            ["item1", "item2", "item3"],
        )
        .await?;

    let result = client
        .cf_insert(
            "key2",
            CfInsertOptions::default().nocreate(),
            ["item1", "item2", "item3"],
        )
        .await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_insertnx() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let results: Vec<i64> = client
        .cf_insertnx(
            "key",
            CfInsertOptions::default(),
            ["item1", "item2", "item3"],
        )
        .await?;
    assert_eq!(vec![1, 1, 1], results);

    let results: Vec<i64> = client
        .cf_insertnx(
            "key",
            CfInsertOptions::default(),
            ["item3", "item4", "item5"],
        )
        .await?;
    assert_eq!(vec![0, 1, 1], results);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_reserve_loadchunk_scandump() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .cf_reserve("cf", 10, CfReserveOptions::default())
        .await?;

    client.cf_add("cf", "item1").await?;

    let mut iterator: i64 = 0;
    let mut chunks: VecDeque<CfScanDumpResult> = VecDeque::new();

    loop {
        let result = client.cf_scandump("cf", iterator).await?;

        if result.iterator == 0 {
            break;
        } else {
            iterator = result.iterator;
            chunks.push_back(result);
        }
    }

    client.flushall(FlushingMode::Sync).await?;

    while let Some(dump) = chunks.pop_front() {
        client.cf_loadchunk("cf", dump.iterator, dump.data).await?;
    }

    let result = client.cf_exists("cf", "item1").await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cf_mexists() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .cf_insert("filter", CfInsertOptions::default(), ["item1", "item2"])
        .await?;

    let results: [bool; 3] = client
        .cf_mexists("filter", ["item1", "item2", "item3"])
        .await?;
    assert_eq!([true, true, false], results);

    Ok(())
}
