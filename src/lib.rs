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
//! # Basic Usage
//! 
//! ## Client
//! 
//! The central object in **rustis** is the client.
//! There are 3 kinds of clients.
//! 
//! ### The single client
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
//! ### The multiplexed client
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
//! ### The pooled client manager
//! The pooled client manager holds a pool of client, based on [bb8](https://docs.rs/bb8/latest/bb8/).
//! 
//! Each time a new command must be sent to the Redis Server, a client will be borrowed temporarily to the manager
//! and automatic given back to it at the end of the operation.
//! 
//! The manager can be configured via [bb8](https://docs.rs/bb8/latest/bb8/) with a various of options like maximum size, maximum lifetime, etc.
//! 
//! For you convenience, [bb8](https://docs.rs/bb8/latest/bb8/) is reexport from the **rustis** crate.
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

