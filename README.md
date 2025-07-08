An asynchronous Redis client for Rust.

[![Crate](https://img.shields.io/crates/v/rustis.svg)](https://crates.io/crates/rustis)
[![docs.rs](https://docs.rs/rustis/badge.svg)](https://docs.rs/rustis)
[![Build](https://github.com/dahomey-technologies/rustis/actions/workflows/compile_and_test.yml/badge.svg)](https://github.com/dahomey-technologies/rustis/actions/workflows/compile_and_test.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![libs.tech recommends](https://libs.tech/project/530004740/badge.svg)](https://libs.tech/project/530004740/rustis)

# Documentation

[Official Documentation](https://docs.rs/rustis/latest/rustis/)

# Philosophy

* Low allocations
* Full async library
* Lock free implementation
* Rust idiomatic API
* Multiplexing as a core feature

# Features

* Full documentation with multiple examples
* Support all [Redis Commands](https://redis.io/commands/) until Redis 8.0
* Async support ([tokio](https://tokio.rs/) or [async-std](https://async.rs/))
* Different client modes:
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
* [Cluster](https://redis.io/docs/manual/scaling/) support (minimus supported Redis version is 6)
* [Client-side caching](https://redis.io/docs/latest/develop/reference/client-side-caching/) support
* [RedisGraph v2.10](https://redis.io/docs/stack/graph/) support

# Basic Usage

```rust
use rustis::{
     client::Client, 
     commands::{FlushingMode, ServerCommands, StringCommands},
     Result,
};

#[tokio::main]
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

# Tests

1. From the `redis` directory, run `docker_up.sh` or `docker_up.cmd`
2. run `cargo test --features pool,tokio-rustls,json,client-cache` (Tokio runtime)
3. run `cargo test --no-default-features --features async-std-runtime,async-std-native-tls,json,client-cache` (async-std runtime)
4. run `cargo fmt --all -- --check`

# Benchmarks

1. From the `redis` directory, run `docker_up.sh` or `docker_up.cmd`
2. run `cargo bench`
