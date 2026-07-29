//! [Client-side caching](https://redis.io/docs/latest/develop/reference/client-side-caching/) support
use crate::{
    ClientError, Error, Result,
    client::{Client, PreparedCommand},
    commands::{
        BitFieldSubCommand, BitRange, BitmapCommands, ClientTrackingOptions, ClientTrackingStatus,
        ConnectionCommands, HashCommands, ListCommands, SetCommands, SortedSetCommands,
        StringCommands, ZRangeOptions,
    },
    network::{JoinHandle, spawn},
    resp::{
        BulkString, Command, CommandArgsMut, FastPathCommandBuilder, RespDeserializer,
        RespResponse, Response,
    },
};
use bytes::Bytes;
use dashmap::DashMap;
use futures_util::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

/// Re-export the moka cache builder.
pub use moka::future::CacheBuilder;

type SubCache = DashMap<Bytes, RespResponse>;
type MokaCache = moka::future::Cache<BulkString, Arc<SubCache>>;
type MokaCacheBuilder = moka::future::CacheBuilder<BulkString, Arc<SubCache>, MokaCache>;

/// A local client-side Redis cache with RESP3 tracking-based invalidation.
///
/// The `Cache` struct wraps a Moka async cache and maintains Redis key-based
/// invalidation using the `CLIENT TRACKING` feature from Redis 6+.
///
/// It transparently caches the results of read-only Redis commands (`GET`, `HGET`, etc.)
/// keyed by the Redis key and the specific command arguments used. When Redis sends an
/// invalidation message for a key, all cached entries under that key are automatically
/// invalidated.
///
/// Internally, the cache uses a `moka::future::Cache<String, Arc<DashMap<CommandArgs, resp::Value>>>`:
/// - The outer key is the Redis key (`String`)
/// - The inner `DashMap` holds one entry per distinct command issued on that key,
///   with `CommandArgs` (e.g., `["HGET", "myhash", "field1"]`) as subkeys.
///
/// # Examples
///
/// ```rust
/// use rustis::{client::Client, Result, cache::Cache, commands::{ClientTrackingOptions}};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = Client::connect("127.0.0.1:6379").await?;
///     let tracking_opts = ClientTrackingOptions::default().broadcasting().noloop();
///
///     let cache = Cache::new(client.clone(), 60, tracking_opts).await?;
///
///     let value: String = cache.get("mykey").await?;
///     let field: String = cache.hget("myhash", "field1").await?;
///
///     Ok(())
/// }
/// ```
///
/// # Limitations
/// - Only works with commands supported by Redis' client-side caching (typically `@read`)
/// - Invalidations are only at the Redis key level; field-level invalidation in hashes/lists
///   must be handled at the application layer if needed.
///
/// # See also
/// - [`CLIENT TRACKING`](https://redis.io/docs/latest/develop/client-side-caching/)
/// - [`moka`](https://docs.rs/moka)
pub struct Cache {
    cache: Arc<MokaCache>,
    client: Client,
    /// Monotonic counter bumped once per received invalidation. A fetch samples
    /// it before sending; the sampled value is compared at insert time to detect
    /// an invalidation that raced the in-flight response (see `process_command`).
    generation_counter: Arc<AtomicU64>,
    /// Last `generation_counter` value at which each key was invalidated. Only
    /// keys with an in-flight or recent invalidation appear here; entries are
    /// pruned when the key is next inserted cleanly.
    key_generations: Arc<DashMap<BulkString, u64>>,
    /// `generation_counter` value at the last whole-cache flush.
    ///
    /// A flush happens when invalidations were lost — dropped under
    /// backpressure, or missed while the connection was down. It names no key,
    /// because the lost messages named keys nobody will ever learn, so any fetch
    /// that sampled before it must discard its result whatever its own key's
    /// record says.
    flush_generation: Arc<AtomicU64>,
    #[allow(dead_code)]
    invalidation_task: JoinHandle<()>,
    #[allow(dead_code)]
    reconnection_task: JoinHandle<()>,
}

