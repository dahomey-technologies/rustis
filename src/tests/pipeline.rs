use crate::{
    Result,
    client::BatchPreparedCommand,
    commands::{FlushingMode, ServerCommands, StringCommands, VAddOptions, VectorSetCommands},
    resp::{Value, cmd},
    tests::{get_cluster_test_client, get_test_client},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn pipeline() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn single_command_forget() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // A single forgotten command must have its response dropped, so the pipeline
    // resolves to the empty tuple rather than surfacing that command's response.
    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.execute::<()>().await?;

    let value: String = client.get("key1").await?;
    assert_eq!("value1", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn empty_pipeline() -> Result<()> {
    let client = get_test_client().await?;

    // An empty pipeline must resolve cleanly instead of failing with an opaque
    // channel-canceled error.
    let pipeline = client.create_pipeline();
    pipeline.execute::<()>().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn error() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.queue_command(cmd("UNKNOWN"));
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let result: Result<(Value, String, String)> = pipeline.execute().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn pipeline_on_cluster() -> Result<()> {
    let client = get_cluster_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn vector_set_commands() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // Vector-set commands must be queueable like every other command family.
    let mut pipeline = client.create_pipeline();
    pipeline
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .forget();
    pipeline.vcard("key").queue();

    let card: u32 = pipeline.execute().await?;
    assert_eq!(1, card);

    Ok(())
}

/// A pipeline retaining a single reply is still a batch. A caller looping over a
/// variable number of commands reads `Vec<T>` whatever that number turns out to
/// be, so one retained command must not turn the batch into the reply.
#[tokio::test]
#[serial]
async fn a_single_retained_reply_reads_as_a_collection() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("ver0", 1).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.get::<()>("ver0").queue();
    let versions: Vec<Option<i64>> = pipeline.execute().await?;
    assert_eq!(vec![Some(1)], versions);

    // The queued count and the retained count differ here: a forgotten command
    // in front must not change what the retained one reads as.
    let mut pipeline = client.create_pipeline();
    pipeline.set("list0", "[]").forget();
    pipeline.get::<()>("ver0").queue();
    let versions: Vec<Option<i64>> = pipeline.execute().await?;
    assert_eq!(vec![Some(1)], versions);

    // An absent key is a `None` inside the collection, not a shorter collection.
    let mut pipeline = client.create_pipeline();
    pipeline.get::<()>("no_such_key").queue();
    let versions: Vec<Option<i64>> = pipeline.execute().await?;
    assert_eq!(vec![None], versions);

    Ok(())
}

/// The other reading of the very same pipelines: asked for the reply's own type,
/// a batch of one is that reply. The collection reading must not have cost it.
#[tokio::test]
#[serial]
async fn a_single_retained_reply_still_reads_as_the_scalar() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("ver0", 1).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.get::<()>("ver0").queue();
    let version: Option<i64> = pipeline.execute().await?;
    assert_eq!(Some(1), version);

    let mut pipeline = client.create_pipeline();
    pipeline.set("list0", "[]").forget();
    pipeline.get::<()>("ver0").queue();
    let version: i64 = pipeline.execute().await?;
    assert_eq!(1, version);

    let mut pipeline = client.create_pipeline();
    pipeline.get::<()>("no_such_key").queue();
    let version: Option<i64> = pipeline.execute().await?;
    assert_eq!(None, version);

    Ok(())
}

/// The command name the one-reply path used to carry has to survive both
/// readings: a pipeline of one awaited command names it when it fails.
#[tokio::test]
#[serial]
async fn a_failing_single_retained_reply_names_its_command() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("a_text_key", "not_a_number").forget();
    pipeline.get::<()>("a_text_key").queue();
    let result: Result<Vec<i64>> = pipeline.execute().await;
    let error = result.expect_err("text read as an integer must be refused");
    assert_eq!(
        Some("GET"),
        error.command(),
        "the awaited command must name itself on the collection reading: {error:?}"
    );

    let mut pipeline = client.create_pipeline();
    pipeline.set("a_text_key", "not_a_number").forget();
    pipeline.get::<()>("a_text_key").queue();
    let result: Result<i64> = pipeline.execute().await;
    let error = result.expect_err("text read as an integer must be refused");
    assert_eq!(
        Some("GET"),
        error.command(),
        "the awaited command must name itself on the scalar reading: {error:?}"
    );

    Ok(())
}

/// The tuple reading of a pipeline holding one retained command: one element per
/// retained command, down to one. This is what `examples/pipelining.rs` shows.
#[tokio::test]
#[serial]
async fn a_single_retained_reply_reads_as_a_one_element_tuple() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("pipelined:3", "three").forget();
    pipeline.get::<()>("pipelined:3").queue();
    let (third,): (String,) = pipeline.execute().await?;
    assert_eq!("three", third);

    Ok(())
}
