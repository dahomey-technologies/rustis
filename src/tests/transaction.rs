use crate::{
    Error, RedisError, RedisErrorKind, Result,
    client::BatchPreparedCommand,
    commands::{FlushingMode, ListCommands, ServerCommands, StringCommands, TransactionCommands},
    resp::cmd,
    tests::{get_cluster_test_client, get_test_client},
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_exec() -> Result<()> {
    let client = get_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "value1").forget();
    transaction.set("key2", "value2").forget();
    transaction.get::<_, ()>("key1").queue();
    transaction.get::<_, ()>("key2").queue();
    let (value1, value2): (String, String) = transaction.execute().await?;

    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    let mut transaction = client.create_transaction();

    transaction.set("key", "value").forget();
    transaction.get::<_, ()>("key").queue();
    let value: String = transaction.execute().await?;

    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_error() -> Result<()> {
    let client = get_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "abc").forget();
    transaction.queue(cmd("UNKNOWN"));
    let result: Result<String> = transaction.execute().await;

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let mut transaction = client.create_transaction();

    transaction.set("key1", "abc").forget();
    transaction.lpop::<_, (), ()>("key1", 1).queue();
    let result: Result<String> = transaction.execute().await;

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::WrongType,
            description: _
        }))
    ));

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

    let mut transaction = client.create_transaction();

    transaction.set("key", value).queue();
    transaction.execute::<()>().await?;

    let value: i32 = client.get("key").await?;
    assert_eq!(2, value);

    let value = 3;
    client.watch("key").await?;

    let mut transaction = client.create_transaction();

    // set key on another client during the transaction
    let client2 = get_test_client().await?;
    client2.set("key", value).await?;

    transaction.set("key", value).queue();
    let result: Result<()> = transaction.execute().await;
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

    let mut transaction = client.create_transaction();

    // set key on another client during the transaction
    let client2 = get_test_client().await?;
    client2.set("key", 3).await?;

    transaction.set("key", value).queue();
    transaction.execute::<()>().await?;

    let value: i32 = client.get("key").await?;
    assert_eq!(2, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_discard() -> Result<()> {
    let client = get_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "value1").forget();
    transaction.set("key2", "value2").forget();
    transaction.get::<_, ()>("key1").queue();

    std::mem::drop(transaction);

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_on_cluster_connection_with_keys_with_same_slot() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut transaction = client.create_transaction();

    transaction
        .mset([("{hash}key1", "value1"), ("{hash}key2", "value2")])
        .queue();
    transaction.get::<_, String>("{hash}key1").queue();
    transaction.get::<_, String>("{hash}key2").queue();
    let ((), val1, val2): ((), String, String) = transaction.execute().await.unwrap();
    assert_eq!("value1", val1);
    assert_eq!("value2", val2);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_on_cluster_connection_with_keys_with_different_slots() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut transaction = client.create_transaction();

    transaction
        .mset([("key1", "value1"), ("key2", "value2")])
        .queue();
    transaction.get::<_, String>("key1").queue();
    transaction.get::<_, String>("key2").queue();
    let result: Result<((), String, String)> = transaction.execute().await;
    assert!(result.is_err());

    Ok(())
}
