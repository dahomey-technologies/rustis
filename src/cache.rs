//! [Client-side caching](https://redis.io/docs/latest/develop/reference/client-side-caching/) support
use crate::{
    Error, Result,
    client::{Client, PreparedCommand},
    commands::{
        BitFieldSubCommand, BitRange, BitmapCommands, ClientTrackingOptions, ClientTrackingStatus,
        ConnectionCommands, HashCommands, ListCommands, SetCommands, SortedSetCommands,
        StringCommands, ZRangeOptions,
    },
    resp::{
        BulkString, Command, CommandArgsMut, RespBuf, RespDeserializer, RespSerializer, Response,
        Value, cmd,
    },
};
use bytes::BytesMut;
use dashmap::DashMap;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{fmt::Write, sync::Arc, time::Duration};

/// Re-export the moka cache builder.
pub use moka::future::CacheBuilder;

type SubCache = DashMap<Command, RespBuf>;
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
/// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
/// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
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
    #[allow(dead_code)]
    invalidation_task: tokio::task::JoinHandle<()>,
}

impl Cache {
    /// Create cache from a moka CacheBuilder and activates Redis client tracking invalidations
    #[allow(clippy::type_complexity)]
    pub async fn from_builder(
        client: Client,
        builder: MokaCacheBuilder,
        tracking_opts: ClientTrackingOptions,
    ) -> Result<Arc<Self>> {
        client
            .client_tracking(ClientTrackingStatus::On, tracking_opts)
            .await?;

        let stream = client.create_client_tracking_invalidation_stream()?;

        let cache = Arc::new(builder.build());
        let cache_clone = cache.clone();

        let connection_tag = client.connection_tag().to_owned();
        let invalidation_task = tokio::spawn(async move {
            let mut stream = stream;
            while let Some(keys) = stream.next().await {
                for key in keys {
                    log::debug!(
                        "[{}] Invalidating key `{key}` from client cache",
                        connection_tag
                    );
                    let key: BulkString = key.into_bytes().into();
                    cache_clone.invalidate(&key).await;
                }
            }
        });

        Ok(Arc::new(Self {
            cache,
            client,
            invalidation_task,
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

    /// Executes the `GET` command with client-side caching.
    pub async fn get<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.get(key))
            .await
    }

    /// Executes the `MGET` command with client-side caching.
    pub async fn mget<R: Response + DeserializeOwned>(&self, keys: impl Serialize) -> Result<R> {
        let prepared_command = self.client.mget::<R>(keys);
        let mut collection_buf = BytesMut::new();
        let _ =
            collection_buf.write_fmt(format_args!("*{}\r\n", prepared_command.command.num_args()));

        for arg in (0..prepared_command.command.num_args())
            .filter_map(|i| prepared_command.command.get_arg(i))
        {
            let key = BulkString::from(arg.to_vec());

            let Some(values) = self.cache.get(&key).await else {
                collection_buf.clear();
                break;
            };

            let prepared_command = self.client.get::<R>(arg);
            let Some(buf) = values.get(&prepared_command.command) else {
                collection_buf.clear();
                break;
            };

            collection_buf.extend(buf.iter());
        }

        if !collection_buf.is_empty() {
            log::debug!("[{}] Cache hit on mget", self.client.connection_tag(),);

            let mut deserializer = RespDeserializer::new(&collection_buf);
            return R::deserialize(&mut deserializer);
        }

        let buf = self
            .client
            .send(prepared_command.command.clone(), None)
            .await?;
        let mut deserializer = RespDeserializer::new(&buf);
        let Value::Array(values) = Value::deserialize(&mut deserializer)? else {
            return Err(Error::Client(
                "Expected array result for MGET command".to_string(),
            ));
        };

        for (value, key) in values.iter().zip(
            (0..prepared_command.command.num_args())
                .filter_map(|i| prepared_command.command.get_arg(i)),
        ) {
            let mut serializer = RespSerializer::new();
            value.serialize(&mut serializer)?;

            // Insert into cache
            self.cache
                .entry(key.to_vec().into())
                .or_insert_with(async { Arc::new(DashMap::new()) })
                .await
                .value()
                .insert(
                    cmd("GET").arg(key).into(),
                    RespBuf::new(serializer.get_output().into()),
                );
        }

        R::deserialize(&Value::Array(values))
    }

    /// Executes the `GETRANGE` command with client-side caching.
    pub async fn getrange<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        start: isize,
        end: isize,
    ) -> Result<R> {
        self.process_prepared_command(
            key_to_bulk_string(&key),
            self.client.getrange(key, start, end),
        )
        .await
    }

    /// Executes the `STRLEN` command with client-side caching.
    pub async fn strlen(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.strlen(key))
            .await
    }

    /// Executes the `HEXISTS` command with client-side caching.
    pub async fn hexists(&self, key: impl Serialize, field: impl Serialize) -> Result<bool> {
        self.process_prepared_command(
            key_to_bulk_string(&key),
            self.client.hexists(key_to_bulk_string(&key), field),
        )
        .await
    }