impl Cache {
    /// Create cache from a moka CacheBuilder and activates Redis client tracking invalidations
    #[allow(clippy::type_complexity)]
    pub(crate) async fn from_builder(
        client: Client,
        builder: MokaCacheBuilder,
        tracking_opts: ClientTrackingOptions,
    ) -> Result<Arc<Self>> {
        client
            .client_tracking(ClientTrackingStatus::On, tracking_opts.clone())
            .await?;

        let stream = client.create_client_tracking_invalidation_stream()?;

        let cache = Arc::new(builder.build());
        let cache_clone = cache.clone();

        let generation_counter = Arc::new(AtomicU64::new(0));
        let key_generations: Arc<DashMap<BulkString, u64>> = Arc::new(DashMap::new());
        let flush_generation = Arc::new(AtomicU64::new(0));

        let connection_tag = client.connection_tag().to_owned();
        let counter_clone = generation_counter.clone();
        let key_generations_clone = key_generations.clone();
        let flush_generation_clone = flush_generation.clone();
        let invalidation_task = spawn(async move {
            let mut stream = stream;
            let mut dropped_seen = 0usize;
            while let Some(keys) = stream.next().await {
                // The invalidation channel is bounded, and it sheds the oldest
                // messages when a burst outruns this task. Those messages name
                // keys that are now stale and will never be named again, so
                // acting only on what survived would leave them cached and
                // served for good. Losing invalidations means no longer knowing
                // what is stale — the same situation as after a reconnection
                // (see below), and it takes the same answer: drop everything.
                let dropped = stream.dropped_messages();
                if dropped != dropped_seen {
                    tracing::warn!(
                        tag = %connection_tag,
                        "Dropped {} invalidation message(s) under backpressure; \
                         invalidating the whole client cache",
                        dropped - dropped_seen
                    );
                    dropped_seen = dropped;
                    // Record the flush at a fresh generation, so a fetch already
                    // in flight — which sampled the counter before this point —
                    // discards its value instead of re-inserting it after the
                    // flush. A per-key record cannot express this: the dropped
                    // messages named keys we never saw.
                    let generation = counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    flush_generation_clone.store(generation, Ordering::SeqCst);
                    cache_clone.invalidate_all();
                }

                for key in keys {
                    tracing::debug!(
                        tag = %connection_tag,
                        "Invalidating key `{key}` from client cache"
                    );
                    // Record the invalidation before removing the entry, so a
                    // fetch that samples the counter after this point and inserts
                    // afterwards observes the newer generation and drops its stale
                    // value (see `process_command`). Ordering is `SeqCst` so the
                    // bump and the record cannot be reordered past the sample.
                    let generation = counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
                    key_generations_clone.insert(key.clone(), generation);
                    cache_clone.invalidate(&key).await;
                }
            }
        });

        // Server-side tracking is per-connection state: it dies with the socket and
        // nothing on the server restores it. The invalidation stream itself survives
        // a reconnection, so without this the cache would keep answering hits while
        // silently never being invalidated again.
        let cache_clone = cache.clone();
        let client_clone = client.clone();
        let connection_tag = client.connection_tag().to_owned();
        let mut on_reconnect = client.on_reconnect();
        let counter_clone = generation_counter.clone();
        let flush_generation_clone = flush_generation.clone();
        let reconnection_task = spawn(async move {
            while on_reconnect.recv().await.is_ok() {
                tracing::debug!(tag = %connection_tag, "Re-enabling client tracking after reconnection");

                // Invalidations emitted while the connection was down are lost for
                // good, so every entry must be considered stale. A partial refresh
                // cannot be correct here. Marking the flush is what also protects
                // a fetch that was in flight across the reconnection: without it,
                // that fetch would re-insert its value after the flush.
                let generation = counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
                flush_generation_clone.store(generation, Ordering::SeqCst);
                cache_clone.invalidate_all();

                if let Err(e) = client_clone
                    .client_tracking(ClientTrackingStatus::On, tracking_opts.clone())
                    .await
                {
                    tracing::error!(
                        tag = %connection_tag,
                        "Cannot re-enable client tracking after reconnection: {e}"
                    );
                }
            }
        });

        Ok(Arc::new(Self {
            cache,
            client,
            generation_counter,
            key_generations,
            flush_generation,
            invalidation_task,
            reconnection_task,
        }))
    }

