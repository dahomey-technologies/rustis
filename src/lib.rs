#![forbid(unsafe_code)]
// Panic policy — the achievable analogue of the line above.
//
// No `forbid(panics)` can exist: a panic has no single syntactic form. It
// escapes from `unwrap`/`expect`/`panic!`/`unreachable!` *and* from indexing,
// integer arithmetic and dozens of std methods. What is enforceable is three
// families of lint, and all three are denied:
//
// * the explicit-panic family, crate-wide here;
// * `arithmetic_side_effects`, crate-wide here — every `+ - * / %` on integers
//   that the compiler cannot prove safe. It covers the failure mode that has
//   produced the most defects in this crate: an announced length or a counter
//   that overflows, which panics in debug builds and *wraps silently* in release
//   ones, turning a hostile length into a plausible offset;
// * `indexing_slicing` — the only lint covering `a[i]` / `a[i..j]`, the crate's
//   largest panic surface by count — in the two zones where a panic is fatal
//   rather than merely wrong: `network/` (a panic in the network task kills the
//   client with no reconnect) and `resp/` (fed directly by server bytes). See
//   their `mod.rs`.
//
// This is deny-plus-justified-allow, not a blanket ban: not every panic is a
// bug. An `unreachable!` on an exhaustive internal match, an index guarded by
// the compare on the line above, and stepping a slice offset past a byte that
// was just read are correct code, and rewriting them into `.get().unwrap()` or
// `checked_add` would trade clarity — and, in the parser, throughput — for
// nothing. Every surviving site therefore carries `#[expect(…, reason = "…")]`
// naming the invariant that makes it unreachable — the same contract a
// `// SAFETY:` comment carries over an `unsafe` block, and reviewable the same
// way. `expect` rather than `allow`, so a justification whose lint stops firing
// becomes a warning and is deleted instead of rotting.
//
// `warn` would have been indistinguishable from `deny` here: CI runs clippy
// with `-D warnings`, so the real tiers are enforced and exempt. Test code is
// exempt (`#![allow]` at the top of each test module): a test that panics is a
// test that failed, which is the mechanism, not a defect.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::arithmetic_side_effects
)]
// `pub` on an item the outside world cannot reach is a lie the compiler does not
// otherwise report: it suppresses `dead_code`, and a reader — or a CHANGELOG
// entry — takes it for public API. This lint covers the method-level half of the
// problem. It does not fire on a type re-exported by a `pub(crate) use` glob, so
// the module boundary in `resp/mod.rs` still has to be read, not trusted.
#![warn(unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
/*!
rustis is a Redis client for Rust.
# Philosophy
* Low allocations
* Full async library
* Lock free implementation
* Rust idiomatic API

# Features
* Support all documented [Redis Commands](https://redis.io/commands/) up to and including Redis 8.8
* Async support ([tokio](https://tokio.rs/))
* Different client types:
  * Single client
  * [Multiplexed](https://redis.com/blog/multiplexing-explained/) client
  * Pooled client manager (based on [bb8](https://docs.rs/bb8/latest/bb8/))
* Automatic command batching
* Advanced reconnection & retry strategy
* [Pipelining](https://redis.io/docs/manual/pipelining/) support
* Configuration with Redis URL or dedicated builder
* [TLS](https://redis.io/docs/latest/operate/oss_and_stack/management/security/encryption/) support
* [Transaction](https://redis.io/docs/manual/transactions/) support
* [Pub/sub](https://redis.io/docs/manual/pubsub/) support
* [Sentinel](https://redis.io/docs/manual/sentinel/) support
* [LUA Scripts/Functions](https://redis.io/docs/manual/programmability/) support
* [Cluster](https://redis.io/docs/manual/scaling/) support
* [Client-side caching](https://redis.io/docs/latest/develop/reference/client-side-caching/) support

# Optional Features
| Feature | Description |
| ------- | ----------- |
| `tokio-runtime` | [Tokio](https://tokio.rs/) runtime (default) |
| `tokio-rustls` | Tokio Rustls TLS support |
| `tokio-native-tls` | Tokio native_tls TLS support |
| `json` | Enables JSON (de)serialization support via `serde_json` |
| `client-cache` | Enables client-side caching support |
| `pool` | Pooled client manager |

`tokio-rustls` and `tokio-native-tls` are **mutually exclusive**: enabling both is a
compile error. Each implies the corresponding backend-only feature (`rustls`,
`native-tls`), which gates the TLS configuration types. Enabling a backend-only
feature on its own is also a compile error: it brings the configuration types
without the connection code that honours them.

`pool` puts bb8 in rustis' public API: [`PooledClientManager`](client::PooledClientManager)
implements `bb8::ManageConnection` and the crate is re-exported as [`bb8`]. A bb8 major
release is therefore a breaking rustis release even when no rustis code changes, and two
crates in one dependency graph cannot disagree about the bb8 version. That is the price of
configuring the pool with bb8's own builder rather than through a wrapper.

The remaining features are for developing rustis itself and carry **no stability
guarantee**: `bench` (exposes internal RESP entry points to the benchmark crates, as
`resp::bench_support`), `fuzzing` (same, for the `cargo-fuzz` targets in `fuzz/`) and
`web-examples` (the `axum` / `actix-web` examples). None of the three carries a
dependency: the crates they need are dev dependencies, so a dependent that enables one
still builds nothing extra.

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

```
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
* Convert Rust types into Rust Commands with the [`Command`](resp::Command) struct, whose
  arguments are any type implementing serde's [`Serialize`](serde::Serialize).
* Convert Rust command responses into Rust type with serde and helpful marker traits.

# Commands
In order to send [Commands](https://redis.io/commands/) to the Redis server,
**rustis** offers two API levels:
* High-level Built-in commands that implement all documented Redis commands up to and
  including Redis 8.8, plus the [Redis Stack](https://redis.io/docs/stack/) commands.
* Low-level Generic command API to express any request that may not exist in **rustis**:
  * new official commands not yet implemented by **rustis**.
  * commands exposed by additional [Redis modules](https://redis.io/resources/modules/)
    not included in [Redis Stack](https://redis.io/docs/stack/).

## Built-in commands
See the module [`commands`] to discover how Redis built-in commands are organized in different traits.

## Generic command API
To use the generic command API, you can use the [`cmd`](crate::resp::cmd) function to specify the name of the command,
followed by one or multiple calls to [`CommandBuilder::arg`](crate::resp::CommandBuilder::arg) to add arguments to the command,
and to [`CommandBuilder::key`](crate::resp::CommandBuilder::key) to add arguments that are Redis keys.

This command can then be passed as a parameter to one of the following associated functions,
depending on the client, transaction or pipeline struct used:
* [`send`](crate::client::Client::send)
* [`send_and_forget`](crate::client::Client::send_and_forget)
* [`Pipeline::queue_command`](crate::client::Pipeline::queue_command), to batch several of them

```
use rustis::{client::Client, resp::cmd, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    client
        .send::<()>(
            cmd("MSET")
                .key("{my}key1")
                .arg("value1")
                .key("{my}key2")
                .arg("value2")
                .key("{my}key3")
                .arg("value3")
                .key("{my}key4")
                .arg("value4"),
            None,
        )
        .await?;

    let values: Vec<String> = client
        .send(
            cmd("MGET")
                .key("{my}key1")
                .key("{my}key2")
                .key("{my}key3")
                .key("{my}key4"),
            None,
        )
        .await?;

    assert_eq!(vec!["value1", "value2", "value3", "value4"], values);

    Ok(())
}
```

## Warning: keys must be added with `key`, not `arg`
Only arguments added with [`key`](crate::resp::CommandBuilder::key) take part in Cluster slot
computation. A command built with `arg` alone carries no slot and is sent to a **random node**
of the cluster, with no error to tell you: a single-key command gets a `MOVED` reply, and the
retry that follows the topology refresh picks a random node again. A multi-key command such as
`MSET` fails with `CROSSSLOT`.

A multi-key command additionally requires all its keys to hash to the same slot, which is what
the `{my}` hash tag guarantees in the example above.

This does not apply to the strongly typed command API ([`commands`]): those functions already
mark their keys.

## Adding a command family of your own
The generic API sends anything, but it costs the fluent shape. Add a missing command on the
same footing as the built-in ones with [`prepare_command`](crate::client::prepare_command):

```
use rustis::{
    client::{Client, PreparedCommand, prepare_command},
    resp::cmd,
};
use serde::Serialize;

trait MyCommands<'a> {
    #[must_use]
    fn myget(self, key: impl Serialize) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("MYGET").key(key))
    }
}

impl<'a> MyCommands<'a> for &'a Client {}
```

Every trait in [`commands`] is written this way. Implement it for
[`Pipeline`](crate::client::Pipeline) and [`Transaction`](crate::client::Transaction) too to
queue the command into a batch.

# Warning: raw bytes need an adapter type
`client.set("key", b"val")` compiles and fails **at runtime**: serde serializes `&[u8]` and
`Vec<u8>` as sequences of integers, not as one bulk string. Wrap them in
[`RefBulkString`](crate::resp::RefBulkString) or [`BulkString`](crate::resp::BulkString). See the
[`resp`] module page for the reason.

# Errors
Every fallible call returns [`Result<T>`](crate::Result), whose error is [`Error`].
An `Error` is what went wrong, [`kind()`](Error::kind), plus the command it belongs
to, [`command()`](Error::command):

```
use rustis::{Error, ErrorKind, Result};

fn report(result: Result<String>) {
    if let Err(e) = result {
        match e.kind() {
            ErrorKind::Timeout(_) => eprintln!("{:?} timed out", e.command()),
            ErrorKind::Redis(redis_error) => eprintln!("the server refused it: {redis_error}"),
            _ => eprintln!("{e}"),
        }
    }
}
```

The command matters because a client multiplexes: a single connection carries
hundreds of commands at once, so a bare "the operation timed out" names nothing
the application can act on. It is set for every error the client raises on behalf
of a command, and absent for the ones raised outside any — a connection timeout,
for instance. `Display` appends it, so a logged error reads
`The I/O operation's timeout expired (while executing BLMPOP)`.

# Client-side caching
See the module [`cache`] to discover how you can implement client-side caching.
*/

