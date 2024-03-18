/*!
Defines types related to the clients structs and their dependencies:
[`Client`], [`PooledClientManager`], [`Pipeline`], [`Transaction`] and how to configure them

# Clients

The central object in **rustis** is the [`Client`].

It will allow you to connect to the Redis server, to send command requests
and to receive command responses and push messages.

The [`Client`] struct can be used in 3 different modes
* As a single client
* As a mutiplexer
* In a pool of clients

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

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
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
A [`Client`] instance can be cloned, allowing requests
to be sent concurrently on the same underlying connection.

The multiplexer mode is great because it offers much performance in a multi-threaded architecture, with only a single
underlying connection. It should be the prefered mode for Web applications.

### Limitations
Beware that using [`Client`] in a multiplexer mode, by cloning an instance across multiple threads,
is not suitable for using [blocking commands](crate::commands::BlockingCommands)
because they monopolize the whole connection which cannot be shared anymore.

Moreover using the [`watch`](crate::commands::TransactionCommands::watch) command is not compatible
with the multiplexer mode is either. Indeed, it's the shared connection that will be watched, not only
the [`Client`] instance through which the [`watch`](crate::commands::TransactionCommands::watch) command is sent.

### Managing multiplexed subscriptions

Even if the [`subscribe`][crate::commands::PubSubCommands::subscribe] monopolize the whole connection,
it is still possible to use it in a multiplexed [`Client`].

Indeed the subscribing mode of Redis still allows to share the connection between multiple clients,
at the only condition that this connection is dedicated to subscriptions.

In a Web application that requires subscriptions and regualar commands, the prefered solution
would be to connect two multiplexed clients to the Redis server:
* 1 for the subscriptions
* 1 for the regular commands

### See also
[Multiplexing Explained](https://redis.com/blog/multiplexing-explained/)

### Example
```
use rustis::{
    client::{Client, IntoConfig},
    commands::{FlushingMode, PubSubCommands, ServerCommands, StringCommands},
    Result
};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let config = "127.0.0.1:6379".into_config()?;
    let regular_client1 = Client::connect(config.clone()).await?;
    let pub_sub_client = Client::connect(config).await?;

    regular_client1.flushdb(FlushingMode::Sync).await?;

    regular_client1.set("key", "value").await?;
    let value: String = regular_client1.get("key").await?;
    println!("value: {value:?}");

    // clone a second instance on the same underlying connection
    let regular_client2 = regular_client1.clone();
    let value: String = regular_client2.get("key").await?;
    println!("value: {value:?}");

    // use 2nd connection to manager subscriptions
    let pub_sub_stream = pub_sub_client.subscribe("my_channel").await?;
    pub_sub_stream.close().await?;

    Ok(())
}
```

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

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
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

### Schemes
The URL scheme is used to detect the server type:
* `redis://` - Non secure TCP connection to a standalone Redis server
* `rediss://` - Secure (TSL) TCP connection to a standalone Redis server
* `redis+sentinel://` or `redis-sentinel://` - Non secure TCP connection to a Redis sentinel network
* `rediss+sentinel://` or `rediss-sentinel://` - Secure (TSL) TCP connection to a Redis sentinel network
* `redis+cluster://` or `redis-cluster://` - Non secure TCP connection to a Redis cluster
* `rediss+cluster://` or `rediss-cluster://` - Secure (TSL) TCP connection to a Redis cluster

### QueryParameters
Query parameters match perfectly optional configuration fields
of the struct [`Config`] or its dependencies:
* [`connect_timeout`](Config::connect_timeout) - The time to attempt a connection before timing out (default `10,000` ms).
* [`command_timeout`](Config::command_timeout) - If a command does not return a reply within a set number of milliseconds,
   a timeout error will be thrown. If set to 0, no timeout is apply (default `0`).
* [`auto_resubscribe`](Config::auto_resubscribe) - When the client reconnects, channels subscribed in the previous connection will be
 resubscribed automatically if `auto_resubscribe` is `true` (default `true`).
* [`auto_remonitor`](Config::auto_remonitor) - When the client reconnects, if in `monitor` mode, the
  [`monitor`](crate::commands::BlockingCommands::monitor) command will be resent automatically
* [`connection_name`](Config::connection_name) - Set the name of the connection to make
  it easier to identity the connection in client list.
* [`keep_alive`](Config::keep_alive) - Enable/disable keep-alive functionality (default `None`)
* [`no_delay`](Config::no_delay) - Enable/disable the use of Nagle's algorithm (default `true`)
* [`max_command_attempts`](Config::max_command_attempts) - Maximum number of retry attempts to send a command to the Redis server (default `3`).
* [`retry_on_error`](Config::retry_on_error) - Defines the default strategy for retries on network error (default `false`).
* [`wait_between_failures`](SentinelConfig::wait_between_failures) - (Sentinel only) Waiting time after
  failing before connecting to the next Sentinel instance (default `250` ms).
* [`sentinel_username`](SentinelConfig::username) - (Sentinel only) Sentinel username
* [`sentinel_password`](SentinelConfig::password) - (Sentinel only) Sentinel password

### Example

```
use rustis::{client::Client, resp::cmd, Result};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // standalone, host=localhost, port=6379 (default), database=1
    let client = Client::connect("redis://localhost/1").await?;

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

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    let mut pipeline = client.create_pipeline();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.get::<_, ()>("key1").queue();
    pipeline.get::<_, ()>("key2").queue();

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

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    let mut transaction = client.create_transaction();

    transaction.set("key1", "value1").forget();
    transaction.set("key2", "value2").forget();
    transaction.get::<_, ()>("key1").queue();
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

Subscribing will block the current client connection, in order to let the client wait for incoming messages.
Consequently, **rustis** implements subsribing through an async [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html).

You can create a [`PubSubStream`] by calling [`subscribe`](crate::commands::PubSubCommands::subscribe),
[`psubscribe`](crate::commands::PubSubCommands::psubscribe), or [`ssubscribe`](crate::commands::PubSubCommands::ssubscribe).

Then by calling [`next`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.next) on the pub/sub stream, you can
wait for an incoming message in the form of the struct [`PubSubMessage`].

You can also create a [`PubSubStream`] without an upfront subscription by calling [`create_pub_sub`](crate::client::Client::create_pub_sub).

### Warning!

Multiplexed [`Client`] instances must be dedicated to Pub/Sub once a subscribing function has been called.
Because subscription blocks the multiplexed client shared connection other callers would be blocked when sending regular commands.

### Simple Example

```
use rustis::{
    client::{Client, ClientPreparedCommand},
    commands::{FlushingMode, PubSubCommands, ServerCommands},
    resp::{cmd, Value}, Result,
};
use futures_util::StreamExt;

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let subscribing_client = Client::connect("127.0.0.1:6379").await?;
    let regular_client = Client::connect("127.0.0.1:6379").await?;

    regular_client.flushdb(FlushingMode::Sync).await?;

    // Create a subscription from the subscribing client:
    let mut pub_sub_stream = subscribing_client.subscribe("mychannel").await?;

    // The regular client publishes a message on the channel:
    regular_client.publish("mychannel", "mymessage").await?;

    // Let's now iterate over messages received:
    while let Some(Ok(message)) = pub_sub_stream.next().await {
        assert_eq!(b"mychannel".to_vec(), message.channel);
        assert_eq!(b"mymessage".to_vec(), message.payload);
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

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
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
        assert_eq!(b"mychannel".to_vec(), message.channel);
        assert_eq!(b"mymessage".to_vec(), message.payload);
        break;
    }

    Ok(())
}
```
*/

#[allow(clippy::module_inception)]
mod client;
mod client_state;
mod client_tracking_invalidation_stream;
mod config;
mod message;
mod monitor_stream;
mod pipeline;
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
#[cfg(feature = "pool")]
mod pooled_client_manager;
mod prepared_command;
mod pub_sub_stream;
mod transaction;

pub use client::*;
pub use client_state::*;
pub(crate) use client_tracking_invalidation_stream::*;
pub use config::*;
pub(crate) use message::*;
pub use monitor_stream::*;
pub use pipeline::*;
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
#[cfg(feature = "pool")]
pub use pooled_client_manager::*;
pub use prepared_command::*;
pub use pub_sub_stream::*;
pub use transaction::*;
