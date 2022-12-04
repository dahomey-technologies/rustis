/*!
Defines types related to the [`RESP`](https://redis.io/docs/reference/protocol-spec/) protocol and their encoding/decoding

# Object Model

**rustis** provides an object model in the form of a generic data struct, comparable to the XML DOM,
and which matches perfectly the RESP protocol: the enum [`resp::Value`](Value).

Each variant of this enum matches a [`RESP`](https://redis.io/docs/reference/protocol-spec/) type.

Because, navigating through a [`resp::Value`](Value) instance can be verbose and requires a lot of pattern matching,
** rustis** provides:
* Rust type to [`Command`](Command) conversion, with the trait [IntoArgs](IntoArgs).
* [`resp::Value`](Value) to Rust type conversion, with the trait [FromValue](FromValue).

# Command arguments

** rustis** provides an idiomatic way to pass arguments to [commands](crate::commands).
Basically a [`Command`](Command) is a collection of [`CommandArg`](CommandArg)

You will notice that each built-in command expects arguments through a set of traits defined in this module.

For each trait, you can add your own implementations for your custom types
or request additional implementation for standard types.

### Into\<CommandArg\>/From\<CommandArg\>

These traits let the caller transform any primitive rust type into a CommandArg.

Current implementation provides the following conversions: `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`,
`i64`, `usize`, `isize`, `f32`, `f64`, `bool`, `char`, `String`, `&str` and [`BulkString`](BulkString)

Example:
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{BulkString, CommandArg},
    Result,
};

pub struct MyI32(i32);

impl From<MyI32> for CommandArg {
    #[inline]
    fn from(i: MyI32) -> Self {
        Self::Signed(i.0 as i64)
    }
}

#[tokio::main]
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
    client.set("key", BulkString(b"value".to_vec())).await?;
    client.set("key", MyI32(12)).await?;

    Ok(())
}
```

### IntoArgs

The trait [`IntoArgs`](IntoArgs) allows to convert a complex type into one ore multiple argumentss.
Basically, the conversion function can add multiple arguments to an existing argument collection: the [`CommandArgs`](CommandArgs) struct.

Current implementation provides the following conversions:
* `(T, U)`
* `(T, U, V)`
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`
* `BTreeSet<T>`
* `HashSet<T, S>`
* `Option<T>`
* `SmallVec<A>`
* `Vec<T>`
* `[T;N]`

Nevertheless, [`IntoArgs`](IntoArgs) is not expected directly in built-in commands arguments.

The following traits are used to constraints which implementations of [`IntoArgs`](IntoArgs)
are expected by a specific argument of a built-in command

### SingleArgOrCollection

Several Redis commands expect one or multiple items, elements, values of the same type.
**rustis** uses the trait [`SingleArgOrCollection`](SingleArgOrCollection) to implement this behavior.

Current implementation provides the following conversions:
* `T` (for the single item case)
* `BTreeSet<T>`
* `HashSet<T, S>`
* `SmallVec<A>`
* `Vec<T>`
* `[T;N]`
where each of theses implementations must also implement [`IntoArgs`](IntoArgs)

Example:
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, ListCommands},
    resp::{BulkString, CommandArg},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashSet, BTreeSet};

#[tokio::main]
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

### KeyValueArgOrCollection

Several Redis commands expect one or multiple key/value pairs.
**rustis** uses the trait [`KeyValueArgOrCollection`](KeyValueArgOrCollection) to implement this behavior.

Current implementation provides the following conversions:
* `(K, V)` (for the single item case)
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`
* `Vec<(K, V)>`
* `[(K, V);N]`
where each of theses implementations must also implement [`IntoArgs`](IntoArgs)

Example:
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{BulkString, CommandArg},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashMap, BTreeMap};

#[tokio::main]
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

** rustis** provides an idiomatic way to convert command results into Rust types.

You will notice that each built-in command returns a [`PreparedCommand<R>`](crate::client::PreparedCommand)
where `R` must implement the [`FromValue`](FromValue) trait.

This trait allows to convert the object model [`Value`](Value), freshly deserialized from RESP, into a Rust type.

Some more advanced traits allow to constraint more which Rust types are allowed.

For each trait, you can add your own implementations for your custom types
or request additional implementation for standard types.

### FromValue

Current implementation provides the following conversions from [`Value`](Value):
* `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `usize`, `isize`,
* `f32`, `f64`,
* `bool`,
* `String`, [`BulkString`](BulkString)
* Option<T>
* Tuples, up to 10 members
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`
* `BTreeSet<T>`
* `HashSet<T, S>`
* `SmallVec<A>`
* `Vec<T>`
* `[T;N]`

Example:
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{BulkString, FromValue, Value},
    Result,
};

pub struct MyI32(i32);

impl FromValue for MyI32 {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        Ok(MyI32(value.into()?))
    }
}

#[tokio::main]
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
    let _result: BulkString = client.get("key").await?;

    Ok(())
}
```

### FromSingleValueArray

Several Redis commands return a collection of items
**rustis** uses the trait [`FromSingleValueArray`](FromSingleValueArray) to implement this behavior.

Current implementation provides the following conversions from [`Value`](Value):
* `BTreeSet<T>`
* `HashSet<T, S>`
* `SmallVec<A>`
* `Vec<T>`
* `[T;N]`
where each of theses implementations must also implement [`FromValue`](FromValue)

Example:
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, ListCommands},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashSet, BTreeSet};

#[tokio::main]
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

### FromSingleValueArray

Several Redis commands return a collection of key/value pairs
**rustis** uses the trait [`FromKeyValueValueArray`](FromKeyValueValueArray) to implement this behavior.

Current implementation provides the following conversions from [`Value`](Value):
* `BTreeMap<K, V>`
* `HashMap<K, V, S>`
* `SmallVec<A>` where `A: Array<Item = (K, V)>`
* `Vec<(K, V>)>`
where each of theses implementations must also implement [`FromValue`](FromValue)

Example:
```
use rustis::{
    client::Client,
    commands::{FlushingMode, ServerCommands, HashCommands},
    resp::{BulkString, CommandArg},
    Result,
};
use smallvec::{SmallVec};
use std::collections::{HashMap, BTreeMap};

#[tokio::main]
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

mod command;
mod command_arg;
mod command_args;
mod command_encoder;
mod from_value;
mod from_value_tuple;
mod into_args;
mod value;
mod value_decoder;

pub use command::*;
pub use command_arg::*;
pub use command_args::*;
pub(crate) use command_encoder::*;
pub use from_value::*;
pub use from_value_tuple::*;
pub use into_args::*;
pub use value::*;
pub(crate) use value_decoder::*;