    /// Executes the `HGET` command with client-side caching.
    pub async fn hget<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        field: impl Serialize,
    ) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hget(key, field))
            .await
    }

    /// Executes the `HGETALL` command with client-side caching.
    pub async fn hgetall<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hgetall(key))
            .await
    }

    /// Executes the `HLEN` command with client-side caching.
    pub async fn hlen(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hlen(key))
            .await
    }

    /// Executes the `HKEYS` command with client-side caching.
    pub async fn hkeys<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hkeys(key))
            .await
    }

    /// Executes the `HKEYS` command with client-side caching.
    pub async fn hvals<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hvals(key))
            .await
    }

    /// Executes the `HSTRLEN` command with client-side caching.
    pub async fn hstrlen(&self, key: impl Serialize, field: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hstrlen(key, field))
            .await
    }

    /// Executes the `HMGET` command with client-side caching.
    pub async fn hmget<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.hmget(key, fields))
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
            key_to_bulk_string(&key),
            self.client.lrange(key, start, stop),
        )
        .await
    }

    /// Executes the `LLEN` command with client-side caching.
    pub async fn llen(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.llen(key))
            .await
    }

    /// Executes the `LINDEX` command with client-side caching.
    pub async fn lindex<R: Response + DeserializeOwned>(
        &self,
        key: impl Serialize,
        index: isize,
    ) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.lindex(key, index))
            .await
    }

    /// Executes the `SMEMBERS` command with client-side caching.
    pub async fn smembers<R: Response + DeserializeOwned>(&self, key: impl Serialize) -> Result<R> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.smembers(key))
            .await
    }

    /// Executes the `SCARD` command with client-side caching.
    pub async fn scard(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.scard(key))
            .await
    }

    /// Executes the `SISMEMBER` command with client-side caching.
    pub async fn sismember(&self, key: impl Serialize, member: impl Serialize) -> Result<bool> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.sismember(key, member))
            .await
    }

    /// Executes the `ZCARD` command with client-side caching.
    pub async fn zcard(&self, key: impl Serialize) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.zcard(key))
            .await
    }

    /// Executes the `ZCOUNT` command with client-side caching.
    pub async fn zcount(
        &self,
        key: impl Serialize,
        min: impl Serialize,
        max: impl Serialize,
    ) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.zcount(key, min, max))
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
            key_to_bulk_string(&key),
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
            key_to_bulk_string(&key),
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
        self.process_prepared_command(key_to_bulk_string(&key), self.client.zrank(key, member))
            .await
    }

    /// Executes the `ZREMRANGEBYSCORE` command with client-side caching.
    pub async fn zremrangebyscore(
        &self,
        key: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
    ) -> Result<usize> {
        self.process_prepared_command(
            key_to_bulk_string(&key),
            self.client.zremrangebyscore(key, start, stop),
        )
        .await
    }

    /// Executes the `ZREVRANK` command with client-side caching.
    pub async fn zrevrank(
        &self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> Result<Option<usize>> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.zrevrank(key, member))
            .await
    }

    /// Executes the `ZSCORE` command with client-side caching.
    pub async fn zscore(&self, key: impl Serialize, member: impl Serialize) -> Result<Option<f64>> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.zscore(key, member))
            .await
    }

    /// Executes the `BITCOUNT` command with client-side caching.
    pub async fn bitcount(&self, key: impl Serialize, range: BitRange) -> Result<usize> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.bitcount(key, range))
            .await
    }

    /// Executes the `BITPOS` command with client-side caching.
    pub async fn bitpos(&self, key: impl Serialize, bit: u64, range: BitRange) -> Result<usize> {
        self.process_prepared_command(
            key_to_bulk_string(&key),
            self.client.bitpos(key, bit, range),
        )
        .await
    }

    /// Executes the `GETBIT` command with client-side caching.
    pub async fn getbit(&self, key: impl Serialize, offset: u64) -> Result<u64> {
        self.process_prepared_command(key_to_bulk_string(&key), self.client.getbit(key, offset))
            .await
    }

    /// Executes the `BITFIELD_RO` command with client-side caching.
    pub async fn bitfield_readonly<'a>(
        &self,
        key: impl Serialize,
        sub_commands: impl IntoIterator<Item = BitFieldSubCommand<'a>> + Serialize,
    ) -> Result<Vec<u64>> {
        self.process_prepared_command(
            key_to_bulk_string(&key),
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
            && let Some(buf) = values.get(&command)
        {
            log::debug!(
                "[{}] Cache hit on key `{}`",
                self.client.connection_tag(),
                key
            );
            let mut deserializer = RespDeserializer::new(&buf);
            return R::deserialize(&mut deserializer);
        }

        // Cache miss: fetch from Redis
        log::debug!(
            "[{}] Cache miss on key `{}`",
            self.client.connection_tag(),
            key
        );

        let buf = self.client.send(command.clone(), None).await?;
        let mut deserializer = RespDeserializer::new(&buf);
        let deserialized = R::deserialize(&mut deserializer)?;

        // Insert into cache
        self.cache
            .entry(key)
            .or_insert_with(async { Arc::new(DashMap::new()) })
            .await
            .value()
            .insert(command, buf);

        Ok(deserialized)
    }
}

fn key_to_bulk_string(key: &impl Serialize) -> BulkString {
    let args = CommandArgsMut::default().arg(key).freeze();
    args.into_iter()
        .next()
        .expect("expected a single argument")
        .into()
}
