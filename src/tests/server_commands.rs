use crate::{
    resp::{BulkString, Value},
    tests::get_default_addr,
    ConnectionMultiplexer, FlushingMode, Result, ServerCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushdb() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database0 = connection.get_database(0);
    let database1 = connection.get_database(1);

    database0.set("key1", "value1").await?;
    database0.set("key2", "value2").await?;

    database1.set("key1", "value1").await?;
    database1.set("key2", "value2").await?;

    database0.flushdb(FlushingMode::Default).await?;

    let value: Value = database0.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = database0.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: String = database1.get("key1").await?;
    assert_eq!("value1", value);

    let value: String = database1.get("key2").await?;
    assert_eq!("value2", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushall() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database0 = connection.get_database(0);
    let database1 = connection.get_database(1);

    database0.set("key1", "value1").await?;
    database0.set("key2", "value2").await?;

    database1.set("key1", "value1").await?;
    database1.set("key2", "value2").await?;

    database0.flushall(FlushingMode::Default).await?;

    let value: Value = database0.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = database0.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = database1.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = database1.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}
