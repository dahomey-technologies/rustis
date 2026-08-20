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
* Support all documented [Redis Commands](https://redis.io/commands/) up to and including Redis 8.8
* Async support ([tokio](https://tokio.rs/))
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
* [LUA Scripts/Functions](https://redis.io/docs/latest/develop/programmability/) support
* [Cluster](https://redis.io/docs/latest/operate/oss_and_stack/management/scaling/) support
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

# Observability

Rustis emits [`tracing`](https://docs.rs/tracing) events and spans. Install any
subscriber to see them:

```rust,ignore
tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
```

Every event from the network task is wrapped in a `connection` span carrying a
`tag` field — `host:port`, or `name:host:port` when `connection_name` is set — so
output from several clients stays attributable without any per-message prefix.
Reconnections open a nested `reconnect` span, which groups the in-flight purge,
the retries and the subscription replay into one identifiable unit. In cluster
mode, events about a specific node carry a `node` field.

**If you use `log` rather than `tracing`, you need to change nothing.** Rustis
enables tracing's `log` feature, so every event also emits a `log` record and
existing `env_logger`-style setups keep working unchanged.

Levels follow the usual convention: `error` and `warn` for conditions that need
attention, `info` for connection lifecycle, `debug` for per-command traffic, and
`trace` for the message-queue internals.

# Minimum Supported Rust Version

**Rust 1.88**, declared as `rust-version` in `Cargo.toml` and verified by a CI
job that compiles both runtimes with exactly that toolchain. Let chains hold the
floor there; edition 2024 on its own would allow 1.85.

Raising it is treated as a breaking change and is announced in `CHANGELOG.md`.

# Safety

Rustis is `#![forbid(unsafe_code)]`, and that is a deliberate position rather
than an accident of never having needed unsafe.

It costs less here than it would elsewhere. RESP is length-delimited, so the
parser reads a header and skips the announced number of bytes instead of
searching for delimiters — which leaves little for the usual payoff of unsafe in
a parser, SIMD structural scanning, to find. Hardware CRC16 for cluster slots is
the other candidate, and standalone clients skip slot computation entirely.

What it buys is that a malformed or hostile reply can never become a
memory-safety bug. The real hostile-input surface is then panics and unbounded
allocation, and both are addressed directly:

* The explicit-panic lint family (`unwrap_used`, `expect_used`, `panic`,
  `unreachable`, `todo`, `unimplemented`) is `deny` crate-wide, and
  `clippy::indexing_slicing` is `deny` in `resp/` and `network/` — the two zones
  where a panic is fatal rather than merely wrong. Surviving sites carry an
  `#[expect(…, reason = "…")]` naming the invariant that makes them unreachable —
  `expect` rather than `allow`, so a justification whose lint stops firing becomes
  a warning and is deleted instead of rotting.
* Frame size, nesting depth and element counts are bounded and configurable
  (`Config::limits`), so a crafted reply cannot drive an unbounded allocation or
  a stack overflow.
* Four `cargo-fuzz` targets exercise the frame parser, both deserializers, and
  the chunked decode path.

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
2. run `./run_tests.sh`
3. run `cargo fmt --all -- --check`

The test suite **requires `--test-threads=1`**: tests share a single Redis instance
and flush the database, so running them in parallel produces spurious failures.
That is the only thing `run_tests.sh` does beyond selecting the features —
`cargo test --features tokio-rustls,pool,json,client-cache -- --test-threads=1`.
Extra arguments are forwarded, so `./run_tests.sh string` filters by name.

## Without a server

`./run_tests.sh --hermetic` runs the half of the suite that reaches no server:
**470 tests, in about a second**, with no Docker, no deployment and no network.
It is the signal available offline, and it is a plain `cargo test` away:

```bash
cargo test --tests --no-default-features \
    --features tokio-runtime,tokio-rustls,pool,json,client-cache
```

The `server-tests` feature is what selects the two halves. It is on by default,
so `cargo test` still runs everything; turning it off compiles out every module
that needs a Redis, and what remains passes. The split is carried by the module
list in `src/tests/mod.rs`: a `*_server` module holds the server-bound tests of
the module it is named after, whose own tests stay hermetic. A test placed on the
wrong side does not go unnoticed — it fails the hermetic run.

The doctests are excluded (`--tests`): each one opens a connection.

# Benchmarks

1. From the `redis` directory, run `docker_up.sh` or `docker_up.cmd`
2. run `cargo bench --features bench`

The feature is required: every benchmark target declares
`required-features = ["bench"]`, so a plain `cargo bench` skips all of them and
reports success having measured nothing.
