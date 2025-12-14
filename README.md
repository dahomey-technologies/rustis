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
* [Pipelining](https://redis.io/docs/latest/develop/using-commands/pipelining/) support
* Configuration with Redis URL or dedicated builder
* [TLS](https://redis.io/docs/latest/operate/oss_and_stack/management/security/encryption/) support
* [Transaction](https://redis.io/docs/latest/develop/using-commands/transactions/) support
* [Pub/sub](https://redis.io/docs/latest/develop/pubsub/) support
* [Sentinel](https://redis.io/docs/latest/operate/oss_and_stack/management/sentinel/) support
* [LUA Scripts/Functions](hhttps://redis.io/docs/latest/develop/programmability/) support
* [Cluster](https://redis.io/docs/latest/operate/oss_and_stack/management/scaling/) support (minimus supported Redis version is 6)
* [Client-side caching](https://redis.io/docs/latest/develop/reference/client-side-caching/) support

# Protocol Compatibility

Rustis uses the RESP3 protocol **exclusively**.

The `HELLO 3` command is automatically sent when establishing a connection.  
Therefore, your Redis server **must support RESP3** (Redis ≥6.0+ with RESP3 enabled).

If you use Redis 5 or older, or your Redis 6+ server still defaults to RESP2,  
**Rustis will not work.**

To verify your server supports RESP3:
```bash
redis-cli --raw HELLO 3
```
If you see server info (role, version, etc.), you're good to go.
If you get an error, upgrade Redis.

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
