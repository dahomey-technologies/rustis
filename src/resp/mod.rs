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

pub(crate) use arg_counter::*;
pub(crate) use arg_serializer::*;
pub(crate) use buffer_decoder::*;
pub use bulk_string::*;
pub use command::*;
pub use command_args::*;
pub(crate) use command_encoder::*;
pub use fast_path_command_builder::*;
#[cfg(feature = "json")]
pub use json::*;
pub(crate) use resp_batch_deserializer::*;
pub use resp_buf::*;
pub use resp_deserializer::*;
pub(crate) use resp_frame_scanner::*;
pub use resp_serializer::*;
pub use response::*;
pub use util::*;
pub use value::*;
pub(crate) use value_deserialize::*;

mod arg_counter;
mod arg_serializer;
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
mod resp_frame_scanner;
mod resp_serializer;
mod response;
mod util;
mod value;
mod value_deserialize;
mod value_deserializer;
mod value_serialize;