    pub async fn new(
        client: Client,
        ttl_secs: u64,
        tracking_opts: ClientTrackingOptions,
    ) -> Result<Arc<Self>> {
        let builder = MokaCache::builder()
            .time_to_live(Duration::from_secs(ttl_secs))
            .max_capacity(10_000);
        Self::from_builder(client, builder, tracking_opts).await
    }

    /// Generation of the last whole-cache flush, `0` if none happened.
    ///
    /// A test needs this to wait for the flush instead of sleeping: the `Cache`
    /// owns its invalidation stream, so a caller has no other way to know that a
    /// lost invalidation has been reacted to.
    #[cfg(test)]
    pub(crate) fn flush_generation(&self) -> u64 {
        self.flush_generation.load(Ordering::SeqCst)
    }

    /// Executes the `GET` command with client-side caching.
    pub async fn get<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.get(key))
            .await
    }

    /// Executes the `MGET` command with client-side caching.
    pub async fn mget<R: Response + DeserializeOwned>(&self, keys: impl Serialize) -> Result<R> {
        let prepared_command = self.client.mget::<R>(keys);
        let mut responses = Vec::with_capacity(prepared_command.command.num_args());
        let mut missing_indices = Vec::new();
        let mut missing_keys = Vec::new();
        // Subcache key (`GET <key>` bytes) for each missing key, computed once
        // during the probe below and reused at insert time.
        let mut missing_subcache_keys = Vec::new();

        // 1. check cache
        for (i, arg) in prepared_command.command.args().enumerate() {
            let key = BulkString::from(arg.clone());
            let subcache_key = get_subcache_key(&key);

            if let Some(values) = self.cache.get(&key).await
                && let Some(response) = values.get(&subcache_key)
            {
                tracing::debug!(
                    tag = %self.client.connection_tag(),
                    "Cache hit on key `{key}`"
                );
                responses.push(response.clone());
            } else {
                tracing::debug!(
                    tag = %self.client.connection_tag(),
                    "Cache miss on key `{key}`"
                );
                responses.push(RespResponse::null());
                missing_indices.push(i);
                missing_keys.push(key);
                missing_subcache_keys.push(subcache_key);
            }
        }

        // 2. Fetch missing keys from Redis server if any
        if !missing_keys.is_empty() {
            let missing_prepared_command = self.client.mget::<R>(missing_keys);
            let response = self
                .client
                .internal_send(missing_prepared_command.command, None)
                .await?;
            let Ok(collection_iter) = response.clone().into_collection_iter() else {
                return Err(Error::Client(ClientError::ExpectedArrayForMGet));
            };

            for (idx_in_missing, response) in collection_iter.enumerate() {
                let response = response?;
                let original_idx = missing_indices[idx_in_missing];

                let Some(key) = prepared_command
                    .command
                    .get_arg(original_idx)
                    .map(BulkString::from)
                else {
                    break;
                };

                // Insert into cache. Compact first so a retained entry holds
                // only its own bytes instead of pinning the whole MGET reply
                // block every element still shares.
                self.cache
                    .entry(key)
                    .or_insert_with(async { Arc::new(DashMap::new()) })
                    .await
                    .value()
                    .insert(
                        missing_subcache_keys[idx_in_missing].clone(),
                        response.compact(),
                    );

                responses[original_idx] = response;
            }
        } else {
            tracing::debug!(tag = %self.client.connection_tag(), "Cache hit on mget");
        }

        // 3. deserialize
        let response = RespResponse::owned_array(responses);
        let deserializer = RespDeserializer::new(response.view()?);
        R::deserialize(deserializer)
    }

    /// Executes the `GETRANGE` command with client-side caching.
    pub async fn getrange<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        start: isize,
        end: isize,
    ) -> Result<R> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.getrange(key, start, end),
        )
        .await
    }

    /// Executes the `STRLEN` command with client-side caching.
    pub async fn strlen(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.strlen(key))
            .await
    }

    /// Executes the `HEXISTS` command with client-side caching.
    pub async fn hexists(&self, key: impl Serialize, field: impl Serialize) -> Result<bool> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.hexists(key_to_bulk_string(&key)?, field),
        )
        .await
    }

    /// Executes the `HGET` command with client-side caching.
    pub async fn hget<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        field: impl Serialize,
    ) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hget(key, field))
            .await
    }

    /// Executes the `HGETALL` command with client-side caching.
    pub async fn hgetall<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hgetall(key))
            .await
    }

    /// Executes the `HLEN` command with client-side caching.
    pub async fn hlen(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hlen(key))
            .await
    }

    /// Executes the `HKEYS` command with client-side caching.
    pub async fn hkeys<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hkeys(key))
            .await
    }

    /// Executes the `HKEYS` command with client-side caching.
    pub async fn hvals<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hvals(key))
            .await
    }

    /// Executes the `HSTRLEN` command with client-side caching.
    pub async fn hstrlen(&self, key: impl Serialize, field: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hstrlen(key, field))
            .await
    }

    /// Executes the `HMGET` command with client-side caching.
    pub async fn hmget<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.hmget(key, fields))
            .await
    }

    /// Executes the `LRANGE` command with client-side caching.
    pub async fn lrange<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        start: isize,
        stop: isize,
    ) -> Result<R> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.lrange(key, start, stop),
        )
        .await
    }

    /// Executes the `LLEN` command with client-side caching.
    pub async fn llen(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.llen(key))
            .await
    }

    /// Executes the `LINDEX` command with client-side caching.
    pub async fn lindex<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        index: isize,
    ) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.lindex(key, index))
            .await
    }

    /// Executes the `SMEMBERS` command with client-side caching.
    pub async fn smembers<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.smembers(key))
            .await
    }

    /// Executes the `SCARD` command with client-side caching.
    pub async fn scard(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.scard(key))
            .await
    }

    /// Executes the `SISMEMBER` command with client-side caching.
    pub async fn sismember(&self, key: impl Serialize, member: impl Serialize) -> Result<bool> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.sismember(key, member),
        )
        .await
    }

    /// Executes the `ZCARD` command with client-side caching.
    pub async fn zcard(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.zcard(key))
            .await
    }

    /// Executes the `ZCOUNT` command with client-side caching.
    pub async fn zcount(
        &self,
        key: impl Serialize,
        min: impl Serialize,
        max: impl Serialize,
    ) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.zcount(key, min, max))
            .await
    }

    /// Executes the `ZLEXCOUNT` command with client-side caching.
    pub async fn zlexcount(
        &self,
        key: impl Serialize,
        min: impl Serialize,
        max: impl Serialize,
    ) -> Result<usize> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.zlexcount(key, min, max),
        )
        .await
    }

    /// Executes the `ZRANGE` command with client-side caching.
    pub async fn zrange<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
        options: ZRangeOptions,
    ) -> Result<R> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.zrange(key, start, stop, options),
        )
        .await
    }

    /// Executes the `ZRANK` command with client-side caching.
    pub async fn zrank(
        &self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> Result<Option<usize>> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.zrank(key, member))
            .await
    }

    /// Executes the `ZREVRANK` command with client-side caching.
    pub async fn zrevrank(
        &self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> Result<Option<usize>> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.zrevrank(key, member))
            .await
    }

    /// Executes the `ZSCORE` command with client-side caching.
    pub async fn zscore(&self, key: impl Serialize, member: impl Serialize) -> Result<Option<f64>> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.zscore(key, member))
            .await
    }

    /// Executes the `BITCOUNT` command with client-side caching.
    pub async fn bitcount(&self, key: impl Serialize, range: BitRange) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.bitcount(key, range))
            .await
    }

    /// Executes the `BITPOS` command with client-side caching.
    pub async fn bitpos(&self, key: impl Serialize, bit: u64, range: BitRange) -> Result<usize> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.bitpos(key, bit, range),
        )
        .await
    }

    /// Executes the `GETBIT` command with client-side caching.
    pub async fn getbit(&self, key: impl Serialize, offset: u64) -> Result<u64> {
        self.process_prepared_command(key_to_bulk_string(&key)?, self.client.getbit(key, offset))
            .await
    }

    /// Executes the `BITFIELD_RO` command with client-side caching.
    pub async fn bitfield_readonly<'a>(
        &self,
        key: impl Serialize,
        sub_commands: impl IntoIterator<Item = BitFieldSubCommand<'a>> + Serialize,
    ) -> Result<Vec<u64>> {
        self.process_prepared_command(
            key_to_bulk_string(&key)?,
            self.client.bitfield_readonly(key, sub_commands),
        )
        .await
    }

    async fn process_prepared_command<'a, R>(
        &self,
        key: BulkString,
        prepared_command: PreparedCommand<'a, &'a Client, R>,
    ) -> Result<R>
    where
        R: Response + DeserializeOwned,
    {
        self.process_command(key, prepared_command.command).await
    }

    async fn process_command<R>(&self, key: BulkString, command: Command) -> Result<R>
    where
        R: Response + DeserializeOwned,
    {
        if let Some(values) = self.cache.get(&key).await
            && let Some(response) = values.get(command.bytes())
        {
            tracing::debug!(
                tag = %self.client.connection_tag(),
                "Cache hit on key `{key}`"
            );
            let deserializer = RespDeserializer::new(response.view()?);
            return R::deserialize(deserializer);
        }

        // Cache miss: fetch from Redis
        tracing::debug!(
            tag = %self.client.connection_tag(),
            "Cache miss on key `{key}`"
        );

        // Sample the invalidation counter *before* sending: any invalidation for
        // this key recorded at a higher generation raced our in-flight response,
        // so the value we are about to cache may already be stale.
        let generation_before = self.generation_counter.load(Ordering::SeqCst);

        let command_bytes = command.bytes().clone();
        let response = self.client.internal_send(command, None).await?;
        let deserializer = RespDeserializer::new(response.view()?);
        let deserialized = R::deserialize(deserializer)?;

        // Insert into cache. Compact first so a retained entry holds only its
        // own bytes instead of pinning the whole recycled network block it was
        // decoded from.
        let key_for_check = key.clone();
        self.cache
            .entry(key)
            .or_insert_with(async { Arc::new(DashMap::new()) })
            .await
            .value()
            .insert(command_bytes, response.compact());

        // If an invalidation for this key landed while the response was in flight,
        // drop what we just inserted rather than pinning a stale entry until TTL.
        // Biased toward safety: this only ever over-invalidates (a spurious later
        // miss), never serves stale data. If no invalidation raced, prune this
        // key's now-obsolete generation record so the map does not grow unbounded.
        let recorded = self.key_generations.get(&key_for_check).map(|g| *g);
        let flushed_at = self.flush_generation.load(Ordering::SeqCst);
        match post_insert_action(recorded, generation_before, flushed_at) {
            PostInsertAction::DropStale => {
                self.cache.invalidate(&key_for_check).await;
            }
            PostInsertAction::PruneGeneration => {
                self.key_generations.remove(&key_for_check);
            }
            PostInsertAction::Keep => {}
        }

        Ok(deserialized)
    }
}

