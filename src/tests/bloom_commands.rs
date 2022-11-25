use std::collections::VecDeque;

use crate::{
    tests::get_redis_stack_test_client,
    FlushingMode, Result, ServerCommands, BloomCommands, BfInfoParameter, BfInsertOptions, BfReserveOptions, resp::BulkString,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_add() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.bf_add("key", "item").await?;
    assert!(result);

    let result = client.bf_add("key", "item").await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_exists() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.bf_exists("key", "item").await?;
    assert!(!result);   

    let result = client.bf_add("key", "item").await?;
    assert!(result);

    let result = client.bf_exists("key", "item").await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_info() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.bf_add("key", "item1").await?;
    client.bf_add("key", "item2").await?;
    client.bf_add("key", "item3").await?;

    let result = client.bf_info("key", BfInfoParameter::NumItemsInserted).await?;
    assert_eq!(3, result);

    let result = client.bf_info_all("key").await?;
    assert_eq!(3, result.num_items_inserted);
    assert_eq!(1, result.num_filters);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_insert() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let results: Vec<bool> = client.bf_insert("filter", ["boo", "bar", "barz"], BfInsertOptions::default()).await?;
    assert_eq!(vec![true, true, true], results);

    let results: Vec<bool> = client.bf_insert("filter", "hello", BfInsertOptions::default().capacity(1000)).await?;
    assert_eq!(vec![true], results);

    let results: Vec<bool> = client.bf_insert("filter", ["boo", "bar"], BfInsertOptions::default().nocreate()).await?;
    assert_eq!(vec![false, false], results);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_madd() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let results: Vec<bool> = client.bf_madd("filter", ["item1", "item2"]).await?;
    assert_eq!(vec![true, true], results);

    let results: Vec<bool> = client.bf_madd("filter", ["item2", "item3"]).await?;
    assert_eq!(vec![false, true], results);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_mexists() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let results: [bool; 2] = client.bf_madd("filter", ["item1", "item2"]).await?;
    assert_eq!([true, true], results);

    let results: [bool; 3] = client.bf_mexists("filter", ["item1", "item2", "item3"]).await?;
    assert_eq!([true, true, false], results);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bf_reserve_loadchunk_scandump() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.bf_reserve("bf", 0.1, 10, BfReserveOptions::default()).await?;

    let result = client.bf_add("bf", "item1").await?;
    assert!(result);

    let mut iterator: i64 = 0;
    let mut chunks: VecDeque<(i64, Vec<u8>)> = VecDeque::new();

    loop {
        let result = client.bf_scandump("bf", iterator).await?;

        if result.0 == 0 {
            break;
        } else {
            iterator = result.0;
            chunks.push_back(result);
        }
    }

    client.flushall(FlushingMode::Sync).await?;

    while let Some((iterator, chunk)) = chunks.pop_front() {
        client.bf_loadchunk("bf", iterator, BulkString::from_bytes(chunk)).await?;
    }

    let result = client.bf_exists("bf", "item1").await?;
    assert!(result);

    Ok(())
}