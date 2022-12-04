use crate::{
    commands::{CountMinSketchCommands, FlushingMode, ServerCommands},
    tests::get_redis_stack_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cms_incrby() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result: Result<Vec<usize>> = client.cms_incrby("key", [("item1", 1), ("item2", 2)]).await;
    assert!(result.is_err()); // key does not exist

    client.cms_initbydim("key", 2000, 5).await?;

    let result: Vec<usize> = client
        .cms_incrby("key", [("item1", 1), ("item2", 2)])
        .await?;
    assert_eq!(vec![1, 2], result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cms_info() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cms_initbydim("key", 2000, 5).await?;

    let result: Vec<usize> = client
        .cms_incrby("key", [("item1", 1), ("item2", 2)])
        .await?;
    assert_eq!(vec![1, 2], result);

    let info = client.cms_info("key").await?;
    assert_eq!(2000, info.width);
    assert_eq!(5, info.depth);
    assert_eq!(3, info.total_count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cms_initbydim() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cms_initbydim("key", 2000, 5).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cms_initbyprob() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cms_initbyprob("key", 0.001, 0.01).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cms_merge() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cms_initbydim("key1", 2000, 5).await?;

    let result: Vec<usize> = client
        .cms_incrby("key1", [("item1", 1), ("item2", 2)])
        .await?;
    assert_eq!(vec![1, 2], result);

    client.cms_initbydim("key2", 2000, 5).await?;

    let result: Vec<usize> = client
        .cms_incrby("key2", [("item1", 1), ("item2", 2)])
        .await?;
    assert_eq!(vec![1, 2], result);

    client.cms_initbydim("key3", 2000, 5).await?;

    client
        .cms_merge("key3", ["key1", "key2"], Some([1, 2]))
        .await?;
    let info = client.cms_info("key3").await?;
    assert_eq!(9, info.total_count);

    client.cms_initbydim("key4", 2000, 5).await?;

    client
        .cms_merge("key4", ["key1", "key2"], Option::<usize>::None)
        .await?;
    let info = client.cms_info("key4").await?;
    assert_eq!(6, info.total_count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cms_query() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.cms_initbydim("key1", 2000, 5).await?;

    let _result: Vec<usize> = client
        .cms_incrby("key1", [("item1", 1), ("item2", 2)])
        .await?;
    let _result: Vec<usize> = client
        .cms_incrby("key1", [("item1", 2), ("item2", 1)])
        .await?;

    let result: Vec<usize> = client.cms_query("key1", ["item1", "item2"]).await?;
    assert_eq!(vec![3, 3], result);

    Ok(())
}
