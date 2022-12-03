#![cfg_attr(docsrs, feature(doc_cfg))]
//! rustis is a Redis client for Rust.
//! # Philosophy
//! * Low allocations
//! * Full async library
//! * Lock free implementation
//! * Rust idiomatic API
//! 
//! # Features
//! * Support all [Redis Commands](https://redis.io/commands/) until Redis 7.0
//! * Async support ([tokio](https://tokio.rs/) or [async-std](https://async.rs/))
//! * Different client types:
//!   * Single client
//!   * [Multiplexed](https://redis.com/blog/multiplexing-explained/) client
//!   * Pooled client manager (based on [bb8](https://docs.rs/bb8/latest/bb8/))
//! * Automatic command batching
//! * [Pipelining](https://redis.io/docs/manual/pipelining/) support
//! * Configuration with Redis URL or dedicated builder
//! * [TLS](https://redis.io/docs/manual/security/encryption/) support
//! * [Transaction](https://redis.io/docs/manual/transactions/) support
//! * [Pub/sub](https://redis.io/docs/manual/pubsub/) support
//! * [Sentinel](https://redis.io/docs/manual/sentinel/) support
//! * [LUA Scripts/Functions](https://redis.io/docs/manual/programmability/) support
//! * [Cluster](https://redis.io/docs/manual/scaling/) support
//! * [Redis Stack](https://redis.io/docs/stack/) support:
//!   * [RedisJSON v2.4](https://redis.io/docs/stack/json/) support
//!   * [RedisSearch v2.6](https://redis.io/docs/stack/search/) support
//!   * [RedisGraph v2.10](https://redis.io/docs/stack/graph/) support
//!   * [RedisBloom v2.4](https://redis.io/docs/stack/bloom/) support
//!   * [RedisTimeSeries v1.8](https://redis.io/docs/stack/timeseries/) support
//! 
//! # Optional Features
//! * `tokio-runtime` - [Tokio](https://tokio.rs/) runime (default)
//! * `async-std-runtime` - [async-std](https://async.rs/) runtime (optional)
//! * `tokio-tls` - Tokio TLS support (optional)
//! * `async-std-tls`- async-std TLS support (optional)
//! * `pool` - Pooled client manager (optional)
//! * `redis-json`- [RedisJSON v2.4](https://redis.io/docs/stack/json/) support (optional)
//! * `redis-search` - [RedisSearch v2.6](https://redis.io/docs/stack/search/) support (optional)
//! * `redis-graph` - [RedisGraph v2.10](https://redis.io/docs/stack/graph/) support (optional)
//! * `redis-bloom` - [RedisBloom v2.4](https://redis.io/docs/stack/bloom/) support (optional)
//! * `redis-time-series` - [RedisTimeSeries v1.8](https://redis.io/docs/stack/timeseries/) support (optional)
//! * `redis-stack` - activate `redis-json`, `redis-search`, `redis-graph`, `redis-bloom` & `redis-time-series` at the same time (optional)
//! 
//! # Client
//! 
//! The central object in **rustis** is the client.
//! There are 3 kinds of clients.
//! 
//! ## The single client
//! The single [`Client`](crate::Client) maintains a unique connection to a Redis Server or cluster and is not thread-safe.
//! 
//! ```
//! use rustis::{
//!     Client, FlushingMode,
//!     Result, ServerCommands, StringCommands
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut client = Client::connect("127.0.0.1:6379").await?;
//!     client.flushdb(FlushingMode::Sync).await?;
//!
//!     client.set("key", "value").await?;
//!     let value: String = client.get("key").await?;
//!     println!("value: {value:?}");
//!
//!     Ok(())
//! }
//! ```
//! 
//! ## The multiplexed client
//! A [multiplexed client](crate::MultiplexedClient) can be cloned, allowing requests
//! to be be sent concurrently on the same underlying connection.
//!
//! Compared to a [single client](crate::Client), a multiplexed client cannot offers access
//! to all existing Redis commands.
//! Transactions and [blocking commands](crate::BlockingCommands) are not compatible with a multiplexed client
//! because they monopolize the whole connection which cannot be shared anymore. It means other consumers of the same
//! multiplexed client will be blocked each time a transaction or a blocking command is in progress, losing the advantage
//! of a shared connection.
//! 
//! See also [Multiplexing Explained](https://redis.com/blog/multiplexing-explained/)
//! 
//! ```
//! use rustis::{
//!     FlushingMode, MultiplexedClient,
//!     Result, ServerCommands, StringCommands
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut client1 = MultiplexedClient::connect("127.0.0.1:6379").await?;
//!     client1.flushdb(FlushingMode::Sync).await?;
//!
//!     client1.set("key", "value").await?;
//!     let value: String = client1.get("key").await?;
//!     println!("value: {value:?}");
//! 
//!     // clone a second instance on the same underlying connection
//!     let mut client2 = client1.clone();
//!     let value: String = client2.get("key").await?;
//!     println!("value: {value:?}");
//! 
//!     Ok(())
//! }
//! ```
//! 
//! ## The pooled client manager
//! The pooled client manager holds a pool of client, based on [bb8](https://docs.rs/bb8/latest/bb8/).
//! 
//! Each time a new command must be sent to the Redis Server, a client will be borrowed temporarily to the manager
//! and automatic given back to it at the end of the operation.
//! 
//! The manager can be configured via [bb8](https://docs.rs/bb8/latest/bb8/) with a various of options like maximum size, maximum lifetime, etc.
//! 
//! For you convenience, [bb8](https://docs.rs/bb8/latest/bb8/) is reexported from the **rustis** crate.
//! 
//! ```
//! use rustis::{
//!     PooledClientManager, Result, StringCommands
//! };
//! 
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let manager = PooledClientManager::new("127.0.0.1:6379")?;
//!     let pool = rustis::bb8::Pool::builder()
//!         .max_size(10)
//!         .build(manager).await?;
//! 
//!     let mut client1 = pool.get().await.unwrap();
//!     client1.set("key1", "value1").await?;
//!     let value: String = client1.get("key1").await?;
//!     println!("value: {value:?}");
//! 
//!     let mut client2 = pool.get().await.unwrap();
//!     client2.set("key2", "value2").await?;
//!     let value: String = client2.get("key2").await?;
//!     println!("value: {value:?}");
//! 
//!     Ok(())
//! }
//! ```
//! 
//! # Commands
//! 
//! In order to send [Commands](https://redis.io/commands/) to the Redis server, 
//! **rustis** offers two API levels:
//! * High-level Built-in commands that implement all [Redis 7.0](https://redis.com/blog/redis-7-generally-available/) commands + 
//!   [Redis Stack](https://redis.io/docs/stack/) commands.
//! * Low-level Generic command API to express any request that may not exist in **rustis**:
//!   * new official commands not yet implemented by **rustis**.
//!   * commands exposed by additional [Redis modules](https://redis.io/resources/modules/) 
//!     not included in [Redis Stack](https://redis.io/docs/stack/).
//! 
//! ## Built-in commands
//! 
//! Because Redis offers hundreds of commands, in **rustis** commands have been split in several traits that gather commands by groups, 
//! most of the time, groups describe in [Redis official documentation](https://redis.io/commands/).
//! 
//! Depending on the group of commands, traits will be implemented by [`Client`](crate::Client), [`MultiplexedClient`](crate::MultiplexedClient),
//! [`Pipeline`](crate::Pipeline), [`Transaction`](crate::Transaction) or some of these structs.
//! 
//! These is the list of existing command traits:
//! * [`BitmapCommands`](crate::BitmapCommands): [Bitmaps](https://redis.io/docs/data-types/bitmaps/) & [Bitfields](https://redis.io/docs/data-types/bitfields/)
//! * [`BlockingCommands`](crate::BlockingCommands): Commands that block the connection until the Redis server 
//!   has a new element to send. This trait is implemented only by the [`Client`](crate::Client) struct.
//! * [`ClusterCommands`](crate::ClusterCommands): [Redis cluster](https://redis.io/docs/reference/cluster-spec/)
//! * [`ConnectionCommands`](crate::ConnectionCommands): Connection management like authentication or RESP version management
//! * [`GenericCommands`](crate::GenericCommands): Generic commands like deleting, renaming or expiring keys
//! * [`GeoCommands`](crate::GeoCommands): [Geospatial](https://redis.io/docs/data-types/geospatial/) indices
//! * [`HashCommands`](crate::HashCommands): [Hashes](https://redis.io/docs/data-types/hashes/)
//! * [`HyperLogLogCommands`](crate::HyperLogLogCommands): [HyperLogLog](https://redis.io/docs/data-types/hyperloglogs/)
//! * [`ListCommands`](crate::ListCommands): [Lists](https://redis.io/docs/data-types/lists/)
//! * [`PubSubCommands`](crate::PubSubCommands): [Pub/Sub](https://redis.io/docs/manual/pubsub/)
//! * [`ScriptingCommands`](crate::ScriptingCommands): [Scripts](https://redis.io/docs/manual/programmability/eval-intro/) &
//!   [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
//! * [`SentinelCommands`](crate::SentinelCommands): [Sentinel](https://redis.io/docs/management/sentinel/)
//! * [`ServerCommands`](crate::ServerCommands): Server management like [Access Control Lists](https://redis.io/docs/management/security/acl/) or monitoring
//! * [`SetCommands`](crate::SetCommands): [Sets](https://redis.io/docs/data-types/sets/)
//! * [`SortedSetCommands`](crate::SortedSetCommands): [Sorted sets](https://redis.io/docs/data-types/sorted-sets/)
//! * [`StreamCommands`](crate::StreamCommands): [Streams](https://redis.io/docs/data-types/streams/)
//! * [`StringCommands`](crate::StringCommands): [Strings](https://redis.io/docs/data-types/strings/)
//! * [`TransactionCommands`](crate::TransactionCommands): [Transactions](https://redis.io/docs/manual/transactions/)
//! 
//! Redis Stack commands:
//! * [`BloomCommands`](crate::BloomCommands): [Bloom filters](https://redis.io/docs/stack/bloom/)
//! * [`CuckooCommands`](crate::CuckooCommands): [Cuckoo filters](https://redis.io/docs/stack/bloom/)
//! * [`CountMinSketchCommands`](crate::CountMinSketchCommands): [Count min-sketch](https://redis.io/docs/stack/bloom/)
//! * [`GraphCommands`](crate::GraphCommands): [RedisGraph](https://redis.io/docs/stack/graph/)
//! * [`JsonCommands`](crate::JsonCommands): [RedisJson](https://redis.io/docs/stack/json/)
//! * [`SearchCommands`](crate::SearchCommands): [`RedisSearch`](https://redis.io/docs/stack/search/)
//! * [`TDigestCommands`](crate::TDigestCommands): [`T-Digest`](https://redis.io/docs/stack/bloom/)
//! * [`TimeSeriesCommands`](crate::TimeSeriesCommands): [`Time Series`](https://redis.io/docs/stack/timeseries/)
//! * [`TopKCommands`](crate::TopKCommands): [`Top-K`](https://redis.io/docs/stack/bloom/)
//! 
//! To use a command, simple add the related trait to your use declerations 
//! and call the related method directly to a client, pipeline, transaction instance.
//! 
//! Commands can be directly awaited or [forgotten](ClientPreparedCommand::forget).
//! 
//! ```
//! use rustis::{
//!     Client, ClientPreparedCommand, Result, ListCommands, SortedSetCommands, 
//!     ZAddOptions
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut client = Client::connect("127.0.0.1:6379").await?;
//! 
//!     // Send & await ListCommands::lpush command
//!     let _size = client.lpush("mylist", ["element1", "element2"]).await?;
//! 
//!     // Send & forget SortedSetCommands::zadd command
//!     let _size = client.zadd(
//!         "mySortedSet", 
//!         [(1.0, "member1"), (2.0, "member2")], 
//!         ZAddOptions::default()
//!     ).forget();
//! 
//!     Ok(())
//! }
//! ```
//! ## Generic command API
//! To use the generic command API, you can use the [`cmd`](crate::resp::cmd) function to specify the name of the command, 
//! followed by one or multiple calls to the [`Commmand::arg`](crate::resp::Command::arg) method to add arguments to the command.
//! 
//! This command can then be passed as a parameter to one of the following methods, 
//! depending on the client, transaction or pipeline struct used:
//! * [`send`](crate::Client::send)
//! * [`send_and_forget`](crate::Client::send_and_forget)
//! * [`send_batch`](crate::Client::send_batch)

mod clients;
mod commands;
mod error;
mod network;
pub mod resp;

pub use clients::*;
pub use commands::*;
pub use error::*;
use network::*;
#[cfg(feature = "pool")]
pub use bb8;

/// Library general result type.
pub type Result<T> = std::result::Result<T, Error>;
/// Library general future type.
pub type Future<'a, T> = futures::future::BoxFuture<'a, Result<T>>;

#[cfg(all(feature = "tokio-runtime", feature = "async-std-runtime"))]
compile_error!("feature \"tokio-runtime\" and feature \"async-std-runtime\" cannot be enabled at the same time");

#[cfg(test)]
mod tests;

