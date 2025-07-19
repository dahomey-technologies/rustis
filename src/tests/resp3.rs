use crate::{
    Result,
    commands::{
        ConnectionCommands, FlushingMode, HelloOptions, ServerCommands, SortedSetCommands,
        StringCommands,
    },
    tests::get_test_client,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn double() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.hello(HelloOptions::new(3)).await?;

    client
        .zadd(
            "key",
            [(1.1, "one"), (2.2, "two"), (3.3, "three")],
            Default::default(),
        )
        .await?;

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, Default::default())
        .await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 1.1), values[0]);
    assert_eq!(("two".to_owned(), 2.2), values[1]);
    assert_eq!(("three".to_owned(), 3.3), values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn null() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.hello(HelloOptions::new(3)).await?;

    let value: Option<String> = client.get("key").await?;
    assert_eq!(None, value);

    Ok(())
}
