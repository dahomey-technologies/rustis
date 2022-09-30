use std::collections::HashMap;

use crate::{
    resp::{BulkString, Value},
    tests::get_test_client,
    ConnectionCommands, FlushingMode, Result, ServerCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn config_get() -> Result<()> {
    let client  = get_test_client().await?;

    let configs: HashMap<String, String> = client.config_get(["hash-max-listpack-entries", "zset-max-listpack-entries"]).await?;
    assert_eq!(2, configs.len());
    assert_eq!(Some(&"512".to_owned()), configs.get("hash-max-listpack-entries"));
    assert_eq!(Some(&"128".to_owned()), configs.get("zset-max-listpack-entries"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn config_set() -> Result<()> {
    let client  = get_test_client().await?;

    client.config_set([("hash-max-listpack-entries", 513), ("zset-max-listpack-entries", 129)]).await?;

    let configs: HashMap<String, String> = client.config_get(["hash-max-listpack-entries", "zset-max-listpack-entries"]).await?;
    assert_eq!(2, configs.len());
    assert_eq!(Some(&"513".to_owned()), configs.get("hash-max-listpack-entries"));
    assert_eq!(Some(&"129".to_owned()), configs.get("zset-max-listpack-entries"));

    client.config_set([("hash-max-listpack-entries", 512), ("zset-max-listpack-entries", 128)]).await?;

    Ok(())
}

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
