use crate::{
    Result,
    client::BatchPreparedCommand,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{Value, cmd},
    tests::{get_cluster_test_client, get_test_client},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn pipeline() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn single_command_forget() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // A single forgotten command must have its response dropped, so the pipeline
    // resolves to the empty tuple rather than surfacing that command's response.
    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.execute::<()>().await?;

    let value: String = client.get("key1").await?;
    assert_eq!("value1", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn empty_pipeline() -> Result<()> {
    let client = get_test_client().await?;

    // An empty pipeline must resolve cleanly instead of failing with an opaque
    // channel-canceled error.
    let pipeline = client.create_pipeline();
    pipeline.execute::<()>().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn error() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.queue(cmd("UNKNOWN"));
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let result: Result<(Value, String, String)> = pipeline.execute().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn pipeline_on_cluster() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}
