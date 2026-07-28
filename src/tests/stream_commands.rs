use crate::{
    Result,
    commands::{
        ConsumerGroupOptions, FlushingMode, ServerCommands, StreamCommands, StreamEntry,
        StreamEntryDeletionPolicy, XAddOptions, XAutoClaimOptions, XAutoClaimResult,
        XCfgSetOptions, XClaimOptions, XGroupCreateOptions, XInfoStreamOptions, XNackMode,
        XNackOptions, XPendingMessageResult, XPendingOptions, XReadGroupOptions, XReadOptions,
        XSetIdOptions, XTrimOptions,
    },
    resp::Value,
    tests::{TestClient, get_test_client},
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
async fn xdelex() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;
    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;

    client
        .xgroup_create("mystream", "mygroup", "0", XGroupCreateOptions::default())
        .await?;
    // Read `id1` into the group's PEL, leaving `id2` untouched.
    let _: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "myconsumer",
            XReadGroupOptions::default().count(1),
            "mystream",
            ">",
        )
        .await?;

    // `ACKED` refuses to delete an entry still pending in a group: 2.
    let results = client
        .xdelex("mystream", StreamEntryDeletionPolicy::Acked, &id1)
        .await?;
    assert_eq!(vec![2], results);

    // `KEEPREF` deletes unconditionally: 1. An unknown id yields -1.
    let results = client
        .xdelex(
            "mystream",
            StreamEntryDeletionPolicy::KeepRef,
            [id2.as_str(), "999999999999-0"],
        )
        .await?;
    assert_eq!(vec![1, -1], results);

    // Omitting the policy defaults to `KEEPREF` server-side.
    let results = client.xdelex("mystream", None, &id1).await?;
    assert_eq!(vec![1], results);

    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
    assert!(results.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xackdel() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;
    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;

    client
        .xgroup_create("mystream", "mygroup", "0", XGroupCreateOptions::default())
        .await?;
    let _: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "myconsumer",
            XReadGroupOptions::default(),
            "mystream",
            ">",
        )
        .await?;

    // Acknowledge and delete in one round trip: 1 for `id1`, -1 for an unknown id.
    let results = client
        .xackdel(
            "mystream",
            "mygroup",
            StreamEntryDeletionPolicy::DelRef,
            [id1.as_str(), "999999999999-0"],
        )
        .await?;
    assert_eq!(vec![1, -1], results);

    let results = client.xackdel("mystream", "mygroup", None, &id2).await?;
    assert_eq!(vec![1], results);

    // Both entries are gone and the PEL is empty.
    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
    assert!(results.is_empty());

    let pending = client.xpending("mystream", "mygroup").await?;
    assert_eq!(0, pending.num_pending_messages);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xnack() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value1")],
            XAddOptions::default(),
        )
        .await?;
    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value2")],
            XAddOptions::default(),
        )
        .await?;

    client
        .xgroup_create("mystream", "mygroup", "0", XGroupCreateOptions::default())
        .await?;
    let _: Vec<(String, Vec<StreamEntry<String>>)> = client
        .xreadgroup(
            "mygroup",
            "consumer1",
            XReadGroupOptions::default(),
            "mystream",
            ">",
        )
        .await?;

    let released = client
        .xnack(
            "mystream",
            "mygroup",
            XNackMode::Fail,
            [id1.as_str(), id2.as_str()],
            XNackOptions::default(),
        )
        .await?;
    assert_eq!(2, released);

    // The entries stay pending but lose their owner, so another consumer can
    // take them without waiting for the idle timeout.
    let pending = client.xpending("mystream", "mygroup").await?;
    assert_eq!(2, pending.num_pending_messages);
    let pending: Vec<XPendingMessageResult> = client
        .xpending_with_options(
            "mystream",
            "mygroup",
            XPendingOptions::default().start("-").end("+").count(10),
        )
        .await?;
    assert!(pending.iter().all(|message| message.consumer.is_empty()));

    // An id that is not in the PEL is ignored rather than counted.
    let released = client
        .xnack(
            "mystream",
            "mygroup",
            XNackMode::Silent,
            "999999999999-0",
            XNackOptions::default(),
        )
        .await?;
    assert_eq!(0, released);

    Ok(())
}