/// Derives the subcache key (`GET <key>` RESP bytes) under which a value for
/// `key` is stored. Kept identical to the single-command `get` path so that
/// `get` and `mget` cross-hit on the same entry.
fn get_subcache_key(key: &BulkString) -> Bytes {
    FastPathCommandBuilder::get(key.clone()).bytes().clone()
}

fn key_to_bulk_string(key: &impl Serialize) -> Result<BulkString> {
    let args = CommandArgsMut::default().arg(key).freeze();
    args.into_iter()
        .next()
        .map(Into::into)
        .ok_or_else(|| Error::Client(ClientError::InvalidCacheKey))
}

/// What to do with a freshly inserted cache entry once the response is in, given
/// the key's last recorded invalidation generation and the counter value sampled
/// before the request was sent.
#[derive(Debug, PartialEq, Eq)]
enum PostInsertAction {
    /// An invalidation for this key raced the in-flight response — drop the entry.
    DropStale,
    /// A stale, older generation record is present — remove it to bound the map.
    PruneGeneration,
    /// No invalidation touched this key during the fetch — keep the entry.
    Keep,
}

/// Pure decision behind the insert-after-response race guard, split out so the
/// ordering logic is unit-testable without a live cache or a real race.
///
/// `flushed_at` is the generation of the last whole-cache flush, which happens
/// when invalidation messages were dropped under backpressure. Such a flush
/// carries no key: the dropped messages named keys nobody will ever learn, so a
/// fetch that sampled before it cannot be trusted whatever its own key's record
/// says. It is checked first for that reason.
fn post_insert_action(
    recorded_generation: Option<u64>,
    sampled_before: u64,
    flushed_at: u64,
) -> PostInsertAction {
    if sampled_before < flushed_at {
        return PostInsertAction::DropStale;
    }
    match recorded_generation {
        Some(generation) if generation > sampled_before => PostInsertAction::DropStale,
        Some(_) => PostInsertAction::PruneGeneration,
        None => PostInsertAction::Keep,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::{PostInsertAction, post_insert_action};

    #[test]
    fn no_invalidation_recorded_keeps_entry() {
        assert_eq!(PostInsertAction::Keep, post_insert_action(None, 5, 0));
    }

    #[test]
    fn invalidation_after_sample_drops_stale_entry() {
        // Sampled 5 before sending; key invalidated at generation 6 in flight.
        assert_eq!(
            PostInsertAction::DropStale,
            post_insert_action(Some(6), 5, 0)
        );
    }

    /// A whole-cache flush names no key, so it must invalidate a fetch that
    /// sampled before it even though that key has no invalidation record. This
    /// is what keeps the cache correct when invalidation messages are dropped
    /// under backpressure, or missed across a reconnection.
    #[test]
    fn a_flush_drops_an_entry_fetched_before_it_whatever_its_key_record() {
        // Sampled 5, cache flushed at generation 6 while the response was in
        // flight: nothing says this key is clean, because the flush knows no keys.
        assert_eq!(PostInsertAction::DropStale, post_insert_action(None, 5, 6));
        assert_eq!(
            PostInsertAction::DropStale,
            post_insert_action(Some(3), 5, 6)
        );
    }

    /// A fetch started after the flush is fetching post-flush data, so the flush
    /// must not condemn it — otherwise the cache could never repopulate.
    #[test]
    fn a_flush_leaves_a_later_fetch_alone() {
        assert_eq!(PostInsertAction::Keep, post_insert_action(None, 6, 6));
        assert_eq!(PostInsertAction::Keep, post_insert_action(None, 7, 6));
    }

    #[test]
    fn invalidation_at_or_before_sample_is_stale_record_pruned() {
        // A record no newer than our sample cannot have raced this fetch.
        assert_eq!(
            PostInsertAction::PruneGeneration,
            post_insert_action(Some(5), 5, 0)
        );
        assert_eq!(
            PostInsertAction::PruneGeneration,
            post_insert_action(Some(4), 5, 0)
        );
    }
}
