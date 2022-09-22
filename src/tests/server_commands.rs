use crate::{
    resp::{BulkString, Value},
    tests::get_default_addr,
    Connection, ConnectionCommandResult, FlushingMode, Result, ServerCommands, StringCommands, ConnectionCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushdb() -> Result<()> {
    let connection0 = Connection::connect(get_default_addr()).await?;
    let connection1 = Connection::connect(get_default_addr()).await?;
    connection1.select(1).send().await?;

    connection0.set("key1", "value1").send().await?;
    connection0.set("key2", "value2").send().await?;

    connection1.set("key1", "value1").send().await?;
    connection1.set("key2", "value2").send().await?;

    connection0.flushdb(FlushingMode::Default).send().await?;

    let value: Value = connection0.get("key1").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = connection0.get("key2").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: String = connection1.get("key1").send().await?;
    assert_eq!("value1", value);

    let value: String = connection1.get("key2").send().await?;
    assert_eq!("value2", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushall() -> Result<()> {
    let connection0 = Connection::connect(get_default_addr()).await?;
    let connection1 = Connection::connect(get_default_addr()).await?;
    connection1.select(1).send().await?;

    connection0.set("key1", "value1").send().await?;
    connection0.set("key2", "value2").send().await?;

    connection1.set("key1", "value1").send().await?;
    connection1.set("key2", "value2").send().await?;

    connection0.flushall(FlushingMode::Default).send().await?;

    let value: Value = connection0.get("key1").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = connection0.get("key2").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = connection1.get("key1").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = connection1.get("key2").send().await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn time() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    let (_unix_timestamp, _microseconds) = connection.time().send().await?;

    Ok(())
}