#[cfg(feature = "client-cache")]
pub mod cache;
#[cfg(feature = "client-cache")]
mod cache_store;
pub mod client;
pub mod commands;
mod error;
#[cfg(feature = "fuzzing")]
pub mod fuzz_api;
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

// Every function of `network::async_executor_strategy` is provided by a runtime
// feature and has no fallback body. Without this guard the user gets a dozen
// "not found in this scope" errors instead of the actual cause.
#[cfg(not(feature = "tokio-runtime"))]
compile_error!(
    "rustis needs an async runtime feature. Enable `tokio-runtime`, which is the default feature: \
     with `default-features = false` you must list it explicitly, as in \
     `rustis = { version = \"...\", default-features = false, features = [\"tokio-runtime\"] }`."
);

#[cfg(all(feature = "tokio-native-tls", feature = "tokio-rustls"))]
compile_error!(
    "Features `tokio-native-tls` and `tokio-rustls` cannot be enabled at the same time."
);

// The two backends each define their own `TlsConfig` and their own `Tls` error
// variant, with different fields, so the union defines both names twice. Feature
// unification reaches it without anyone asking: two crates in one dependency
// graph, each enabling one backend. The pair is named here rather than left to
// the two guards below, which would report it as two independent mistakes.
//
// The cascade behind this message is not suppressed. Doing so would mean giving
// one backend precedence over the other under `cfg` — a silent winner, and a
// `not(feature = "rustls")` clause to remember on every `native-tls` item ever
// added. The guard names the cause; the errors after it are its consequences.
#[cfg(all(
    feature = "rustls",
    feature = "native-tls",
    // The two runtime features imply both backends, and the guard above already
    // names that pair. Every rejected configuration reports exactly one cause.
    not(all(feature = "tokio-rustls", feature = "tokio-native-tls"))
))]
compile_error!(
    "Features `rustls` and `native-tls` cannot be enabled at the same time: each defines its own \
     `TlsConfig` and its own `Error::Tls`, with different fields. Pick one backend — \
     `tokio-rustls` or `tokio-native-tls`. If a dependency enabled the other one, the two were \
     unified into this build."
);

// The backend-only features gate the TLS configuration types; the connection
// code that reads them lives behind the runtime feature. Enabled alone they
// build a `TlsConfig` nothing would ever use, so name that rather than let the
// missing stream types surface as "not found in this scope".
//
// Both carry `not(<the other backend>)` so the pair above is reported once,
// by the guard that diagnoses it, instead of three times.
#[cfg(all(
    feature = "rustls",
    not(feature = "tokio-rustls"),
    not(feature = "native-tls")
))]
compile_error!(
    "Feature `rustls` cannot be enabled on its own: it only gates the TLS configuration types. \
     Enable `tokio-rustls`, which implies it and brings the TLS connection code."
);

#[cfg(all(
    feature = "native-tls",
    not(feature = "tokio-native-tls"),
    not(feature = "rustls")
))]
compile_error!(
    "Feature `native-tls` cannot be enabled on its own: it only gates the TLS configuration types. \
     Enable `tokio-native-tls`, which implies it and brings the TLS connection code."
);

#[cfg(test)]
mod tests;
