use crate::resp::{BulkString, RespResponse};
use bytes::Bytes;
use dashmap::DashMap;
use std::{future::Future, sync::Arc};

/// A server reply as a [`CacheStore`] holds it: opaque, cheap to clone, and
/// already detached from the network buffers it was decoded from.
///
/// A store keeps it and hands it back; it cannot read it. The bytes stay inside
/// the crate because handing them out is what would pin a recycled read buffer
/// for as long as the cache holds the entry. So a store can live anywhere in the
/// process — shared between clients, with an eviction policy of its own, counted
/// or instrumented — but cannot serialize an entry to disk or to another
/// machine.
#[derive(Debug, Clone)]
pub struct CachedValue(RespResponse);

impl CachedValue {
    pub(crate) fn new(response: RespResponse) -> Self {
        Self(response)
    }

    pub(crate) fn into_response(self) -> RespResponse {
        self.0
    }
}

/// Where a [`Cache`](crate::cache::Cache) keeps what it has already read.
///
/// The default store is [`MokaStore`], an in-process cache with a TTL and a
/// capacity. Implement this to back the cache with something else: a store
/// shared by several clients, one with an eviction policy of its own, one that
/// counts its hits, or one that survives the process.
///
/// # The two levels
///
/// An entry is addressed by a pair: the Redis `key`, which is what an
/// invalidation names, and a `subkey` identifying the exact command that read
/// it — `GET k` and `GETRANGE k 0 3` are different values of the same key. So a
/// store must be able to drop every subkey of one key at once, which is what
/// [`invalidate`](Self::invalidate) does; how it lays that out is its own
/// business.
///
/// # What the cache guarantees
///
/// Every method is called from the caller's task, except
/// [`invalidate`](Self::invalidate) and [`invalidate_all`](Self::invalidate_all),
/// which also run on the task reading the server's invalidation pushes. So a
/// store must be `Sync` and must not block.
///
/// Nothing here returns an error: a local cache that cannot answer answers
/// [`None`] and the value is read from the server instead. A store that cannot
/// *invalidate*, however, would serve stale data for good — one that can fail
/// that way must drop everything rather than return quietly.
///
/// Staleness is not the store's problem otherwise: the cache re-checks the
/// invalidation generation after every insert and removes what raced it.
///
/// # Writing one
///
/// Implement the three async methods with plain `async fn`; the
/// `-> impl Future + Send` shape is what the declarations need to say, and an
/// `async fn` in the impl satisfies it. Nothing is boxed: a hit is the path this
/// whole feature exists to make fast, and [`Cache`](crate::cache::Cache) is
/// generic over the store rather than holding a `dyn`, which is what lets the
/// futures stay unboxed.
pub trait CacheStore: Send + Sync + 'static {
    /// The value cached for this exact command, or `None`.
    fn get(
        &self,
        key: &BulkString,
        subkey: &Bytes,
    ) -> impl Future<Output = Option<CachedValue>> + Send;

    /// Records `response` for this exact command, replacing any previous value.
    fn insert(
        &self,
        key: BulkString,
        subkey: Bytes,
        response: CachedValue,
    ) -> impl Future<Output = ()> + Send;

    /// Drops every value cached under `key`, whatever the command that read it.
    ///
    /// This is what an invalidation from the server maps to, so it must leave
    /// nothing behind.
    fn invalidate(&self, key: &BulkString) -> impl Future<Output = ()> + Send;

    /// Drops everything.
    ///
    /// Called when invalidations were lost — shed under backpressure, or missed
    /// while the connection was down — so the cache no longer knows what is
    /// stale. Anything short of emptying the store serves stale data.
    fn invalidate_all(&self);
}

/// The sub-cache [`MokaStore`] keeps under one Redis key: one entry per
/// distinct command read from it.
type SubCache = DashMap<Bytes, CachedValue>;
type MokaCache = moka::future::Cache<BulkString, Arc<SubCache>>;

/// A [`CacheStore`] builder over [`moka`], the default store.
pub type MokaStoreBuilder = moka::future::CacheBuilder<BulkString, Arc<SubCache>, MokaCache>;

/// The default [`CacheStore`]: an in-process [`moka`] cache with a TTL and a
/// capacity, holding one sub-map per Redis key.
///
/// The two levels are what make an invalidation one `moka` removal rather than a
/// scan: the server names a key, and every command that read it goes with it.
pub struct MokaStore(MokaCache);

impl MokaStore {
    /// Builds a store from a [`moka`] builder, for a TTL, a capacity or an
    /// eviction listener the defaults do not give.
    pub fn from_builder(builder: MokaStoreBuilder) -> Self {
        Self(builder.build())
    }

    /// Builds a store holding at most `max_capacity` keys for `ttl`.
    pub fn new(ttl: std::time::Duration, max_capacity: u64) -> Self {
        Self::from_builder(
            MokaCache::builder()
                .time_to_live(ttl)
                .max_capacity(max_capacity),
        )
    }
}

impl CacheStore for MokaStore {
    async fn get(&self, key: &BulkString, subkey: &Bytes) -> Option<CachedValue> {
        let values = self.0.get(key).await?;
        let response = values.get(subkey)?;
        Some(response.clone())
    }

    async fn insert(&self, key: BulkString, subkey: Bytes, response: CachedValue) {
        self.0
            .entry(key)
            .or_insert_with(async { Arc::new(DashMap::new()) })
            .await
            .value()
            .insert(subkey, response);
    }

    async fn invalidate(&self, key: &BulkString) {
        self.0.invalidate(key).await;
    }

    fn invalidate_all(&self) {
        self.0.invalidate_all();
    }
}