#[test]
fn xnack_args() {
    let cmd = TestClient
        .xnack(
            "mystream",
            "mygroup",
            XNackMode::Fatal,
            ["1-1", "2-2"],
            XNackOptions::default().retry_count(3).force(),
        )
        .command;
    assert_eq!(
        "XNACK mystream mygroup FATAL IDS 2 1-1 2-2 RETRYCOUNT 3 FORCE",
        cmd.to_string()
    );

    let cmd = TestClient
        .xnack(
            "mystream",
            "mygroup",
            XNackMode::Silent,
            "1-1",
            XNackOptions::default(),
        )
        .command;
    assert_eq!("XNACK mystream mygroup SILENT IDS 1 1-1", cmd.to_string());
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xadd_idmp() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // Manual mode: the same (pid, iid) pair is added once and echoed back on
    // every resend.
    let id1: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default().idmp("producer-1", "iid-1"),
        )
        .await?;
    let id1_again: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "other value")],
            XAddOptions::default().idmp("producer-1", "iid-1"),
        )
        .await?;
    assert_eq!(id1, id1_again);
    assert_eq!(1, client.xlen("mystream").await?);

    // A different producer may reuse the same iid: tracking is per producer.
    let id2: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default().idmp("producer-2", "iid-1"),
        )
        .await?;
    assert_ne!(id1, id2);
    assert_eq!(2, client.xlen("mystream").await?);

    // Automatic mode: the server derives the iid from the entry's content, so
    // the same content sent twice is deduplicated.
    let id3: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "auto")],
            XAddOptions::default().idmp_auto("producer-3"),
        )
        .await?;
    let id3_again: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "auto")],
            XAddOptions::default().idmp_auto("producer-3"),
        )
        .await?;
    assert_eq!(id3, id3_again);
    assert_eq!(3, client.xlen("mystream").await?);

    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!(Some(3), result.pids_tracked);
    assert_eq!(Some(3), result.iids_tracked);
    assert_eq!(Some(3), result.iids_added);
    assert_eq!(Some(2), result.iids_duplicates);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xcfgset() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let _: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;

    // Server-wide defaults, before any per-stream override.
    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!(Some(100), result.idmp_duration);
    assert_eq!(Some(100), result.idmp_maxsize);

    client
        .xcfgset(
            "mystream",
            XCfgSetOptions::default()
                .idmp_duration(300)
                .idmp_maxsize(50),
        )
        .await?;

    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!(Some(300), result.idmp_duration);
    assert_eq!(Some(50), result.idmp_maxsize);

    // Each parameter can be set on its own.
    client
        .xcfgset("mystream", XCfgSetOptions::default().idmp_duration(600))
        .await?;

    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!(Some(600), result.idmp_duration);
    assert_eq!(Some(50), result.idmp_maxsize);

    // At least one parameter is required, and the stream must exist.
    let result = client.xcfgset("mystream", XCfgSetOptions::default()).await;
    assert!(result.is_err());

    let result = client
        .xcfgset("unknown", XCfgSetOptions::default().idmp_duration(300))
        .await;
    assert!(result.is_err());

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
async fn xgroup_setid() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let id: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;

    client
        .xgroup_create("mystream", "mygroup", "$", XGroupCreateOptions::default())
        .await?;

    let results = client.xinfo_groups("mystream").await?;
    assert_eq!(id, results[0].last_delivered_id);

    // Rewinding to 0-0 makes the group deliver the existing entry again.
    client
        .xgroup_setid("mystream", "mygroup", "0-0", None)
        .await?;

    let results = client.xinfo_groups("mystream").await?;
    assert_eq!("0-0", results[0].last_delivered_id);
    assert_eq!(None, results[0].entries_read);

    // ENTRIESREAD seeds the counter the lag is derived from.
    client
        .xgroup_setid("mystream", "mygroup", "0-0", Some(1))
        .await?;

    let results = client.xinfo_groups("mystream").await?;
    assert_eq!(Some(1), results[0].entries_read);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xgroup_delconsumer() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let _: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;
    client
        .xgroup_create("mystream", "mygroup", "0-0", XGroupCreateOptions::default())
        .await?;

    // Bob reads the entry without acknowledging it, so deleting him reports the
    // one message he still owned.
    let _: Value = client
        .xreadgroup(
            "mygroup",
            "Bob",
            XReadGroupOptions::default().count(1),
            "mystream",
            ">",
        )
        .await?;

    let pending = client
        .xgroup_delconsumer("mystream", "mygroup", "Bob")
        .await?;
    assert_eq!(1, pending);

    let results = client.xinfo_consumers("mystream", "mygroup").await?;
    assert!(results.is_empty());

    // Deleting an unknown consumer is not an error.
    let pending = client
        .xgroup_delconsumer("mystream", "mygroup", "Alice")
        .await?;
    assert_eq!(0, pending);

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

    let results: Vec<XPendingMessageResult> = client
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
async fn xsetid() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let _: String = client
        .xadd(
            "mystream",
            "5-1",
            [("field", "value")],
            XAddOptions::default(),
        )
        .await?;

    // The last id can only move forward.
    client
        .xsetid("mystream", "100-0", XSetIdOptions::default())
        .await?;
    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!("100-0", result.last_generated_id);

    let result = client
        .xsetid("mystream", "1-1", XSetIdOptions::default())
        .await;
    assert!(result.is_err());

    // The two counters a replica needs are settable on their own.
    client
        .xsetid(
            "mystream",
            "200-0",
            XSetIdOptions::default()
                .entries_added(42)
                .max_deleted_id("7-3"),
        )
        .await?;
    let result = client
        .xinfo_stream("mystream", XInfoStreamOptions::default())
        .await?;
    assert_eq!("200-0", result.last_generated_id);
    assert_eq!(42, result.entries_added);
    assert_eq!("7-3", result.max_deleted_entry_id);

    // The stream must exist.
    let result = client
        .xsetid("unknown", "1-1", XSetIdOptions::default())
        .await;
    assert!(result.is_err());

    Ok(())
}

