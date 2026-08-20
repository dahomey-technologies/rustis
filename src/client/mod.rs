/*!
Defines types related to the clients structs and their dependencies:
[`Client`], [`ExclusiveClient`], [`PooledClientManager`], [`Pipeline`], [`Transaction`]
and how to configure them

# Clients

The central object in **rustis** is the [`Client`].

It will allow you to connect to the Redis server, to send command requests
and to receive command responses and push messages.

There are 3 ways to use a connection
* As a single client
* As a multiplexer
* In a pool of clients

Two of them share the connection, and a shared connection cannot run the
commands that hold it: those live on [`ExclusiveClient`], the client that owns
its connection — see [Exclusive commands](#exclusive-commands).

## The single client
The single [`Client`] maintains a unique connection to a Redis Server or cluster.

This use case of the client is not meant to be used directly in a Web application, where multiple HTTP connections access
the Redis server at the same time in a multi-threaded architecture (like [Actix](https://actix.rs/) or [Rocket](https://rocket.rs/)).

It could be used in tools where the load is minimal.

```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    println!("value: {value:?}");

    Ok(())
}
```

## The multiplexer
A [`Client`] instance can be cloned, allowing multiple requests
to be sent concurrently on the same underlying connection.

Multiplexer mode is highly efficient in multi-threaded architectures because it uses only a single
underlying connection. It is the prefered mode for most Web applications.

### Managing Multiplexed Subscriptions
Because **rustis** implements the RESP3 protocol, there is no limitation when using subscriptions on a multiplexed connection.
Pub/Sub messages and regular command responses are cleanly distinguished at the protocol level,
allowing both to coexist safely on the same shared connection.

### Limitations
Blocking commands and [`watch`](crate::commands::TransactionCommands::watch) hold or
attach state to the connection, so they are not available on a multiplexed [`Client`] —
see [Exclusive commands](#exclusive-commands).

## The pooled client manager
The pooled client manager holds a pool of [`Client`]s, based on [bb8](https://docs.rs/bb8/latest/bb8/).

Each time a new command must be sent to the Redis Server, a client will be borrowed temporarily to the manager
and automatically given back to it at the end of the operation.

It is an alternative to multiplexing, for managing **rustis** within a Web application.

The manager can be configured via [bb8](https://docs.rs/bb8/latest/bb8/) with a various of options like maximum size, maximum lifetime, etc.

For you convenience, [bb8](https://docs.rs/bb8/latest/bb8/) is reexported from the **rustis** crate.

```
#[cfg(feature = "pool")]
use rustis::{
    client::PooledClientManager, commands::StringCommands,
};
use rustis::Result;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "pool")] {
        let manager = PooledClientManager::new("127.0.0.1:6379")?;
        let pool = rustis::bb8::Pool::builder()
            .max_size(10)
            .build(manager).await?;

        let client1 = pool.get().await.unwrap();
        client1.set("key1", "value1").await?;
        let value: String = client1.get("key1").await?;
        println!("value: {value:?}");

        let client2 = pool.get().await.unwrap();
        client2.set("key2", "value2").await?;
        let value: String = client2.get("key2").await?;
        println!("value: {value:?}");
        }

    Ok(())
}
```

# Exclusive commands

Two families of commands are incompatible with a shared connection:

* [blocking commands](crate::commands::BlockingCommands), which monopolize the
  connection until they return — every other caller queued behind a `BLPOP` with
  a 30-second timeout waits 30 seconds;
* [`watch`](crate::commands::TransactionCommands::watch), whose watched state
  applies to the connection itself, not to the handle that issued it, so any
  clone can invalidate or discard it.

They are therefore not implemented for [`Client`], which is clonable, but for
[`ExclusiveClient`], which is not. Reaching for one of them on a multiplexed
client is a compile error rather than a stalled connection at run time.

```
use rustis::{
    client::{Client, ExclusiveClient},
    commands::{BlockingCommands, TransactionCommands},
    Result,
};

async fn example() -> Result<()> {
    // A connection of its own …
    let client = ExclusiveClient::connect("127.0.0.1:6379").await?;

    // … or an existing client, which the conversion refuses while another
    // handle on the connection is alive.
    let client = Client::connect("127.0.0.1:6379").await?.into_exclusive()?;

    let result: Option<(String, String)> = client.blpop("key", 30.).await?;
    client.watch("key").await?;

    Ok(())
}
```

A [`PooledClientManager`] hands out an [`ExclusiveClient`] for the same reason:
a borrowed connection returns to the pool only once the command completes, so
the block stays confined to it.

No other command is refused. Pub/sub is fine on a multiplexed connection —
RESP3 keeps messages and command replies apart — and so are `MULTI`/`EXEC`
transactions, which [`Client::create_transaction`] still opens: of the two
families above, only `WATCH` attaches state to the connection. A handful of
commands do attach state without being incompatible with sharing; they are the
subject of the next section.

# Connection-scoped commands

Some commands configure the connection instead of acting on data: `select`,
`auth`, `client_setname`, `client_setinfo`, `client_no_evict`, `client_no_touch`,
`client_tracking`, `client_reply` and `script_debug`.

A [`Client`] is clonable, and one connection carries the commands of every clone.
These commands therefore apply to every clone: `client.select(5)` moves the
commands of all clones to database 5.

The client records each of them and replays it after a reconnection, so the state
survives a broken socket. It cannot scope the state to one clone, because the
connection is the scope.

Set the database with [`Config::database`](crate::client::Config::database).
Set the credentials with [`Config::username`](crate::client::Config::username),
[`Config::password`](crate::client::Config::password) or
[`Config::credentials_provider`](crate::client::Config::credentials_provider). The
handshake sends them on every connection. An [`ExclusiveClient`] owns its
connection, which scopes these commands to the caller.

# Configuration

A [`Client`] instance can be configured with the [`Config`] struct:
* Authentication
* [`TlsConfig`]
* [`ServerConfig`] (Standalone, Sentinel or Cluster)

[`IntoConfig`] is a convenient trait to convert more known types to a [`Config`] instance:
* &[`str`](https://doc.rust-lang.org/std/primitive.str.html): host and port separated by a colon
* `(impl Into<String>, u16)`: a pair of host and port
* [`String`](https://doc.rust-lang.org/alloc/string/struct.String.html): host and port separated by a colon
* [`Url`](https://docs.rs/url/latest/url/struct.Url.html): see Url syntax below.

## Url Syntax

The **rustis** [`Config`] can also be built from an URL

### Standalone

```text
redis|rediss://[[<username>]:<password>@]<host>[:<port>][/<database>]
```

### Cluster

```text
redis|rediss[+cluster]://[[<username>]:<password>@]<host1>[:<port1>][,<host2>:[<port2>][,<hostN>:[<portN>]]]
```

### Sentinel

```text
redis|rediss[+sentinel]://[[<username>]:<password>@]<host>[:<port>]/<service>[/<database>]
                          [?wait_between_failures=<250>[&sentinel_username=<username>][&sentinel_password=<password>]]
```

`service` is the required name of the sentinel service

### Unix socket

```text
unix://<path>[?db=<database>]
```

`path` is the absolute path of the socket the server listens on. It is the whole path of the
URI, so the database is the `db` query parameter here rather than the last path segment, and
there is no authority to carry credentials: set [`Config::username`] / [`Config::password`] on
the config. [`keep_alive`](Config::keep_alive) and [`no_delay`](Config::no_delay) describe a TCP
socket and are not applied.

### Schemes
The URL scheme is used to detect the server type:
* `redis://` - Non secure TCP connection to a standalone Redis server
* `rediss://` - Secure (TSL) TCP connection to a standalone Redis server
* `redis+sentinel://` or `redis-sentinel://` - Non secure TCP connection to a Redis sentinel network
* `rediss+sentinel://` or `rediss-sentinel://` - Secure (TSL) TCP connection to a Redis sentinel network
* `redis+cluster://` or `redis-cluster://` - Non secure TCP connection to a Redis cluster
* `rediss+cluster://` or `rediss-cluster://` - Secure (TSL) TCP connection to a Redis cluster
* `unix://`, `redis+unix://` or `redis-unix://` - Connection to a standalone Redis server
  listening on a Unix domain socket

### QueryParameters
Query parameters set optional configuration fields of the struct [`Config`] or its
dependencies. The list below is exhaustive: an unknown parameter, or a value that
does not parse, is rejected with an error rather than ignored.
* [`connect_timeout`](Config::connect_timeout) - The time to attempt a connection before timing out (default `10,000` ms).
* [`command_timeout`](Config::command_timeout) - If a command does not return a reply within a set number of milliseconds,
  a timeout error will be thrown. If set to 0, no timeout is apply (default `0`).
* [`auto_resubscribe`](Config::auto_resubscribe) - When the client reconnects, channels subscribed in the previous connection will be
  resubscribed automatically if `auto_resubscribe` is `true` (default `true`).
* [`auto_remonitor`](Config::auto_remonitor) - When the client reconnects, if in `monitor` mode, the
  [`monitor`](crate::commands::BlockingCommands::monitor) command will be resent automatically
* [`connection_name`](Config::connection_name) - Set the name of the connection to make
  it easier to identity the connection in client list.
* [`keep_alive`](Config::keep_alive) - Idle time before the TCP keep-alive probes start,
  or `None` (`keep_alive=0` in a URL) to disable keep-alive (default `30` s).
* [`no_delay`](Config::no_delay) - Enable/disable the use of Nagle's algorithm (default `true`)
* [`retry_on_error`](Config::retry_on_error) - Defines the default strategy for retries on network error (default `false`).
* [`max_command_attempts`](Config::max_command_attempts) - Maximum number of times a command is sent
  before giving up (default `5`).
* [`buffers.read_capacity`](BufferConfig::read_capacity), [`buffers.tape_capacity`](BufferConfig::tape_capacity),
  [`buffers.shrink_factor`](BufferConfig::shrink_factor), [`buffers.shrink_hysteresis`](BufferConfig::shrink_hysteresis) -
  Sizing and recycling policy of the connection's buffers, one parameter per field of [`BufferConfig`].
* [`backpressure.max_queued_bytes`](BackpressureConfig::max_queued_bytes),
  [`backpressure.max_pubsub_bytes`](BackpressureConfig::max_pubsub_bytes),
  [`backpressure.max_push_bytes`](BackpressureConfig::max_push_bytes) - Memory budgets, in bytes,
  `0` to disable one.
* [`limits.max_nesting_depth`](RespLimits::max_nesting_depth), [`limits.max_bulk_length`](RespLimits::max_bulk_length),
  [`limits.max_collection_length`](RespLimits::max_collection_length) - What the RESP parser accepts
  from the server.
* [`reconnection`](Config::reconnection) - The reconnection policy: `constant`, `linear` or `exponential`.
  Its fields follow, each one spelled `reconnection.<field>`: `max_attempts` (default `0`, retry forever)
  and `jitter` (default `100` ms) on all three, `delay` (default `1000` ms) on `constant`, `delay` and
  `max_delay` on `linear`, `min_delay`, `max_delay` and `multiplicative_factor` on `exponential`.
  A field the policy does not carry is rejected, and the ones with no default are required: `max_delay`
  clamps the delay, so a policy given none would reconnect with no backoff at all.
  [`ReconnectionConfig::Custom`] is Rust code and has no URL spelling.
* [`read_preference`](ClusterConfig::read_preference) - (Cluster only) Which node of a shard reads
  are routed to (default `master`).
* [`topology_refresh_interval`](ClusterConfig::topology_refresh_interval) - (Cluster only) How often
  the topology is reloaded on its own, `0` to reload it only on a redirection (default `60000` ms).
* [`wait_between_failures`](SentinelConfig::wait_between_failures) - (Sentinel only) Waiting time after
  failing before connecting to the next Sentinel instance (default `250` ms).
* [`sentinel_username`](SentinelConfig::username) - (Sentinel only) Sentinel username
* [`sentinel_password`](SentinelConfig::password) - (Sentinel only) Sentinel password
* `db` - (Unix socket only) The default database, which the TCP schemes spell as the last
  path segment instead (default `0`).

### Rotating credentials

A password embedded in an URL is fixed for the life of the client. When the password is a
short-lived token (AWS ElastiCache IAM, GCP Memorystore IAM, Azure Entra ID, Vault), set
[`Config::credentials_provider`] instead: it is consulted at every handshake, so each
reconnection authenticates with the current token. [`SentinelConfig::credentials_provider`] does
the same for the Sentinel instances themselves. Neither has a URL representation; both are set on
the config itself. See [`CredentialsProvider`].

### Supplying the transport

[`ServerConfig::Custom`] takes a [`TransportFactory`], which hands the client a byte stream to
speak RESP over instead of one it opens itself: an in-memory pipe, a tunnel, an
SSH-forwarded channel, a TLS stack configured elsewhere. It is asked for a stream at every dial,
so a reconnection gets a fresh one. Like a credentials provider, it has no URL representation.

```
use rustis::client::{Config, CustomTransport, ServerConfig, TransportReader, TransportWriter};

let mut config = Config::default();
config.server = ServerConfig::Custom(CustomTransport::new(|| async {
    let (client_side, server_side) = tokio::io::duplex(4096);
    // drive `server_side` with a server of your own here
    drop(server_side);
    let (reader, writer) = tokio::io::split(client_side);
    Ok((Box::new(reader) as TransportReader, Box::new(writer) as TransportWriter))
}));
```

### Example

```
use rustis::{client::Client, resp::cmd, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // standalone, host=localhost, port=6379 (default), database=1
    let client = Client::connect("redis://localhost/1").await?;

    Ok(())
}
```

# Cancellation and timeouts

Dropping the future of a command does **not** cancel the command. By the time the future is
awaited, the message is already in the send queue: it is written to the connection and executed
by the server. Dropping the future only discards the reply.

This is what happens on every `tokio::time::timeout`, every `select!` branch that loses, every
`JoinHandle::abort`, every HTTP request cancelled by the client — and on
[`Config::command_timeout`](crate::client::Config::command_timeout), which is itself a timeout
around the wait for the reply.

So a timeout error tells you nothing about whether the command ran. For an idempotent command
(`GET`, `SET` of a constant, `DEL`) that is harmless; for `INCR`, `LPUSH`, `XADD` or a Lua script
with side effects, the effect may have happened, and retrying applies it twice.

The reason is the protocol: commands are pipelined on a shared connection, and replies come back
in the order the commands were sent. A message already handed to the connection cannot be pulled
back, and nothing in RESP cancels a command in flight.
[`Config::retry_on_error`](crate::client::Config::retry_on_error) and
[`Config::max_command_attempts`](crate::client::Config::max_command_attempts) add the same
duplication on a network error: a command that reached the server before the connection broke is
sent again.

Design the commands you put a deadline on to be idempotent, or check the state after a timeout
instead of assuming the command did not run.

```no_run
use rustis::{client::Client, commands::StringCommands, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    match tokio::time::timeout(Duration::from_millis(10), client.incr("counter")).await {
        Ok(result) => {
            let value: i64 = result?;
            println!("counter: {value}");
        }
        // The reply never came back in time. The counter may still have been
        // incremented: incrementing again here would count twice.
        Err(_elapsed) => {
            let value: i64 = client.get("counter").await?;
            println!("counter after timeout: {value}");
        }
    }

    Ok(())
}
```

# Pipelining

One of the most performant Redis feature is [pipelining](https://redis.io/docs/manual/pipelining/).
This allow to optimize round-trip times by batching Redis commands.

### API description

You can create a pipeline on a [`Client`] instance by calling the associated fonction [`create_pipeline`](Client::create_pipeline).
Be sure to store the pipeline instance in a mutable variable because a pipeline requires an exclusive access.

Once the pipeline is created, you can use exactly the same commands that you would directly use on a client instance.
This is possible because the [`Pipeline`] implements all the built-in [command traits](crate::commands).

The main difference, is that you have to choose for each command:
* to [`queue`](BatchPreparedCommand::queue) it, meaning that the [`Pipeline`] instance will queue the command in an internal
  queue to be able to send later the batch of commands to the Redis server.
* to [`forget`](BatchPreparedCommand::forget) it, meaning that the command will be queued as well **BUT** its response won't be awaited
  by the [`Pipeline`] instance

Finally, call the [`execute`](Pipeline::execute) associated function.

It is the caller responsability to use the right type to cast the server response
to the right tuple or collection depending on which command has been
[queued](BatchPreparedCommand::queue) or [forgotten](BatchPreparedCommand::forget).

The most generic type that can be requested as a result is `Vec<resp::Value>`

### Example
```
use rustis::{
    client::{Client, Pipeline, BatchPreparedCommand},
    commands::StringCommands,
    resp::{cmd, Value}, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<()>("key1").queue();
    pipeline.get::<()>("key2").queue();

    let (value1, value2): (String, String) = pipeline.execute().await?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    Ok(())
}
```

# Transactions
[Redis Transactions](https://redis.io/docs/manual/transactions/) allow the execution of a group of commands in a single step.

All the commands in a transaction are serialized and executed sequentially.
A request sent by another client will never be served in the middle of the execution of a Redis Transaction.
This guarantees that the commands are executed as a single isolated operation.

### API description

You can create a transaction on a client instance by calling the associated fonction [`create_transaction`](Client::create_transaction).
Be sure to store the transaction instance in a mutable variable because a transaction requires an exclusive access.

Once the transaction is created, you can use exactly the same commands that you would directly use on a client instance.
This is possible because the [`Transaction`] implements all the built-in [command traits](crate::commands).

The main difference, is that you have to choose for each command:
* to [`queue`](BatchPreparedCommand::queue) it, meaning that the [`Transaction`] instance will queue the command in an internal
  queue to be able to send later the batch of commands to the Redis server.
* to [`forget`](BatchPreparedCommand::forget) it, meaning that the command will be queued as well **BUT** its response won't be awaited
  by the [`Transaction`] instance.

Finally, call the [`execute`](Transaction::execute) associated function.

It is the caller responsability to use the right type to cast the server response
to the right tuple or collection depending on which command has been
[queued](BatchPreparedCommand::queue) or [forgotten](BatchPreparedCommand::forget).

The most generic type that can be requested as a result is `Vec<(resp::Value)>`

### Example
```
use rustis::{
    client::{Client, Transaction, BatchPreparedCommand},
    commands::StringCommands,
    resp::{cmd, Value}, Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "value1").forget();
    transaction.set("key2", "value2").forget();
    transaction.get::<()>("key1").queue();
    let value: String = transaction.execute().await?;

    assert_eq!("value1", value);

    Ok(())
}
```

# Pub/Sub

[`Pub/Sub`](https://redis.io/docs/manual/pubsub/) is a Redis architecture were senders can publish messages into channels
and subscribers can subscribe by channel names or patterns to receive messages.

### Publishing

To publish a message, you can call the [`publish`](crate::commands::PubSubCommands::publish)
associated function on its dedicated trait.

It also possible to use the sharded flavor of the publish function: [`spublish`](crate::commands::PubSubCommands::spublish).

### Subscribing

**rustis** implements subsribing through an async [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html).

You can create a [`PubSubStream`] by calling [`subscribe`](crate::commands::PubSubCommands::subscribe),
[`psubscribe`](crate::commands::PubSubCommands::psubscribe), or [`ssubscribe`](crate::commands::PubSubCommands::ssubscribe).

Then by calling [`next`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.next) on the pub/sub stream, you can
wait for an incoming message in the form of the struct [`PubSubMessage`].

You can also create a [`PubSubStream`] without an upfront subscription by calling [`create_pub_sub`](crate::client::Client::create_pub_sub).

### Managing Multiplexed Subscriptions

Because **rustis** implements the RESP3 protocol, there is no limitation when using subscriptions on a multiplexed connection.
Pub/Sub messages and regular command responses are cleanly distinguished at the protocol level,
allowing both to coexist safely on the same shared connection.

### Simple Example

```
use rustis::{
    client::{Client, ClientPreparedCommand},
    commands::{FlushingMode, PubSubCommands, ServerCommands},
    resp::{cmd, Value}, Result,
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let subscribing_client = Client::connect("127.0.0.1:6379").await?;
    let publishing_client = Client::connect("127.0.0.1:6379").await?;

    subscribing_client.flushdb(FlushingMode::Sync).await?;

    // Create a subscription from the subscribing client:
    let mut pub_sub_stream = subscribing_client.subscribe("mychannel").await?;

    // The publishing client publishes a message on the channel:
    publishing_client.publish("mychannel", "mymessage").await?;

    // Let's now iterate over messages received:
    while let Some(Ok(message)) = pub_sub_stream.next().await {
        assert_eq!(b"mychannel", message.channel());
        assert_eq!(b"mymessage", message.payload());
        break;
    }

    Ok(())
}
```

Once the stream has been created, it is still possible to add additional subscriptions
by calling [`subscribe`](PubSubStream::subscribe), [`psubscribe`](PubSubStream::psubscribe)
or [`ssubscribe`](PubSubStream::ssubscribe) on the [`PubSubStream`] instance.

### Split Stream Example

To make it easy to modify subscriptions while iterating over messages, you can use the [`split`](PubSubStream::split) method to
split the stream into [sink](PubSubSplitSink) and [stream](PubSubSplitStream) parts. Once this is done, you call [`subscribe`](PubSubSplitSink::subscribe)
or [`unsubscribe`](PubSubSplitSink::unsubscribe) (and related methods) on the sink while the split stream is used only for iteration. This can be useful
when you want to split ownership between async tasks.

```
use rustis::{
    client::{Client, ClientPreparedCommand},
    commands::{FlushingMode, PubSubCommands, ServerCommands},
    resp::{cmd, Value}, Result,
};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let subscribing_client = Client::connect("127.0.0.1:6379").await?;
    let regular_client = Client::connect("127.0.0.1:6379").await?;

    regular_client.flushdb(FlushingMode::Sync).await?;

    // This time we will split the stream into sink and stream parts:
    let (mut sink, mut stream) = subscribing_client.subscribe("mychannel").await?.split();

    // You can then subscribe or unsubscribe using the sink.
    // Typically you would pass ownership of the sink to another async task.
    sink.subscribe("otherchannel").await?;
    sink.psubscribe("o*").await?;

    regular_client.publish("mychannel", "mymessage").await?;

    // Iterate over messages using the split stream:
    while let Some(Ok(message)) = stream.next().await {
        assert_eq!(b"mychannel", message.channel());
        assert_eq!(b"mymessage", message.payload());
        break;
    }

    Ok(())
}
```
*/

mod bounded_channel;
#[allow(clippy::module_inception)]
mod client;
mod client_stats;
mod client_tracking_invalidation_stream;
mod command_future;
mod command_traits;
mod config;
mod credentials_provider;
mod exclusive_client;
mod interceptor;
mod message;
mod monitor_stream;
mod pipeline;
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
#[cfg(feature = "pool")]
mod pooled_client_manager;
mod prepared_command;
mod pub_sub_stream;
mod reconnection_policy;
mod transaction;
mod transport_factory;

pub(crate) use bounded_channel::*;

pub use client::*;
pub use client_stats::*;
pub use client_tracking_invalidation_stream::*;
pub use command_future::*;
pub use config::*;
pub use credentials_provider::*;
pub use exclusive_client::*;
pub use interceptor::*;
pub(crate) use message::*;
pub use monitor_stream::*;
pub use pipeline::*;
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
#[cfg(feature = "pool")]
pub use pooled_client_manager::*;
pub use prepared_command::*;
pub use pub_sub_stream::*;
pub use reconnection_policy::*;
pub use transaction::*;
pub use transport_factory::*;
