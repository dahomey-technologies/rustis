use crate::{
    Result,
    commands::{
        FlushingMode, ServerCommands, StreamCommands, StreamEntry, XAddOptions, XAutoClaimOptions,
        XAutoClaimResult, XGroupCreateOptions, XInfoStreamOptions, XPendingOptions,
        XReadGroupOptions, XReadOptions, XTrimOperator, XTrimOptions,
    },
    tests::get_test_client,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xadd() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "123456-0",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
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
        .await?;
    assert!(!id2.is_empty());

    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!(2, result.length);
    assert_eq!(id2, result.last_generated_id);
    assert_eq!(0, result.groups);
    assert_eq!("0-0", result.max_deleted_entry_id);
    assert_eq!(2, result.entries_added);
    assert_eq!(id1, result.recorded_first_entry_id);
    assert_eq!(id1, result.first_entry.stream_id);
    assert_eq!(id2, result.last_entry.stream_id);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xdel() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .await?;

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
        .await?;

    let deleted = client.xdel("mystream", id1).await?;
    assert_eq!(1, deleted);

    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
    assert_eq!(1, results.len());
    assert_eq!(id2, results[0].stream_id);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xgroup() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client
        .xgroup_create(
            "mystream",
            "mygroup",
            "$",
            XGroupCreateOptions::default().mk_stream(),
        )
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Bob")
        .await?;
    assert!(result);

    let results = client.xinfo_groups("mystream").await?;
    assert_eq!(1, results.len());
    assert_eq!("mygroup", results[0].name);
    assert_eq!(1, results[0].consumers);
    assert_eq!(0, results[0].pending);
    assert_eq!("0-0", results[0].last_delivered_id);
    assert_eq!(None, results[0].entries_read);
    assert_eq!(Some(0), results[0].lag);

    let results = client.xinfo_consumers("mystream", "mygroup").await?;
    assert_eq!(1, results.len());
    assert_eq!("Bob", results[0].name);
    assert!(results[0].idle_millis < 100);
    assert_eq!(0, results[0].pending);

    let result = client.xgroup_destroy("mystream", "mygroup").await?;
    assert!(result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xlen() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
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
        .await?;
    assert!(!id2.is_empty());

    let len = client.xlen("mystream").await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
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
        .await?;
    assert!(!id2.is_empty());

    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
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
async fn xread() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "123456-0",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .await?;

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
        .await?;

    let results: Vec<(String, Vec<StreamEntry<String>>)> =
        client.xread(Default::default(), "mystream", 0).await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(2, results[0].1.len());
    assert_eq!(id1, results[0].1[0].stream_id);
    assert_eq!(2, results[0].1[0].items.len());
    assert_eq!(Some(&"John".to_string()), results[0].1[0].items.get("name"));
    assert_eq!(
        Some(&"Doe".to_string()),
        results[0].1[0].items.get("surname")
    );
    assert_eq!(id2, results[0].1[1].stream_id);
    assert_eq!(3, results[0].1[1].items.len());
    assert_eq!(
        Some(&"value1".to_string()),
        results[0].1[1].items.get("field1")
    );
    assert_eq!(
        Some(&"value2".to_string()),
        results[0].1[1].items.get("field2")
    );
    assert_eq!(
        Some(&"value3".to_string()),
        results[0].1[1].items.get("field3")
    );

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xread(XReadOptions::default().block(1000).count(1), "mystream", 0)
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(1, results[0].1.len());
    assert_eq!(id1, results[0].1[0].stream_id);
    assert_eq!(2, results[0].1[0].items.len());
    assert_eq!(Some(&"John".to_string()), results[0].1[0].items.get("name"));
    assert_eq!(
        Some(&"Doe".to_string()),
        results[0].1[0].items.get("surname")
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xreadgroup() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client
        .xgroup_create(
            "mystream",
            "mygroup",
            "$",
            XGroupCreateOptions::default().mk_stream(),
        )
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Bob")
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Alice")
        .await?;
    assert!(result);

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "apple"),
            XAddOptions::default(),
        )
        .await?;

    let id2: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "orange"),
            XAddOptions::default(),
        )
        .await?;

    let id3: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "strawberry"),
            XAddOptions::default(),
        )
        .await?;

    let id4: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "apricot"),
            XAddOptions::default(),
        )
        .await?;

    let id5: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "banana"),
            XAddOptions::default(),
        )
        .await?;

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(3),
            "mystream",
            ">",
        )
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(3, results[0].1.len());
    assert_eq!(id1, results[0].1[0].stream_id);
    assert_eq!(1, results[0].1[0].items.len());
    assert_eq!(id2, results[0].1[1].stream_id);
    assert_eq!(1, results[0].1[1].items.len());
    assert_eq!(id3, results[0].1[2].stream_id);
    assert_eq!(1, results[0].1[2].items.len());

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Alice",
            XReadGroupOptions::default().count(3),
            "mystream",
            ">",
        )
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(2, results[0].1.len());
    assert_eq!(id4, results[0].1[0].stream_id);
    assert_eq!(1, results[0].1[0].items.len());
    assert_eq!(id5, results[0].1[1].stream_id);
    assert_eq!(1, results[0].1[1].items.len());

    let result = client.xpending("mystream", "mygroup").await?;
    assert_eq!(5, result.num_pending_messages);
    assert_eq!(id1, result.smallest_id);
    assert_eq!(id5, result.greatest_id);
    assert_eq!(2, result.consumers.len());
    assert_eq!("Alice", result.consumers[0].consumer);
    assert_eq!(2, result.consumers[0].num_messages);
    assert_eq!("Bob", result.consumers[1].consumer);
    assert_eq!(3, result.consumers[1].num_messages);

    let results = client
        .xpending_with_options(
            "mystream",
            "mygroup",
            XPendingOptions::default().start("-").end("+").count(10),
        )
        .await?;
    assert_eq!(5, results.len());
    assert_eq!(id1, results[0].message_id);
    assert_eq!("Bob", results[0].consumer);
    assert!(results[0].elapsed_millis < 100);
    assert_eq!(1, results[0].times_delivered);
    assert_eq!(id2, results[1].message_id);
    assert_eq!("Bob", results[1].consumer);
    assert!(results[1].elapsed_millis < 100);
    assert_eq!(1, results[1].times_delivered);
    assert_eq!(id3, results[2].message_id);
    assert_eq!("Bob", results[2].consumer);
    assert!(results[2].elapsed_millis < 100);
    assert_eq!(1, results[2].times_delivered);
    assert_eq!(id4, results[3].message_id);
    assert_eq!("Alice", results[3].consumer);
    assert!(results[3].elapsed_millis < 100);
    assert_eq!(1, results[3].times_delivered);
    assert_eq!(id5, results[4].message_id);
    assert_eq!("Alice", results[4].consumer);
    assert!(results[4].elapsed_millis < 100);
    assert_eq!(1, results[4].times_delivered);

    let num = client
        .xack("mystream", "mygroup", [id1, id2, id3, id4, id5])
        .await?;
    assert_eq!(5, num);

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(3),
            "mystream",
            0,
        )
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(0, results[0].1.len());

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Alice",
            XReadGroupOptions::default().count(3),
            "mystream",
            0,
        )
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(0, results[0].1.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xclaim() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client
        .xgroup_create(
            "mystream",
            "mygroup",
            "$",
            XGroupCreateOptions::default().mk_stream(),
        )
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Bob")
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Alice")
        .await?;
    assert!(result);

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "apple"),
            XAddOptions::default(),
        )
        .await?;

    let id2: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "orange"),
            XAddOptions::default(),
        )
        .await?;

    let id3: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "strawberry"),
            XAddOptions::default(),
        )
        .await?;

    let id4: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "apricot"),
            XAddOptions::default(),
        )
        .await?;

    let id5: String = client
        .xadd(
            "mystream",
            "*",
            ("message", "banana"),
            XAddOptions::default(),
        )
        .await?;

    let _results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(3),
            "mystream",
            ">",
        )
        .await?;

    let _results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Alice",
            XReadGroupOptions::default().count(3),
            "mystream",
            ">",
        )
        .await?;

    let num = client.xack("mystream", "mygroup", [id1, id2, id3]).await?;
    assert_eq!(3, num);

    let results: Vec<StreamEntry<String>> = client
        .xclaim(
            "mystream",
            "mygroup",
            "Bob",
            0,
            [id4.clone(), id5.clone()],
            Default::default(),
        )
        .await?;
    assert_eq!(2, results.len());
    assert_eq!(id4, results[0].stream_id);
    assert_eq!(id5, results[1].stream_id);

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(2),
            "mystream",
            0,
        )
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(2, results[0].1.len());
    assert_eq!(id4, results[0].1[0].stream_id);
    assert_eq!(1, results[0].1[0].items.len());
    assert_eq!(id5, results[0].1[1].stream_id);
    assert_eq!(1, results[0].1[1].items.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xautoclaim() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client
        .xgroup_create(
            "mystream",
            "mygroup",
            "$",
            XGroupCreateOptions::default().mk_stream(),
        )
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Bob")
        .await?;
    assert!(result);

    let result = client
        .xgroup_createconsumer("mystream", "mygroup", "Alice")
        .await?;
    assert!(result);

    let id1: String = client
        .xadd(
            "mystream",
            "1-0",
            ("message", "apple"),
            XAddOptions::default(),
        )
        .await?;

    let id2: String = client
        .xadd(
            "mystream",
            "2-0",
            ("message", "orange"),
            XAddOptions::default(),
        )
        .await?;

    let id3: String = client
        .xadd(
            "mystream",
            "3-0",
            ("message", "strawberry"),
            XAddOptions::default(),
        )
        .await?;

    let id4: String = client
        .xadd(
            "mystream",
            "4-0",
            ("message", "apricot"),
            XAddOptions::default(),
        )
        .await?;

    let id5: String = client
        .xadd(
            "mystream",
            "5-0",
            ("message", "banana"),
            XAddOptions::default(),
        )
        .await?;

    let _results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(3),
            "mystream",
            ">",
        )
        .await?;

    let _results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Alice",
            XReadGroupOptions::default().count(3),
            "mystream",
            ">",
        )
        .await?;

    let num = client.xack("mystream", "mygroup", [id1, id2, id3]).await?;
    assert_eq!(3, num);

    let result: XAutoClaimResult<String> = client
        .xautoclaim(
            "mystream",
            "mygroup",
            "Bob",
            0,
            "0-0",
            XAutoClaimOptions::default().count(1),
        )
        .await?;
    assert_eq!(id5, result.start_stream_id);
    assert_eq!(1, result.entries.len());
    assert_eq!(id4, result.entries[0].stream_id);

    let result: XAutoClaimResult<String> = client
        .xautoclaim(
            "mystream",
            "mygroup",
            "Bob",
            0,
            id5.clone(),
            XAutoClaimOptions::default().count(1),
        )
        .await?;
    assert_eq!("0-0", result.start_stream_id);
    assert_eq!(1, result.entries.len());
    assert_eq!(id5, result.entries[0].stream_id);

    let results: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(2),
            "mystream",
            0,
        )
        .await?;
    assert_eq!(1, results.len());
    assert_eq!("mystream", results[0].0);
    assert_eq!(2, results[0].1.len());
    assert_eq!(id4, results[0].1[0].stream_id);
    assert_eq!(1, results[0].1[0].items.len());
    assert_eq!(id5, results[0].1[1].stream_id);
    assert_eq!(1, results[0].1[1].items.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xrevrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
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
        .await?;
    assert!(!id2.is_empty());

    let results: Vec<StreamEntry<String>> = client.xrevrange("mystream", "+", "-", None).await?;
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xtrim() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let _id1: String = client
        .xadd(
            "mystream",
            "*",
            [("name", "John"), ("surname", "Doe")],
            XAddOptions::default(),
        )
        .await?;

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
        .await?;

    let deleted = client
        .xtrim("mystream", XTrimOptions::max_len(XTrimOperator::None, 1))
        .await?;
    assert_eq!(1, deleted);

    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
    assert_eq!(1, results.len());
    assert_eq!(id2, results[0].stream_id);

    Ok(())
}
