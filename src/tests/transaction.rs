use crate::{
    tests::get_default_addr, ConnectionMultiplexer, ListCommands, Result, StringCommands,
    TransactionCommandResult, TransactionExt, IntoCommandResult, cmd, resp::Value, Error,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let transaction = database.create_transaction().await?;

    let value: String = transaction
        .set("key1", "value1")
        .queue_and_forget()
        .await?
        .set("key2", "value2")
        .queue_and_forget()
        .await?
        .get("key1")
        .queue()
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let transaction = database.create_transaction().await?;

    let result = transaction
        .into_command_result::<Value>(cmd("UNKNOWN")).queue().await;
    assert!(
        matches!(result, Err(Error::Redis(e)) if e.starts_with("ERR unknown command 'UNKNOWN'"))
    );

    transaction.discard().await?;

    let transaction = database.create_transaction().await?;

    let result = transaction
        .set("key1", "abc")
        .queue_and_forget()
        .await?
        .lpop::<_, String, Vec<_>>("key1", 1)
        .queue()
        .await?
        .exec()
        .await;

    assert!(result.is_err());

    Ok(())
}
