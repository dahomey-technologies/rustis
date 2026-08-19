use crate::{
    ClientError, Error, ErrorKind, Result, TimeoutKind,
    cache::Cache,
    client::Client,
    commands::{
        ClientTrackingOptions, ClientTrackingStatus, ClusterCommands, ConnectionCommands,
        FlushingMode, HashCommands, ServerCommands, StringCommands,
    },
    network::{sleep, timeout},
    resp::cmd,
    tests::{get_cluster_test_client, get_default_config, get_test_client, log_try_init},
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
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::InvalidCacheKey)
        ),
        "expected InvalidCacheKey, got {error:?}"
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

/// Losing an invalidation must cost a cache flush, not a stale read.
///
/// This is the one path in the backpressure work whose failure is a correctness
/// bug rather than a memory one: an invalidation discarded to stay within budget
/// names a key that will never be named again, so acting only on the survivors
/// would leave that key cached and served for good.
///
/// The scenario forces exactly that. The budget is one byte, so any burst that
/// leaves two messages queued at once evicts; the tracked key is written
/// *first*, so its invalidation is the oldest and therefore the prime candidate
/// for eviction; and thousands of writes follow to guarantee the burst. The
/// flush is waited for rather than slept on — the cache exposes its flush
/// generation for that.
#[tokio::test]
#[serial]
async fn a_lost_invalidation_flushes_the_cache_instead_of_serving_stale_data() -> Result<()> {
    log_try_init();

    const PREFIX: &str = "lost_invalidation_key_";
    const OTHER_KEYS: usize = 5_000;

    let mut config = get_default_config()?;
    // The smallest non-zero budget: `0` would mean unbounded.
    config.backpressure.max_push_bytes = 1;
    let cached_client = Client::connect(config).await?;
    let writer = get_test_client().await?;

    let target = format!("{PREFIX}target");
    writer.set(&target, "v1").await?;

    let cache = Cache::new(
        cached_client.clone(),
        60,
        ClientTrackingOptions::default()
            .prefix(PREFIX)
            .broadcasting(),
    )
    .await?;

    let value: String = cache.get(&target).await?;
    assert_eq!("v1", value, "the value under test must start out cached");

    // The tracked key changes first, so its invalidation is the oldest queued
    // and the one drop-oldest sheds. The rest of the burst is what overruns the
    // budget in the first place.
    //
    // Not awaited, and this is the whole point: awaiting it would let its
    // invalidation arrive alone and be handled key by key, so the test would pass
    // without the flush ever mattering. Pipelined into the burst, it is the oldest
    // queued message when the budget is breached.
    writer.send_and_forget(cmd("SET").arg(&target).arg("v2"), None)?;
    for i in 0..OTHER_KEYS {
        writer.send_and_forget(cmd("SET").arg(format!("{PREFIX}{i}")).arg("v"), None)?;
    }
    let _: String = writer.send(cmd("PING"), None).await?;

    timeout(Duration::from_secs(30), TimeoutKind::Command, async {
        while cache.flush_generation() == 0 {
            tokio::task::yield_now().await;
        }
        Ok::<(), Error>(())
    })
    .await??;

    let value: String = cache.get(&target).await?;
    assert_eq!(
        "v2", value,
        "a dropped invalidation must have flushed the cache, not left a stale value"
    );

    cached_client
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;
    Ok(())
}

/// A store of the caller's own serves the hits and receives the invalidations —
/// the two things a cache backed by anything other than `moka` has to prove.
#[tokio::test]
#[serial]
async fn a_cache_store_of_your_own_serves_the_hits_and_takes_the_invalidations() -> Result<()> {
    use crate::{
        cache::{CacheStore, CachedValue},
        resp::BulkString,
    };
    use bytes::Bytes;
    use std::{
        collections::HashMap,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    #[derive(Default)]
    struct CountingStore {
        entries: Mutex<HashMap<(BulkString, Bytes), CachedValue>>,
        hits: AtomicUsize,
        invalidations: AtomicUsize,
    }

    impl CacheStore for std::sync::Arc<CountingStore> {
        async fn get(&self, key: &BulkString, subkey: &Bytes) -> Option<CachedValue> {
            let found = self
                .entries
                .lock()
                .ok()?
                .get(&(key.clone(), subkey.clone()))
                .cloned();
            if found.is_some() {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
            found
        }

        async fn insert(&self, key: BulkString, subkey: Bytes, response: CachedValue) {
            if let Ok(mut entries) = self.entries.lock() {
                entries.insert((key, subkey), response);
            }
        }

        async fn invalidate(&self, key: &BulkString) {
            self.invalidations.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut entries) = self.entries.lock() {
                entries.retain(|(entry_key, _), _| entry_key != key);
            }
        }

        fn invalidate_all(&self) {
            if let Ok(mut entries) = self.entries.lock() {
                entries.clear();
            }
        }
    }

    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=custom_store").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=custom_store_writer").await?;
    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;
    client2.set("store_key", "value").await?;

    let store = std::sync::Arc::new(CountingStore::default());
    let cache = Cache::with_store(
        client1.clone(),
        std::sync::Arc::clone(&store),
        ClientTrackingOptions::default(),
    )
    .await?;

    // Miss, then a hit that can only have come from the store.
    let value: String = cache.get("store_key").await?;
    assert_eq!("value", value);
    assert_eq!(0, store.hits.load(Ordering::SeqCst));
    let value: String = cache.get("store_key").await?;
    assert_eq!("value", value);
    assert_eq!(1, store.hits.load(Ordering::SeqCst));

    // The server's invalidation must reach the store, not a moka cache behind it.
    client2.set("store_key", "new_value").await?;
    sleep(Duration::from_millis(100)).await;
    assert!(store.invalidations.load(Ordering::SeqCst) >= 1);
    let value: String = cache.get("store_key").await?;
    assert_eq!("new_value", value);

    // A read must stay spawnable. `CacheStore` declares `-> impl Future + Send`
    // rather than a bare `async fn` for exactly this: a bare one makes no
    // Send promise, and the whole `Cache::get` future would stop being Send —
    // a compile error at every `tokio::spawn`, in the user's code, not here.
    let spawned = tokio::spawn(async move {
        let value: String = cache.get("store_key").await?;
        Ok::<String, Error>(value)
    });
    assert_eq!("new_value", spawned.await.expect("task panicked")?);

    Ok(())
}