#[test]
fn xsetid_args() -> Result<()> {
    let cmd = TestClient
        .xsetid("key", "100-0", XSetIdOptions::default())
        .command;
    assert_eq!("XSETID key 100-0", &cmd.to_string());

    let cmd = TestClient
        .xsetid(
            "key",
            "100-0",
            XSetIdOptions::default()
                .entries_added(42)
                .max_deleted_id("7-3"),
        )
        .command;
    assert_eq!(
        "XSETID key 100-0 ENTRIESADDED 42 MAXDELETEDID 7-3",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn xclaim_lastid_args() -> Result<()> {
    let cmd = TestClient
        .xclaim::<()>(
            "key",
            "group",
            "consumer",
            0,
            "1-1",
            XClaimOptions::default().last_id("5-5"),
        )
        .command;
    assert_eq!(
        "XCLAIM key group consumer 0 1-1 LASTID 5-5",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn xadd_idmp_args() -> Result<()> {
    // The idempotency clause sits between the consumer-group policy and the
    // trimming clause, as the XADD grammar requires.
    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default()
                .no_mk_stream()
                .consumer_group_options(ConsumerGroupOptions::DelRef)
                .idmp("producer-1", "iid-1")
                .trim_options(XTrimOptions::max_len(None, 1000)),
        )
        .command;
    assert_eq!(
        "XADD key NOMKSTREAM DELREF IDMP producer-1 iid-1 MAXLEN 1000 * field value",
        &cmd.to_string()
    );

    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default().idmp_auto("producer-1"),
        )
        .command;
    assert_eq!(
        "XADD key IDMPAUTO producer-1 * field value",
        &cmd.to_string()
    );

    // The two modes are mutually exclusive: the last one set wins.
    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default()
                .idmp("producer-1", "iid-1")
                .idmp_auto("producer-1"),
        )
        .command;
    assert_eq!(
        "XADD key IDMPAUTO producer-1 * field value",
        &cmd.to_string()
    );

    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default()
                .idmp_auto("producer-1")
                .idmp("producer-1", "iid-1"),
        )
        .command;
    assert_eq!(
        "XADD key IDMP producer-1 iid-1 * field value",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn xcfgset_args() -> Result<()> {
    let cmd = TestClient
        .xcfgset(
            "key",
            XCfgSetOptions::default()
                .idmp_duration(300)
                .idmp_maxsize(50),
        )
        .command;
    assert_eq!(
        "XCFGSET key IDMP-DURATION 300 IDMP-MAXSIZE 50",
        &cmd.to_string()
    );

    let cmd = TestClient
        .xcfgset("key", XCfgSetOptions::default().idmp_maxsize(50))
        .command;
    assert_eq!("XCFGSET key IDMP-MAXSIZE 50", &cmd.to_string());

    Ok(())
}

#[test]
fn xautoclaim_args() -> Result<()> {
    let cmd = TestClient
        .xclaim::<()>(
            "key",
            "group",
            "consumer",
            1000,
            "1526569498055-0",
            XClaimOptions::default()
                .idle_time(100)
                .time(1000)
                .retry_count(12)
                .force()
                .just_id(),
        )
        .command;
    assert_eq!(
        "XCLAIM key group consumer 1000 1526569498055-0 IDLE 100 TIME 1000 RETRYCOUNT 12 FORCE JUSTID",
        &cmd.to_string()
    );

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
        .xtrim("mystream", XTrimOptions::max_len(None, 1))
        .await?;
    assert_eq!(1, deleted);

    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
    assert_eq!(1, results.len());
    assert_eq!(id2, results[0].stream_id);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xtrim_entries_deletion_policy() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    for _ in 0..3 {
        let _: String = client
            .xadd(
                "mystream",
                "*",
                [("field", "value")],
                XAddOptions::default(),
            )
            .await?;
    }

    // XTRIM with an explicit entry-deletion policy (Redis 8.2).
    let deleted = client
        .xtrim(
            "mystream",
            XTrimOptions::max_len(None, 2).entries_deletion(StreamEntryDeletionPolicy::DelRef),
        )
        .await?;
    assert_eq!(1, deleted);

    // XADD may carry the same policy inside its trim clause.
    let _: String = client
        .xadd(
            "mystream",
            "*",
            [("field", "value")],
            XAddOptions::default().trim_options(
                XTrimOptions::max_len(None, 1).entries_deletion(StreamEntryDeletionPolicy::KeepRef),
            ),
        )
        .await?;

    let results: Vec<StreamEntry<String>> = client.xrange("mystream", "-", "+", None).await?;
    assert_eq!(1, results.len());

    Ok(())
}

/// `XGROUP HELP` answers the subcommand list as a flat array of text lines,
/// which is the shape the declared return type claims.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xgroup_help() -> Result<()> {
    let client = get_test_client().await?;

    let help = client.xgroup_help().await?;

    assert!(help.iter().any(|line| line.contains("CREATE")));

    Ok(())
}

/// `XINFO HELP` answers the subcommand list as a flat array of text lines,
/// which is the shape the declared return type claims.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn xinfo_help() -> Result<()> {
    let client = get_test_client().await?;

    let help = client.xinfo_help().await?;

    assert!(help.iter().any(|line| line.contains("STREAM")));

    Ok(())
}
