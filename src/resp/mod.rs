/*!
Defines types related to the [`RESP`](https://redis.io/docs/reference/protocol-spec/) protocol and their encoding/decoding

# Object Model

**rustis** provides an object model in the form of a generic data struct, comparable to the XML DOM,
and which matches perfectly the RESP protocol: the enum [`resp::Value`](Value).

Each variant of this enum matches a [`RESP`](https://redis.io/docs/reference/protocol-spec/) type.

Because, navigating through a [`resp::Value`](Value) instance can be verbose and requires a lot of pattern matching,
**rustis** provides a [`resp::Value`](Value) to Rust type conversion with a [serde](https://serde.rs/)
deserializer implementation of a [`resp::Value`](Value) reference.

This conversion is easily accessible through the associate function [`Value::into`](Value::into).

# Command arguments

**rustis** provides an idiomatic way to pass arguments to [commands](crate::commands).
Basically a [`Command`] is a built through a builder which accepts a command name and one ore more command arguments.

The only requirement for the command argument is that they must implement the serde [`Serialize`] trait.
It gives to **rustis** a great flexibility to accept many type of arguments for the same command.

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    Result,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct MyI32(i32);

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", 12).await?;
    client.set("key", 12i64).await?;
    client.set("key", 12.12).await?;
    client.set("key", true).await?;
    client.set("key", "value").await?;
    client.set("key", "value".to_owned()).await?;
    client.set("key", 'c').await?;
    client.set("key", MyI32(12)).await?;

    Ok(())
}
```
## Byte arguments and serde limitations

Due to how serde handles byte types, passing raw byte values like `&[u8]`,
`Vec<u8>`, or byte literals like `b"val"` directly as command arguments
will **not** produce a single RESP bulk string. Instead, serde serializes
them as sequences of individual integer values, resulting in a runtime error.

This is a fundamental serde limitation: without specialization, there is no
way to distinguish a `&[u8]` from any other `&[T]` at the trait level.
Note that `&str` works correctly because it is a distinct type, not a slice.

To pass raw bytes as a single bulk string argument, use the provided adapter types:

- [`BulkString`] for owned byte data (`Vec<u8>`) — moves ownership, zero allocation
- [`RefBulkString`] for borrowed byte data (`&[u8]`) — zero allocation

#### Example
```
use rustis::{
    client::Client,
    commands::StringCommands,
    resp::{BulkString, RefBulkString},
    Result,
};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    // &[u8]: use RefBulkString (zero allocation, borrowed)
    client.set("key", RefBulkString::new(b"val")).await?;

    // Vec<u8>: use BulkString (zero allocation, owned)
    client.set("key", BulkString::new(b"val".to_vec())).await?;

    Ok(())
}
```
# Command results

**rustis** provides an idiomatic way to convert command results into Rust types with the help of [serde](serde.rs)

You will notice that each built-in command returns a [`PreparedCommand<R>`](crate::client::PreparedCommand)
struct where `R` represents the [`Response`] of the command.

The different command traits implementations ([`Client`](crate::client::Client), [`Pipeline`](crate::client::Pipeline)
 or [`Transaction`](crate::client::Transaction)) add a constraint on the reponse `R`:
 it must implement serde [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html) trait.

 Indeed, **rustis** provides a serde implementation of a [`RESP deserializer`](RespDeserializer).
 Each custom struct or enum defined as a response of a built-command implements
 serde [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html) trait,
 in order to deserialize it automatically from a RESP Buffer.
*/
// This module is fed directly by server bytes: every length, cardinality and
// offset here is attacker-controlled, so an out-of-bounds index is reachable
// from the wire rather than from a local mistake. Indexing is denied, not
// warned; see the panic policy in `lib.rs`.
#![deny(clippy::indexing_slicing)]

pub(crate) use arg_counter::*;
pub(crate) use arg_serializer::*;
#[cfg(feature = "bench")]
pub use bench_support::*;
pub(crate) use buffer_decoder::*;
pub use bulk_string::*;
pub use command::*;
pub use command_args::*;
pub(crate) use command_encoder::*;
pub use fast_path_command_builder::*;
#[cfg(feature = "json")]
pub use json::*;
pub(crate) use resp_batch_deserializer::*;
pub(crate) use resp_buf::*;
pub use resp_deserializer::*;
pub(crate) use resp_frame_parser::*;
pub(crate) use resp_response::*;
pub(crate) use resp_tape::*;
pub use response::*;
pub use util::*;
pub use value::*;
pub(crate) use value_deserialize::*;

mod arg_counter;
mod arg_serializer;
#[cfg(feature = "bench")]
mod bench_support;
mod buffer_decoder;
mod bulk_string;
mod command;
mod command_args;
mod command_encoder;
mod fast_path_command_builder;
#[cfg(feature = "json")]
mod json;
mod resp_batch_deserializer;
mod resp_buf;
mod resp_deserializer;
mod resp_frame_parser;
mod resp_response;
mod resp_tape;
mod response;
mod util;
mod value;
mod value_deserialize;
mod value_deserializer;
