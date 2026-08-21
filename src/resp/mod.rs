/*!
Defines types related to the [`RESP`](https://redis.io/docs/reference/protocol-spec/) protocol and their encoding/decoding

# Object Model

**rustis** provides an object model in the form of a generic data struct, comparable to the XML DOM,
and which matches perfectly the RESP protocol: the enum [`resp::Value`](Value).

Each variant of this enum matches a [`RESP`](https://redis.io/docs/reference/protocol-spec/) type.

A [`resp::Value`](Value) is read either variant by variant, through the accessors
[`as_str`](Value::as_str), [`as_bytes`](Value::as_bytes), [`as_i64`](Value::as_i64),
[`as_f64`](Value::as_f64), [`as_bool`](Value::as_bool), [`as_array`](Value::as_array),
[`as_map`](Value::as_map), [`as_error`](Value::as_error), [`is_null`](Value::is_null) and
[`get`](Value::get) — each answering [`None`] when the variant does not match — or all at once,
by converting it to a Rust type. **rustis** provides that conversion with a
[serde](https://serde.rs/) deserializer implementation of a [`resp::Value`](Value) reference,
reached through the associate function [`Value::into`](Value::into).

A command whose reply shape is known is best deserialized straight into the type that models it:
`Value` is the fallback for the replies that are not.

# Command arguments

**rustis** provides an idiomatic way to pass arguments to [commands](crate::commands).
Basically a [`Command`] is a built through a builder which accepts a command name and one ore more command arguments.

The only requirement for the command argument is that they must implement the serde [`Serialize`](serde::Serialize) trait.
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

#[tokio::main]
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

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    // &[u8]: use RefBulkString (zero allocation, borrowed)
    client.set("key", RefBulkString::new(b"val")).await?;

    // Vec<u8>: use BulkString (zero allocation, owned)
    client.set("key", BulkString::new(b"val".to_vec())).await?;

    Ok(())
}
```
## Why `impl Serialize` and not a trait of **rustis**' own

A trait of our own — `SingleArg`, `IntoArgs`, whatever the name — would let the compiler check
that a key is a single value. **rustis** does not define one, and cannot, because of Rust's
orphan rule: an `impl` is only allowed in the crate that defines the trait or the crate that
defines the type. Writing

```text
// in your own crate
impl rustis::resp::SingleArg for uuid::Uuid {}
//   error[E0117]: only traits defined in the current crate
//                 can be implemented for types defined outside of it
```

since neither the trait nor the type belongs to your crate. (The example does not compile for a
second reason: `SingleArg` does not exist, which is what this section is about.)
Every third-party type would then need a newtype wrapper at each call site: `uuid::Uuid`,
`serde_json::Value`, `chrono::DateTime`, `rust_decimal::Decimal`. `Serialize` has no such problem,
being already implemented by those crates themselves.

`serde_json::Value` shows the trait could not even be honest where the orphan rule allows it.
One type, five argument counts: `Value::String` and `Value::Number` write one argument,
`Value::Null` writes none, `Value::Array` writes one per element and `Value::Object` two per
entry. A trait is a predicate on a *type*; the count is a property of the *value*. No `impl` can
answer for `Value`.

## Argument counts are checked, not typed

The count is therefore checked where a key is added, since a mistake there is otherwise silent.
`None` and an empty collection write no argument at all, which leaves the command a key short —
and, in Cluster mode, with no hash slot, so it is routed to a **random node** instead of the one
that owns the key. A struct or a sequence writes several arguments where one key was meant.

Both fail the command with
[`InvalidKeyArity`](crate::ClientError::InvalidKeyArity), naming the command and the count:

```
use rustis::{client::Client, commands::StringCommands, ClientError, ErrorKind, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    // a key that serializes to no argument at all
    let result: Result<String> = client.get(None::<String>).await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Client(ClientError::InvalidKeyArity { .. })
    ));

    Ok(())
}
```

The check is on the count alone, so it costs nothing in flexibility: any foreign type is a valid
key as soon as it writes one argument, whether or not **rustis** has ever heard of it. A
`uuid::Uuid` passes, and so does `Value::String` — while `Value::Array` does not, which is the
distinction a marker trait was unable to make.

Values are not checked: a struct or a map as a value is the point of `HSET`
(`client.hset("user:1", my_struct)`), so any count is legitimate there.

# Command results

**rustis** provides an idiomatic way to convert command results into Rust types with the help of [serde](serde.rs)

You will notice that each built-in command returns a [`PreparedCommand<R>`](crate::client::PreparedCommand)
struct where `R` is the response type the caller declares.

`R` must implement serde
[`DeserializeOwned`](https://docs.rs/serde/latest/serde/de/trait.DeserializeOwned.html), which is
the only constraint there is: nothing relates it at compile time to what the server actually
answers.

 Indeed, **rustis** provides a serde deserializer over the RESP wire format.
 Each custom struct or enum defined as a response of a built-command implements
 serde [`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html) trait,
 in order to deserialize it automatically from a RESP Buffer.

## A `nil` reply needs an `Option`
Redis answers `nil` for a key, a field or an element that does not exist. Read as a number, a
string or a `char`, that absence has no honest value: `0` and `""` are values a present key can
hold, so returning one would make the missing key indistinguishable from it.

A `nil` read as a scalar therefore fails the command with
[`UnexpectedNil`](crate::ClientError::UnexpectedNil), and [`Option`] is the type that accepts it.
The rule reaches inside the reply: an element of a collection and a field of a struct are scalars
too, which is why `HMGET` is read as `Vec<Option<String>>`.

Three readings keep the `nil`. A *collection* stays empty — an absent list read as `Vec<String>`
has no elements, and a byte string has no bytes. [`Value`] carries it as [`Value::Null`]. And a
`bool` reads it as `false`, the server answering `nil` to say a conditional write did not happen.

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;
    client.flushall(FlushingMode::Sync).await?;

    // the key does not exist, and `0` would be a lie
    assert!(client.get::<i64>("counter").await.is_err());

    // `Option` accepts the absence
    let counter: Option<i64> = client.get("counter").await?;
    assert_eq!(None, counter);

    client.set("counter", 0).await?;
    let counter: Option<i64> = client.get("counter").await?;
    assert_eq!(Some(0), counter);

    Ok(())
}
```
*/
// This module is fed directly by server bytes: every length, cardinality and
// offset here is attacker-controlled, so an out-of-bounds index is reachable
// from the wire rather than from a local mistake. Indexing is denied, not
// warned; see the panic policy in `lib.rs`.
#![deny(clippy::indexing_slicing)]
// Same reasoning applied to `as`, which `arithmetic_side_effects` does not cover:
// on a wire-supplied value a narrowing cast truncates, a signed-to-unsigned one
// wraps and a float-to-integer one saturates or maps NaN to zero — silently, in
// debug as in release. Every conversion of a decoded value must therefore be
// `TryFrom` or a documented-exact `as`, each surviving cast carrying an
// `#[expect(…, reason = "…")]` naming the invariant that makes it exact.
#![deny(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless
)]

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
pub(crate) use resp_buf::*;
pub(crate) use resp_deserializer::*;
pub(crate) use resp_frame_parser::*;
pub(crate) use resp_response::*;
pub(crate) use resp_scalar::*;
pub(crate) use resp_tags::*;
pub(crate) use resp_tape::*;
pub use util::*;
pub use value::*;
pub(crate) use value_deserialize::*;

mod arg_counter;
mod arg_serializer;
#[cfg(feature = "bench")]
pub mod bench_support;
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
mod resp_scalar;
mod resp_tags;
mod resp_tape;
mod util;
mod value;
mod value_deserialize;
mod value_deserializer;
