use crate::{
    commands::{FlushingMode, ServerCommands, TopKCommands},
    tests::get_redis_stack_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_add() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.topk_reserve("key", 3, None).await?;

    let item: Vec<Option<String>> = client.topk_add("key", "baz").await?;
    assert_eq!(vec![None], item);

    let items: Vec<Option<String>> = client.topk_add("key", ["foo", "bar", "42"]).await?;
    assert_eq!(vec![None, None, Some("baz".to_owned())], items);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_incrby() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.topk_reserve("key", 3, None).await?;

    let items: Vec<Option<String>> = client
        .topk_incrby("key", [("foo", 3), ("bar", 2), ("42", 30)])
        .await?;
    assert_eq!(vec![None, None, None], items);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_info() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .topk_reserve("topk", 50, Some((200, 7, 0.925)))
        .await?;

    let info = client.topk_info("topk").await?;
    assert_eq!(50, info.k);
    assert_eq!(200, info.width);
    assert_eq!(7, info.depth);
    assert_eq!(0.925, info.decay);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_list() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.topk_reserve("key", 50, None).await?;

    let _items: Vec<Option<String>> = client.topk_add("key", ["foo", "bar", "42"]).await?;
    let _items: Vec<Option<String>> = client
        .topk_incrby("key", [("foo", 3), ("bar", 2), ("42", 30)])
        .await?;

    let items: Vec<String> = client.topk_list("key").await?;
    assert_eq!(
        vec!["42".to_owned(), "foo".to_owned(), "bar".to_owned()],
        items
    );

    let items: Vec<(String, usize)> = client.topk_list_with_count("key").await?;
    assert_eq!(
        vec![
            ("42".to_owned(), 31),
            ("foo".to_owned(), 4),
            ("bar".to_owned(), 3)
        ],
        items
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_query() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.topk_reserve("key", 50, None).await?;

    let _items: Vec<Option<String>> = client.topk_add("key", "42").await?;

    let items: Vec<bool> = client.topk_query("key", ["42", "unknown"]).await?;
    assert_eq!(vec![true, false], items);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_reserve() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .topk_reserve("topk", 50, Some((200, 7, 0.925)))
        .await?;

    Ok(())
}
