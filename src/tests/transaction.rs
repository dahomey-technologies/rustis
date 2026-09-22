use crate::{
    ErrorKind, RedisError, RedisErrorKind, Result,
    client::BatchPreparedCommand,
    commands::{
        FlushingMode, ListCommands, ServerCommands, StringCommands, TransactionCommands,
        VAddOptions, VectorSetCommands,
    },
    resp::cmd,
    tests::{get_cluster_test_client, get_exclusive_test_client, get_test_client},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn transaction_exec() -> Result<()> {
    let client = get_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "value1").forget();
    transaction.set("key2", "value2").forget();
    transaction.get::<()>("key1").queue();
    transaction.get::<()>("key2").queue();
    let (value1, value2): (String, String) = transaction.execute().await?;

    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    let mut transaction = client.create_transaction();

    transaction.set("key", "value").forget();
    transaction.get::<()>("key").queue();
    let value: String = transaction.execute().await?;

    assert_eq!("value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn transaction_error() -> Result<()> {
    let client = get_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "abc").forget();
    transaction.queue_command(cmd("UNKNOWN"));
    let result: Result<String> = transaction.execute().await;

    let error = result.unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let mut transaction = client.create_transaction();

    transaction.set("key1", "abc").forget();
    transaction.lpop::<()>("key1", 1).queue();
    let result: Result<String> = transaction.execute().await;

    let error = result.unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::WrongType,
            description: _
        })
    ));

    Ok(())
}

#[tokio::test]
#[serial]
async fn watch() -> Result<()> {
    let client = get_exclusive_test_client().await?;
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
    let error = result.unwrap_err();
    assert!(matches!(error.kind(), ErrorKind::Aborted));

    Ok(())
}

#[tokio::test]
#[serial]
async fn unwatch() -> Result<()> {
    let client = get_exclusive_test_client().await?;
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

#[tokio::test]
#[serial]
async fn transaction_discard() -> Result<()> {
    let client = get_test_client().await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "value1").forget();
    transaction.set("key2", "value2").forget();
    transaction.get::<()>("key1").queue();

    std::mem::drop(transaction);

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn transaction_on_cluster_connection_with_keys_with_same_slot() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut transaction = client.create_transaction();

    transaction
        .mset([("{hash}key1", "value1"), ("{hash}key2", "value2")])
        .queue();
    transaction.get::<String>("{hash}key1").queue();
    transaction.get::<String>("{hash}key2").queue();
    let ((), val1, val2): ((), String, String) = transaction.execute().await.unwrap();
    assert_eq!("value1", val1);
    assert_eq!("value2", val2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn transaction_on_cluster_connection_with_keys_with_different_slots() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut transaction = client.create_transaction();

    transaction
        .mset([("key1", "value1"), ("key2", "value2")])
        .queue();
    transaction.get::<String>("key1").queue();
    transaction.get::<String>("key2").queue();
    let result: Result<((), String, String)> = transaction.execute().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn vector_set_commands() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // Vector-set commands must be queueable like every other command family.
    let mut transaction = client.create_transaction();
    transaction
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .forget();
    transaction.vcard("key").queue();

    let card: u32 = transaction.execute().await?;
    assert_eq!(1, card);

    Ok(())
}

/// A transaction retaining a single reply reads as a pipeline retaining one:
/// `EXEC`'s reply is a batch, and a batch of one is a sequence of one.
#[tokio::test]
#[serial]
async fn a_single_retained_reply_reads_as_a_collection() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("ver0", 1).await?;

    let mut transaction = client.create_transaction();
    transaction.get::<()>("ver0").queue();
    let versions: Vec<Option<i64>> = transaction.execute().await?;
    assert_eq!(vec![Some(1)], versions);

    // The queued count and the retained count differ here: a forgotten command
    // in front must not change what the retained one reads as.
    let mut transaction = client.create_transaction();
    transaction.set("list0", "[]").forget();
    transaction.get::<()>("ver0").queue();
    let versions: Vec<Option<i64>> = transaction.execute().await?;
    assert_eq!(vec![Some(1)], versions);

    // An absent key is a `None` inside the collection, not a shorter collection.
    let mut transaction = client.create_transaction();
    transaction.get::<()>("no_such_key").queue();
    let versions: Vec<Option<i64>> = transaction.execute().await?;
    assert_eq!(vec![None], versions);

    Ok(())
}

/// The other reading of the very same transactions: asked for the reply's own
/// type, a batch of one is that reply.
#[tokio::test]
#[serial]
async fn a_single_retained_reply_still_reads_as_the_scalar_in_a_transaction() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("ver0", 1).await?;

    let mut transaction = client.create_transaction();
    transaction.get::<()>("ver0").queue();
    let version: Option<i64> = transaction.execute().await?;
    assert_eq!(Some(1), version);

    let mut transaction = client.create_transaction();
    transaction.set("list0", "[]").forget();
    transaction.get::<()>("ver0").queue();
    let version: i64 = transaction.execute().await?;
    assert_eq!(1, version);

    let mut transaction = client.create_transaction();
    transaction.get::<()>("no_such_key").queue();
    let version: Option<i64> = transaction.execute().await?;
    assert_eq!(None, version);

    Ok(())
}

/// A transaction of one awaited command names it when it fails, whichever
/// reading the caller asked for.
#[tokio::test]
#[serial]
async fn a_failing_single_retained_reply_names_its_command_in_a_transaction() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut transaction = client.create_transaction();
    transaction.set("a_text_key", "not_a_number").forget();
    transaction.get::<()>("a_text_key").queue();
    let result: Result<Vec<i64>> = transaction.execute().await;
    let error = result.expect_err("text read as an integer must be refused");
    assert_eq!(
        Some("GET"),
        error.command(),
        "the awaited command must name itself on the collection reading: {error:?}"
    );

    let mut transaction = client.create_transaction();
    transaction.set("a_text_key", "not_a_number").forget();
    transaction.get::<()>("a_text_key").queue();
    let result: Result<i64> = transaction.execute().await;
    let error = result.expect_err("text read as an integer must be refused");
    assert_eq!(
        Some("GET"),
        error.command(),
        "the awaited command must name itself on the scalar reading: {error:?}"
    );

    Ok(())
}

/// The tuple reading of a transaction holding one retained command, which must
/// agree with the pipeline's.
#[tokio::test]
#[serial]
async fn a_single_retained_reply_reads_as_a_one_element_tuple_in_a_transaction() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut transaction = client.create_transaction();
    transaction.set("queued:3", "three").forget();
    transaction.get::<()>("queued:3").queue();
    let (third,): (String,) = transaction.execute().await?;
    assert_eq!("three", third);

    Ok(())
}
