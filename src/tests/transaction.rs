use crate::{tests::get_default_addr, ConnectionMultiplexer, Result, StringCommands, ListCommands};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let transaction = database.create_transaction();

    let fut1 = transaction.set("key1", "value1");
    let fut2 = transaction.set("key2", "value2");
    let fut3 = transaction.get("key1");

    transaction.execute().await?;

    let (result1, result2, result3) = tokio::join!(fut1, fut2, fut3);

    result1?;
    result2?;
    let value: String = result3?;
    assert_eq!("value1", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn transaction_error() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let transaction = database.create_transaction();

    let fut1 = transaction.set("key1", "abc");
    let fut2 = transaction.lpop::<_, String, Vec<_>>("key1", 1);

    transaction.execute().await?;

    let (result1, result2) = tokio::join!(fut1, fut2);

    assert!(result1.is_ok());
    assert!(result2.is_err());

    Ok(())
}
