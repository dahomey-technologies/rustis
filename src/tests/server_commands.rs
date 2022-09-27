use crate::{
    resp::{BulkString, Value},
    tests::get_test_client,
    ConnectionCommands, FlushingMode, Result, ServerCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushdb() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    client0.set("key1", "value1").await?;
    client0.set("key2", "value2").await?;

    client1.set("key1", "value1").await?;
    client1.set("key2", "value2").await?;

    client0.flushdb(FlushingMode::Default).await?;

    let value: Value = client0.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: String = client1.get("key1").await?;
    assert_eq!("value1", value);

    let value: String = client1.get("key2").await?;
    assert_eq!("value2", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushall() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    client0.set("key1", "value1").await?;
    client0.set("key2", "value2").await?;

    client1.set("key1", "value1").await?;
    client1.set("key2", "value2").await?;

    client0.flushall(FlushingMode::Default).await?;

    let value: Value = client0.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client1.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client1.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn time() -> Result<()> {
    let client = get_test_client().await?;

    let (_unix_timestamp, _microseconds) = client.time().await?;

    Ok(())
}
