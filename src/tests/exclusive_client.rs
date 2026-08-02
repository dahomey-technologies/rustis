use crate::{
    ClientError, ErrorKind, Result,
    client::{BatchPreparedCommand, Client, ExclusiveClient},
    commands::{
        BlockingCommands, ConnectionCommands, FlushingMode, PubSubCommands, ServerCommands,
        StringCommands, TransactionCommands,
    },
    tests::{get_default_config, get_exclusive_test_client, get_test_client},
};
use futures_util::StreamExt;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn into_exclusive_requires_a_sole_handle() -> Result<()> {
    let client = get_test_client().await?;
    let clone = client.clone();

    // `unwrap_err` would need `ExclusiveClient: Debug`, which the type does not
    // implement any more than `Client` does.
    let Err(error) = client.into_exclusive() else {
        panic!("into_exclusive succeeded while a clone was alive");
    };
    assert!(
        matches!(error.kind(), ErrorKind::Client(ClientError::NotExclusive)),
        "expected a NotExclusive error, got {error:?}"
    );

    // The surviving handle is untouched: the failed conversion gave up its own
    // reference and nothing else.
    clone.ping::<()>(()).await?;
    clone.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn into_exclusive_on_a_sole_handle() -> Result<()> {
    let client = get_test_client().await?;
    let client = client.into_exclusive()?;

    client.flushdb(FlushingMode::Sync).await?;

    // Both families the multiplexed client no longer carries.
    let result: Option<(String, String)> = client.blpop("key", 0.01).await?;
    assert_eq!(None, result);

    client.watch("key").await?;
    client.unwatch().await?;

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn exclusive_client_runs_ordinary_commands() -> Result<()> {
    let client = get_exclusive_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    client.send::<()>(crate::resp::cmd("PING"), None).await?;
    client.send_and_forget(crate::resp::cmd("PING"), None)?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.get::<()>("key1").queue();
    let value: String = pipeline.execute().await?;
    assert_eq!("value1", value);

    let mut transaction = client.create_transaction();
    transaction.set("key2", "value2").forget();
    transaction.get::<()>("key2").queue();
    let value: String = transaction.execute().await?;
    assert_eq!("value2", value);

    let mut pub_sub_stream = client.subscribe("channel").await?;
    let publisher = get_test_client().await?;
    publisher.publish("channel", "payload").await?;
    let message = pub_sub_stream.next().await.unwrap()?;
    assert_eq!(b"payload", message.payload());
    pub_sub_stream.close().await?;
    publisher.close().await?;

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn into_multiplexed_round_trip() -> Result<()> {
    let client = ExclusiveClient::connect(get_default_config()?).await?;

    let client: Client = client.into_multiplexed();
    let clone = client.clone();

    clone.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    clone.close().await?;
    client.close().await?;

    Ok(())
}
