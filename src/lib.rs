#![cfg_attr(docsrs, feature(doc_cfg))]
/*!
rustis is a Redis client for Rust.
# Philosophy
* Low allocations
* Full async library
* Lock free implementation
* Rust idiomatic API

# Features
* Support all [Redis Commands](https://redis.io/commands/) until Redis 7.0
* Async support ([tokio](https://tokio.rs/) or [async-std](https://async.rs/))
* Different client types:
  * Single client
  * [Multiplexed](https://redis.com/blog/multiplexing-explained/) client
  * Pooled client manager (based on [bb8](https://docs.rs/bb8/latest/bb8/))
* Automatic command batching
* Advanced reconnection & retry strategy
* [Pipelining](https://redis.io/docs/manual/pipelining/) support
* Configuration with Redis URL or dedicated builder
* [TLS](https://redis.io/docs/manual/security/encryption/) support
* [Transaction](https://redis.io/docs/manual/transactions/) support
* [Pub/sub](https://redis.io/docs/manual/pubsub/) support
* [Sentinel](https://redis.io/docs/manual/sentinel/) support
* [LUA Scripts/Functions](https://redis.io/docs/manual/programmability/) support
* [Cluster](https://redis.io/docs/manual/scaling/) support
* [Redis Stack](https://redis.io/docs/stack/) support:
  * [RedisJSON v2.4](https://redis.io/docs/stack/json/) support
  * [RedisSearch v2.6](https://redis.io/docs/stack/search/) support
  * [RedisGraph v2.10](https://redis.io/docs/stack/graph/) support
  * [RedisBloom v2.4](https://redis.io/docs/stack/bloom/) support
  * [RedisTimeSeries v1.8](https://redis.io/docs/stack/timeseries/) support

# Optional Features
| Feature | Description |
| ------- | ----------- |
| `tokio-runtime` | [Tokio](https://tokio.rs/) runime (default) |
| `async-std-runtime` | [async-std](https://async.rs/) runtime (optional) |
| `tokio-tls` | Tokio TLS support (optional) |
| `async-std-tls` | async-std TLS support (optional) |
| `pool` | Pooled client manager (optional) |
| `redis-json` | [RedisJSON v2.4](https://redis.io/docs/stack/json/) support (optional) |
| `redis-search` | [RedisSearch v2.6](https://redis.io/docs/stack/search/) support (optional) |
| `redis-graph` | [RedisGraph v2.10](https://redis.io/docs/stack/graph/) support (optional) |
| `redis-bloom` | [RedisBloom v2.4](https://redis.io/docs/stack/bloom/) support (optional) |
| `redis-time-series` | [RedisTimeSeries v1.8](https://redis.io/docs/stack/timeseries/) support (optional) |
| `redis-stack` | activate `redis-json`, `redis-search`, `redis-graph`, `redis-bloom` & `redis-time-series` at the same time (optional) |

# Basic Usage

```
use rustis::{
    client::Client, 
    commands::{FlushingMode, ServerCommands, StringCommands},
    Result,
};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    // sends the command SET to Redis. This command is defined in the StringCommands trait
    client.set("key", "value").await?;

    // sends the command GET to Redis. This command is defined in the StringCommands trait
    let value: String = client.get("key").await?;
    println!("value: {value:?}");

    Ok(())
}
```

# Client
See the module [`client`] to discover which are the 3 
usages of the [`Client`](client::Client) struct and how to configure it.

You will also learn how to use pipeline, pub/sub and transactions.

# RESP
RESP is the [Redis Serialization Protocol](https://redis.io/docs/reference/protocol-spec/).

See the module [`resp`] to discover how **rustis** 
allows programmers to communicate with Redis in a Rust idiomatic way.

You will learn how to:
* Manipulate the **rustis** object model, the enum [`Value`](resp::Value), which is a generic Rust data structure over RESP.
* Convert Rust type into Rust Commands with the [`Command`](resp::Command) struct and the [`ToArgs`](resp::ToArgs) trait.
* Convert Rust command responses into Rust type with serde and helpful marker traits.

# Commands
In order to send [Commands](https://redis.io/commands/) to the Redis server,
**rustis** offers two API levels:
* High-level Built-in commands that implement all [Redis 7.0](https://redis.com/blog/redis-7-generally-available/) commands +
  [Redis Stack](https://redis.io/docs/stack/) commands.
* Low-level Generic command API to express any request that may not exist in **rustis**:
  * new official commands not yet implemented by **rustis**.
  * commands exposed by additional [Redis modules](https://redis.io/resources/modules/)
    not included in [Redis Stack](https://redis.io/docs/stack/).

## Built-in commands
See the module [`commands`] to discover how Redis built-in commands are organized in different traits.

## Generic command API
To use the generic command API, you can use the [`cmd`](crate::resp::cmd) function to specify the name of the command,
followed by one or multiple calls to the [`Commmand::arg`](crate::resp::Command::arg) associated function to add arguments to the command.

This command can then be passed as a parameter to one of the following associated functions,
depending on the client, transaction or pipeline struct used:
* [`send`](crate::client::Client::send)
* [`send_and_forget`](crate::client::Client::send_and_forget)
* [`send_batch`](crate::client::Client::send_batch)

```
use rustis::{client::Client, resp::cmd, Result};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    client
        .send(
            cmd("MSET")
                .arg("key1")
                .arg("value1")
                .arg("key2")
                .arg("value2")
                .arg("key3")
                .arg("value3")
                .arg("key4")
                .arg("value4"),
            None,
        )
        .await?
        .to::<()>()?;

    let values: Vec<String> = client
        .send(
            cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"),
            None,
        )
        .await?
        .to()?;

    assert_eq!(vec!["value1".to_owned(), "value2".to_owned(), "value3".to_owned(), "value4".to_owned()], values);

    Ok(())
}
```
*/

pub mod client;
pub mod commands;
mod error;
mod network;
pub mod resp;

#[cfg(feature = "pool")]
pub use bb8;
pub use error::*;
use network::*;

/// Library general result type.
pub type Result<T> = std::result::Result<T, Error>;
/// Library general future type.
pub type Future<'a, T> = futures_util::future::BoxFuture<'a, Result<T>>;

#[cfg(all(feature = "tokio-runtime", feature = "async-std-runtime"))]
compile_error!("feature \"tokio-runtime\" and feature \"async-std-runtime\" cannot be enabled at the same time");

#[cfg(all(feature = "pool", feature = "async-std-runtime"))]
compile_error!("feature \"pool\" is only compatible with \"tokio-runtime\" (bb8 constraint)");

#[cfg(test)]
mod tests;
