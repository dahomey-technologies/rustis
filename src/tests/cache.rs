use crate::{
    ClientError, Error, Result,
    cache::Cache,
    client::Client,
    commands::{
        ClientTrackingOptions, ClientTrackingStatus, ClusterCommands, ConnectionCommands,
        FlushingMode, HashCommands, ServerCommands, StringCommands,
    },
    network::sleep,
    resp::cmd,
    tests::{get_cluster_test_client, log_try_init},
};
use serial_test::serial;
use std::time::Duration;

#[tokio::test]
#[serial]
async fn cache_get() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    client2.set("key", "value").await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;

    let value: String = cache.get("key").await?;
    assert_eq!("value", value);

    let value: String = cache.get("key").await?;
    assert_eq!("value", value);

    client2.set("key", "new_value").await?;

    sleep(Duration::from_millis(100)).await;

    let value: String = cache.get("key").await?;
    assert_eq!("new_value", value);

    let value: String = cache.get("key").await?;
    assert_eq!("new_value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn cache_key_serializing_to_zero_args_errors_instead_of_panicking() -> Result<()> {
    log_try_init();
    let client = Client::connect("redis://127.0.0.1?connection_name=cache_bad_key").await?;
    let cache = Cache::new(client, 60, ClientTrackingOptions::default()).await?;

    // A key that serializes to zero arguments (e.g. `None`) must surface a clean
    // error rather than panicking the caller thread.
    let result: Result<String> = cache.get(None::<String>).await;
    assert!(
        matches!(result, Err(Error::Client(ClientError::InvalidCacheKey))),
        "expected InvalidCacheKey, got {result:?}"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn cache_hash() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    client2
        .hset("key", [("field1", "value1"), ("field2", "value2")])
        .await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value1".to_string()),
            ("field2".to_string(), "value2".to_string())
        ],
        values
    );

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value1".to_string()),
            ("field2".to_string(), "value2".to_string())
        ],
        values
    );

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    client2
        .hset("key", [("field1", "value11"), ("field2", "value22")])
        .await?;

    // The invalidation reaches `client1` on its own connection and is applied by
    // a separate task, so `client2`'s write completing says nothing about when
    // the entry is evicted. Wait for it, as `cache_get` and
    // `cache_survives_reconnection` already do.
    sleep(Duration::from_millis(100)).await;

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value11".to_string()),
            ("field2".to_string(), "value22".to_string())
        ],
        values
    );

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value11".to_string()),
            ("field2".to_string(), "value22".to_string())
        ],
        values
    );

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    Ok(())
}

#[tokio::test]
#[serial]
async fn cache_mget() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;

    client2
        .mset([("key1", "value1"), ("key2", "value2")])
        .await?;

    assert_eq!("value1", cache.get::<String>("key1").await?);

    let values: Vec<String> = cache.mget(["key1", "key2"]).await?;
    assert_eq!(vec!["value1".to_string(), "value2".to_string()], values);

    assert_eq!("value1", cache.get::<String>("key1").await?);
    assert_eq!("value2", cache.get::<String>("key2").await?);

    let values: Vec<String> = cache.mget(["key1", "key2"]).await?;
    assert_eq!(vec!["value1".to_string(), "value2".to_string()], values);

    Ok(())
}

/// Server-side tracking is per-connection state that dies with the socket. After a
/// reconnection, the cache must both drop everything it holds — invalidations missed
/// during the outage are unrecoverable — and re-arm tracking so later writes keep
/// invalidating it.
#[tokio::test]
#[serial]
async fn cache_survives_reconnection() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    client2.set("key", "value").await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;
    assert_eq!("value", cache.get::<String>("key").await?);

    // Close the connection under the cache, then write the key while it is down:
    // the invalidation for that write can never be delivered.
    client1.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;
    client2.set("key", "new_value").await?;

    sleep(Duration::from_millis(2500)).await;

    assert_eq!(
        "new_value",
        cache.get::<String>("key").await?,
        "a value written during the outage must not be served from the cache"
    );

    // Tracking must be armed again on the new connection, otherwise invalidations
    // silently stop for the rest of the client's life.
    client2.set("key", "newer_value").await?;
    sleep(Duration::from_millis(100)).await;

    assert_eq!(
        "newer_value",
        cache.get::<String>("key").await?,
        "invalidations must still be delivered after a reconnection"
    );

    Ok(())
}

/// Tracking is per-connection and each node only invalidates the keys it holds,
/// so a cluster cache is only correct if tracking reached every node. Keys are
/// picked to land on more than one shard: `key1` hashes into the middle range,
/// `key2` and `key3` into the first.
#[tokio::test]
#[serial]
async fn cluster_cache_is_invalidated_for_keys_on_every_shard() -> Result<()> {
    log_try_init();
    let cached_client = get_cluster_test_client().await?;
    let writer = get_cluster_test_client().await?;

    writer.flushall(FlushingMode::Sync).await?;
    for key in ["key1", "key2", "key3"] {
        writer.set(key, "before").await?;
    }

    // The keys must not all live on the same shard, otherwise the test would pass
    // with tracking armed on a single node.
    let mut slots = Vec::new();
    for key in ["key1", "key2", "key3"] {
        slots.push(writer.cluster_keyslot(key).await?);
    }
    assert!(
        slots.iter().any(|s| *s > 5460),
        "the probe keys must span more than one shard, slots: {slots:?}"
    );

    let cache = Cache::new(cached_client.clone(), 60, ClientTrackingOptions::default()).await?;

    for key in ["key1", "key2", "key3"] {
        assert_eq!("before", cache.get::<String>(key).await?);
    }

    for key in ["key1", "key2", "key3"] {
        writer.set(key, "after").await?;
    }
    sleep(Duration::from_millis(200)).await;

    for key in ["key1", "key2", "key3"] {
        assert_eq!(
            "after",
            cache.get::<String>(key).await?,
            "the cache must be invalidated for `{key}`, whichever shard holds it"
        );
    }

    Ok(())
}
