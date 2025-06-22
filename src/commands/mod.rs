/*!
Define Redis built-in commands in a set of traits

# Built-in commands

Because Redis offers hundreds of commands, in **rustis** commands have been split in several traits that gather commands by groups,
most of the time, groups describe in [Redis official documentation](https://redis.io/commands/).

Depending on the group of commands, traits will be implemented by [`Client`](crate::client::Client),
[`Pipeline`](crate::client::Pipeline), [`Transaction`](crate::client::Transaction) or some of these structs.

These is the list of existing command traits:
* [`BitmapCommands`]: [Bitmaps](https://redis.io/docs/data-types/bitmaps/) & [Bitfields](https://redis.io/docs/data-types/bitfields/)
* [`BlockingCommands`]: Commands that block the connection until the Redis server
  has a new element to send. This trait is implemented only by the [`Client`](crate::client::Client) struct.
* [`ClusterCommands`]: [Redis cluster](https://redis.io/docs/reference/cluster-spec/)
* [`ConnectionCommands`]: Connection management like authentication or RESP version management
* [`GenericCommands`]: Generic commands like deleting, renaming or expiring keys
* [`GeoCommands`]: [Geospatial](https://redis.io/docs/data-types/geospatial/) indices
* [`HashCommands`](crate::commands::HashCommands): [Hashes](https://redis.io/docs/data-types/hashes/)
* [`HyperLogLogCommands`](crate::commands::HyperLogLogCommands): [HyperLogLog](https://redis.io/docs/data-types/hyperloglogs/)
* [`ListCommands`](crate::commands::ListCommands): [Lists](https://redis.io/docs/data-types/lists/)
* [`PubSubCommands`](crate::commands::PubSubCommands): [Pub/Sub](https://redis.io/docs/manual/pubsub/)
* [`ScriptingCommands`](crate::commands::ScriptingCommands): [Scripts](https://redis.io/docs/manual/programmability/eval-intro/) &
  [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
* [`SentinelCommands`](crate::commands::SentinelCommands): [Sentinel](https://redis.io/docs/management/sentinel/)
* [`ServerCommands`](crate::commands::ServerCommands): Server management like [Access Control Lists](https://redis.io/docs/management/security/acl/) or monitoring
* [`SetCommands`](crate::commands::SetCommands): [Sets](https://redis.io/docs/data-types/sets/)
* [`SortedSetCommands`](crate::commands::SortedSetCommands): [Sorted sets](https://redis.io/docs/data-types/sorted-sets/)
* [`StreamCommands`](crate::commands::StreamCommands): [Streams](https://redis.io/docs/data-types/streams/)
* [`StringCommands`](crate::commands::StringCommands): [Strings](https://redis.io/docs/data-types/strings/)
* [`VectorSetCommands`](crate::commands::VectorSetCommands): [Vector sets](https://redis.io/docs/data-types/vector-sets/)
* [`TransactionCommands`](crate::commands::TransactionCommands): [Transactions](https://redis.io/docs/manual/transactions/)
* [`BloomCommands`](crate::commands::BloomCommands): [Bloom filters](https://redis.io/docs/stack/bloom/)
* [`CuckooCommands`](crate::commands::CuckooCommands): [Cuckoo filters](https://redis.io/docs/stack/bloom/)
* [`CountMinSketchCommands`](crate::commands::CountMinSketchCommands): [Count min-sketch](https://redis.io/docs/stack/bloom/)
* [`JsonCommands`](crate::commands::JsonCommands): [RedisJson](https://redis.io/docs/stack/json/)
* [`SearchCommands`](crate::commands::SearchCommands): [RedisSearch](https://redis.io/docs/stack/search/)
* [`TDigestCommands`](crate::commands::TDigestCommands): [T-Digest](https://redis.io/docs/stack/bloom/)
* [`TimeSeriesCommands`](crate::commands::TimeSeriesCommands): [Time Series](https://redis.io/docs/stack/timeseries/)
* [`TopKCommands`](crate::commands::TopKCommands): [Top-K](https://redis.io/docs/stack/bloom/)
* [`GraphCommands`](crate::commands::GraphCommands): [RedisGraph](https://redis.io/docs/stack/graph/)

# Example

To use a command, simply add the related trait to your `use` declerations
and call the related associated function directly to a client, pipeline, transaction instance.

Commands can be directly awaited or [forgotten](crate::client::ClientPreparedCommand::forget).

```
use rustis::{
    client::{Client, ClientPreparedCommand},
    commands::{ListCommands, SortedSetCommands, ZAddOptions},
    Result,
};

#[cfg_attr(feature = "tokio-runtime", tokio::main)]
#[cfg_attr(feature = "async-std-runtime", async_std::main)]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    // Send & await ListCommands::lpush command
    let _size = client.lpush("mylist", ["element1", "element2"]).await?;

    // Send & forget SortedSetCommands::zadd command
    let _size = client.zadd(
        "mySortedSet",
        [(1.0, "member1"), (2.0, "member2")],
        ZAddOptions::default()
    ).forget();

    Ok(())
}
```

# Documentation disclaimer

The commands traits documentation is directly adapated from the official Redis
documentation found [here](https://github.com/redis/redis-doc) with the
following [COPYRIGHT](https://github.com/redis/redis-doc/blob/master/COPYRIGHT).
*/

mod bitmap_commands;
mod blocking_commands;
mod bloom_commands;
mod cluster_commands;
mod connection_commands;
mod count_min_sktech_commands;
mod cuckoo_commands;
#[cfg(test)]
mod debug_commands;
mod generic_commands;
mod geo_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
mod graph_cache;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
mod graph_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
mod graph_value;
mod hash_commands;
mod hyper_log_log_commands;
mod internal_pub_sub_commands;
mod json_commands;
mod list_commands;
mod pub_sub_commands;
mod scripting_commands;
mod search_commands;
mod sentinel_commands;
mod server_commands;
mod set_commands;
mod sorted_set_commands;
mod stream_commands;
mod string_commands;
mod t_disgest_commands;
mod time_series_commands;
mod top_k_commands;
mod transaction_commands;
mod vector_set;

pub use bitmap_commands::*;
pub use blocking_commands::*;
pub use bloom_commands::*;
pub use cluster_commands::*;
pub use connection_commands::*;
pub use count_min_sktech_commands::*;
pub use cuckoo_commands::*;
#[cfg(test)]
pub use debug_commands::*;
pub use generic_commands::*;
pub use geo_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
pub(crate) use graph_cache::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
pub use graph_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
pub use graph_value::*;
pub use hash_commands::*;
pub use hyper_log_log_commands::*;
pub(crate) use internal_pub_sub_commands::*;
pub use json_commands::*;
pub use list_commands::*;
pub use pub_sub_commands::*;
pub use scripting_commands::*;
pub use search_commands::*;
pub use sentinel_commands::*;
pub use server_commands::*;
pub use set_commands::*;
pub use sorted_set_commands::*;
pub use stream_commands::*;
pub use string_commands::*;
pub use t_disgest_commands::*;
pub use time_series_commands::*;
pub use top_k_commands::*;
pub use transaction_commands::*;
pub use vector_set::*;
