/*!
Defines types related to the [`RESP`](https://redis.io/docs/reference/protocol-spec/) protocol and their encoding/decoding

# Object Model

**rustis** provides an object model in the form of a generic data struct, comparable to the XML DOM,
and which matches perfectly the RESP protocol: the enum [`resp::Value`](Value).

Each variant of this enum matches a [`RESP`](https://redis.io/docs/reference/protocol-spec/) type.

Because, navigating through a [`resp::Value`](Value) instance can be verbose and requires a lot of pattern matching,
**rustis** provides:
* Rust type to [`Command`](Command) conversion, with the trait [`IntoArgs`](IntoArgs).
* [`resp::Value`](Value) to Rust type conversion, with the trait [`FromValue`](FromValue).

# Command arguments

**rustis** provides an idiomatic way to pass arguments to [commands](crate::commands).
Basically a [`Command`](Command) is a collection of [`CommandArg`](CommandArg)s

You will notice that each built-in command expects arguments through a set of traits defined in this module.

For each trait, you can add your own implementations for your custom types
or request additional implementation for standard types.

### IntoArgs

The trait [`IntoArgs`](IntoArgs) allows to convert a complex type into one ore multiple argumentss.
Basically, the conversion function can add multiple arguments to an existing argument collection: the [`CommandArgs`](CommandArgs) struct.

Current implementation provides the following conversions:
* `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `usize`, `isize`,
* `f32`, `f64`,
* `bool`,
* `String`, `char`, `&str`, `Vec<u8>`, `&[u8; N]`, `[u8; N]`, `&[u8]`
* `Option<T>`
* `(T, U)`
* `(T, U, V)`
* `Vec<T>`
* `[T;N]`
* `SmallVec<A>`
* `BTreeSet<T>`
* `HashSet<T, S>`
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`

Nevertheless, [`IntoArgs`](IntoArgs) is not expected directly in built-in commands arguments.

The following traits are used to constraints which implementations of [`IntoArgs`](IntoArgs)
are expected by a specific argument of a built-in command.

### SingleArg

Several Redis commands expect a Rust type that should be converted in a single command argument.

**rustis** uses the trait [`SingleArg`](SingleArg) to implement this behavior.

Current implementation provides the following conversions:
* [`CommandArg`](CommandArg)
* `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `usize`, `isize`,
* `f32`, `f64`,
* `bool`,
* `String`, `char`, `&str`, `Vec<u8>`, `&[u8; N]`, `[u8; N]`, `&[u8]`
* `Option<T>` where `T: SingleArg`

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{CommandArgs, IntoArgs, SingleArg},
    Result,
};

pub struct MyI32(i32);

 impl IntoArgs for MyI32 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.0)
    }
}

impl SingleArg for MyI32 {}

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", 12).await?;
    client.set("key", 12i64).await?;
    client.set("key", 12.12).await?;
    client.set("key", true).await?;
    client.set("key", true).await?;
    client.set("key", "value").await?;
    client.set("key", "value".to_owned()).await?;
    client.set("key", 'c').await?;
    client.set("key", b"value").await?;
    client.set("key", &b"value"[..]).await?;
    client.set("key", b"value".to_vec()).await?;
    client.set("key", MyI32(12)).await?;

    Ok(())
}
```

### SingleArgCollection

Several Redis commands expect a collection with elements that will produced a single
command argument each

**rustis** uses the trait [`SingleArgCollection`](SingleArgCollection) to implement this behavior.

Current implementation provides the following conversions:
* `T` (for the single item case)
* `Vec<T>`
* `[T;N]`
* `SmallVec<A>`
* `BTreeSet<T>`
* `HashSet<T, S>`

where each of theses implementations must also implement [`IntoArgs`](IntoArgs)

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, ListCommands},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashSet, BTreeSet};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.lpush("key", 12).await?;
    client.lpush("key", [12, 13, 14]).await?;
    client.lpush("key", vec![12, 13, 14]).await?;
    client.lpush("key", SmallVec::from([12, 13, 14])).await?;
    client.lpush("key", HashSet::from([12, 13, 14])).await?;
    client.lpush("key", BTreeSet::from([12, 13, 14])).await?;

    client.lpush("key", "value1").await?;
    client.lpush("key", ["value1", "value2", "value13"]).await?;
    client.lpush("key", vec!["value1", "value2", "value13"]).await?;
    client.lpush("key", SmallVec::from(["value1", "value2", "value13"])).await?;
    client.lpush("key", HashSet::from(["value1", "value2", "value13"])).await?;
    client.lpush("key", BTreeSet::from(["value1", "value2", "value13"])).await?;

    Ok(())
}
```

### MultipleArgsCollection

Several Redis commands expect a collection with elements that will produced multiple
command arguments each

**rustis** uses the trait [`MultipleArgsCollection`](MultipleArgsCollection) to implement this behavior.

Current implementation provides the following conversions:
* `T` (for the single item case)
* `Vec<T>`
* `[T;N]`

where each of theses implementations must also implement [`IntoArgs`](IntoArgs)

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, SortedSetCommands, ZAddOptions},
    Result,
};
use std::collections::{HashSet, BTreeSet};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.zadd("key", (1.0, "member1"), ZAddOptions::default()).await?;
    client.zadd("key", [(1.0, "member1"), (2.0, "member2")], ZAddOptions::default()).await?;
    client.zadd("key", vec![(1.0, "member1"), (2.0, "member2")], ZAddOptions::default()).await?;

    Ok(())
}
```
### KeyValueArgsCollection

Several Redis commands expect one or multiple key/value pairs.

**rustis** uses the trait [`KeyValueArgsCollection`](KeyValueArgsCollection) to implement this behavior.

Current implementation provides the following conversions:
* `(K, V)` (for the single item case)
* `Vec<(K, V)>`
* `[(K, V);N]`
* `SmallVec<A>` where `A: Array<Item = (K, V)>`
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`

where each of theses implementations must also implement [`IntoArgs`](IntoArgs)

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashMap, BTreeMap};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.mset(("key1", "value1")).await?;
    client.mset([("key1", "value1"), ("key2", "value2")]).await?;
    client.mset(vec![("key1", "value1"), ("key2", "value2")]).await?;
    client.mset(SmallVec::from([("key1", "value1"), ("key2", "value2")])).await?;
    client.mset(HashMap::from([("key1", "value1"), ("key2", "value2")])).await?;
    client.mset(BTreeMap::from([("key1", "value1"), ("key2", "value2")])).await?;

    client.mset(("key1", 12)).await?;
    client.mset([("key1", 12), ("key2", 13)]).await?;
    client.mset(vec![("key1", 12), ("key2", 13)]).await?;
    client.mset(SmallVec::from([("key1", 12), ("key2", 13)])).await?;
    client.mset(HashMap::from([("key1", 12), ("key2", 13)])).await?;
    client.mset(BTreeMap::from([("key1", 12), ("key2", 13)])).await?;

    Ok(())
}
```

# Command results

**rustis** provides an idiomatic way to convert command results into Rust types.

You will notice that each built-in command returns a [`PreparedCommand<R>`](crate::client::PreparedCommand)
where `R` must implement the [`FromValue`](FromValue) trait.

This trait allows to convert the object model [`Value`](Value), freshly deserialized from RESP, into a Rust type.

Some more advanced traits allow to constraint more which Rust types are allowed.

For each trait, you can add your own implementations for your custom types
or request additional implementation for standard types.

### FromSingleValue

Several Redis commands return a single value.

**rustis** uses the trait [`FromSingleValue`](FromSingleValue) to implement this behavior.

Current implementation provides the following conversions from [`Value`](Value):
* [`Value`](Value)
* ()
* `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `usize`, `isize`,
* `f32`, `f64`,
* `bool`,
* `String`, `Vec<u8>`,
* `Option<T>`

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{FromSingleValue, FromValue, Value, deserialize_byte_buf},
    Result,
};
use serde::Deserialize;

pub struct MyI32(i32);

impl FromValue for MyI32 {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        Ok(MyI32(value.into()?))
    }
}

impl FromSingleValue for MyI32 {}

#[derive(Deserialize)]
pub struct Buffer(#[serde(deserialize_with = "deserialize_byte_buf")] pub Vec<u8>);
impl FromSingleValue for Buffer {}

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", 12).await?;
    let _result: i32 = client.get("key").await?;
    let _result: MyI32 = client.get("key").await?;

    client.set("key", 12.12).await?;
    let _result: f64 = client.get("key").await?;

    client.set("key", true).await?;
    let _result: bool = client.get("key").await?;

    client.set("key", "value").await?;
    let _result: String = client.get("key").await?;
    let _result: Buffer = client.get("key").await?;

    Ok(())
}
```

### FromValueArray

Several Redis commands return a collection of items.
**rustis** uses the trait [`FromValueArray`](FromValueArray) to implement this behavior.

Current implementation provides the following conversions from [`Value`](Value):
* `Vec<T>`
* `[T;N]`
* `SmallVec<A>`
* `BTreeSet<T>`
* `HashSet<T, S>`

where each of theses implementations must also implement [`FromValue`](FromValue)

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, ListCommands},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashSet, BTreeSet};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.lpush("key", [12, 13, 14]).await?;
    let _values: Vec<Option<i32>> = client.rpop("key", 3).await?;

    client.lpush("key", [12, 13, 14]).await?;
    let _values: HashSet<Option<i32>> = client.rpop("key", 3).await?;

    client.lpush("key", [12, 13, 14]).await?;
    let _values: BTreeSet<Option<i32>> = client.rpop("key", 3).await?;

    client.lpush("key", [12, 13, 14]).await?;
    let _values: SmallVec<[Option<i32>;3]> = client.rpop("key", 3).await?;

    Ok(())
}
```

### FromKeyValueArray

Several Redis commands return a collection of key/value pairs
**rustis** uses the trait [`FromKeyValueArray`](FromKeyValueArray) to implement this behavior.

Current implementation provides the following conversions from [`Value`](Value):
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`
* `SmallVec<A>` where `A: Array<Item = (K, V)>`
* `Vec<(K, V>)>`

where each of theses implementations must also implement [`FromValue`](FromValue)

#### Example
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, HashCommands},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashMap, BTreeMap};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    // Connect the client to a Redis server from its IP and port
    let mut client = Client::connect("127.0.0.1:6379").await?;

    // Flush all existing data in Redis
    client.flushdb(FlushingMode::Sync).await?;

    client.hset("key", [("field1", 12), ("field2", 13)]).await?;

    let _values: BTreeMap<String, i32> = client.hgetall("key").await?;
    let _values: HashMap<String, i32> = client.hgetall("key").await?;
    let _values: SmallVec<[(String, i32); 10]> = client.hgetall("key").await?;
    let _values: Vec<(String, i32)> = client.hgetall("key").await?;

    Ok(())
}
```
*/

mod buffer_decoder;
mod command;
mod command_arg;
mod command_args;
mod command_encoder;
mod from_value;
mod into_args;
mod raw_value;
mod raw_value_decoder;
mod resp_decoder;
mod resp_deserializer;
mod resp_deserializer2;
mod response;
mod util;
mod value;
mod value_decoder;
mod value_deserialize;
mod value_deserializer;

pub use buffer_decoder::*;
pub use command::*;
pub use command_arg::*;
pub use command_args::*;
pub(crate) use command_encoder::*;
pub use from_value::*;
pub use into_args::*;
pub use raw_value::*;
pub use raw_value_decoder::*;
pub use resp_decoder::*;
pub use resp_deserializer::*;
pub use resp_deserializer2::*;
pub use response::*;
pub use util::*;
pub use value::*;
pub use value_decoder::*;
pub use value_deserialize::*;
pub use value_deserializer::*;
