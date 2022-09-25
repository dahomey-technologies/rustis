use crate::{
    tests::get_test_client, ConnectionCommandResult, FlushingMode, Result, ServerCommands,
    StreamCommands, StreamEntry, XAddOptions, XGroupCreateOptions,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xadd() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    let id1: String = client
        .xadd(
            "mystream",
            "123456-0",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert_eq!("123456-0", &id1);

    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [
                ("field1", "value1"),
                ("field2", "value2"),
                ("field3", "value3"),
            ],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id2.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xgroup() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    client
        .xgroup_create(
            "mystream",
            "mygroup",
            "$",
            XGroupCreateOptions::default().mk_stream(),
        )
        .send()
        .await?;

    let results = client.xinfo_groups("mystream").send().await?;
    assert_eq!(1, results.len());
    assert_eq!("mygroup", results[0].name);
    assert_eq!(0, results[0].consumers);
    assert_eq!(0, results[0].pending);
    assert_eq!("0-0", results[0].last_delivered_id);
    assert_eq!(None, results[0].entries_read);
    assert_eq!(Some(0), results[0].lag);

    let result = client.xgroup_destroy("mystream", "mygroup").send().await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xlen() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id1.is_empty());

    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [
                ("field1", "value1"),
                ("field2", "value2"),
                ("field3", "value3"),
            ],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id2.is_empty());

    let len = client.xlen("mystream").send().await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id1.is_empty());

    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [
                ("field1", "value1"),
                ("field2", "value2"),
                ("field3", "value3"),
            ],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id2.is_empty());

    let results: Vec<StreamEntry<String>> =
        client.xrange("mystream", "-", "+", None).send().await?;
    assert_eq!(2, results.len());
    assert_eq!(id1, results[0].stream_id);
    assert_eq!(Some(&"John".to_owned()), results[0].items.get("name"));
    assert_eq!(Some(&"Doe".to_owned()), results[0].items.get("surname"));
    assert_eq!(id2, results[1].stream_id);
    assert_eq!(Some(&"value1".to_owned()), results[1].items.get("field1"));
    assert_eq!(Some(&"value2".to_owned()), results[1].items.get("field2"));
    assert_eq!(Some(&"value3".to_owned()), results[1].items.get("field3"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xrevrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).send().await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id1.is_empty());

    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [
                ("field1", "value1"),
                ("field2", "value2"),
                ("field3", "value3"),
            ],
            XAddOptions::default(),
        )
        .send()
        .await?;
    assert!(!id2.is_empty());

    let results: Vec<StreamEntry<String>> =
        client.xrevrange("mystream", "+", "-", None).send().await?;
    assert_eq!(2, results.len());
    assert_eq!(id2, results[0].stream_id);
    assert_eq!(Some(&"value1".to_owned()), results[0].items.get("field1"));
    assert_eq!(Some(&"value2".to_owned()), results[0].items.get("field2"));
    assert_eq!(Some(&"value3".to_owned()), results[0].items.get("field3"));
    assert_eq!(id1, results[1].stream_id);
    assert_eq!(Some(&"John".to_owned()), results[1].items.get("name"));
    assert_eq!(Some(&"Doe".to_owned()), results[1].items.get("surname"));

    Ok(())
}
