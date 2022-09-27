use crate::{
    resp::{cmd, Value},
    tests::get_test_client,
    Error, FlushingMode, ListCommands, PrepareCommand, Result, ServerCommands, StringCommands,
    TransactionCommandResult, TransactionCommands, TransactionExt,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction() -> Result<()> {
    let client = get_test_client().await?;

    let transaction = client.create_transaction().await?;

    let value: String = transaction
        .set("key1", "value1")
        .forget()
        .await?
        .set("key2", "value2")
        .forget()
        .await?
        .get("key1")
        .await?
        .exec()
        .await?;

    assert_eq!("value1", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_error() -> Result<()> {
    let client = get_test_client().await?;

    let transaction = client.create_transaction().await?;

    let result = transaction.prepare_command::<Value>(cmd("UNKNOWN")).await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e.starts_with("ERR unknown command 'UNKNOWN'"))
    );

    transaction.discard().await?;

    let transaction = client.create_transaction().await?;

    let result = transaction
        .set("key1", "abc")
        .forget()
        .await?
        .lpop::<_, String, Vec<_>>("key1", 1)
        .await?
        .exec()
        .await;

    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn watch() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", 1).await?;
    client.watch("key").await?;

    let mut value: i32 = client.get("key").await?;
    value += 1;

    let transaction = client.create_transaction().await?;

    transaction.set("key", value).await?.execute().await?;

    let value: i32 = client.get("key").await?;
    assert_eq!(2, value);

    let value = 3;
    client.watch("key").await?;

    let transaction = client.create_transaction().await?;

    // set key on another client during the transaction
    let client2 = get_test_client().await?;
    client2.set("key", value).await?;

    let result = transaction.set("key", value).await?.exec().await;
    assert!(matches!(result, Err(Error::Aborted)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unwatch() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", 1).await?;
    client.watch("key").await?;

    let mut value: i32 = client.get("key").await?;
    value += 1;

    client.watch("key").await?;
    client.unwatch().await?;

    let transaction = client.create_transaction().await?;

    // set key on another client during the transaction
    let client2 = get_test_client().await?;
    client2.set("key", 3).await?;

    transaction.set("key", value).await?.execute().await?;

    let value: i32 = client.get("key").await?;
    assert_eq!(2, value);

    Ok(())
}
