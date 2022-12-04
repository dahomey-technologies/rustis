use crate::{
    client::BatchPreparedCommand,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{cmd, Value},
    tests::get_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pipeline() -> Result<()> {
    let mut client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<_, String>("key1").queue();
    pipeline.get::<_, String>("key2").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn error() -> Result<()> {
    let mut client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.queue(cmd("UNKNOWN"));
    pipeline.get::<_, String>("key1").queue();
    pipeline.get::<_, String>("key2").queue();

    let (result, value1, value2): (Value, String, String) = pipeline.execute().await?;
    assert!(matches!(result, Value::Error(_)));
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}
