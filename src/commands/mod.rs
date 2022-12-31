/*!
Define Redis built-in commands in a set of traits

# Built-in commands

Because Redis offers hundreds of commands, in **rustis** commands have been split in several traits that gather commands by groups,
most of the time, groups describe in [Redis official documentation](https://redis.io/commands/).

Depending on the group of commands, traits will be implemented by [`Client`](crate::client::Client),
[`Pipeline`](crate::client::Pipeline), [`Transaction`](crate::client::Transaction) or some of these structs.

These is the list of existing command traits:
* [`BitmapCommands`](BitmapCommands): [Bitmaps](https://redis.io/docs/data-types/bitmaps/) & [Bitfields](https://redis.io/docs/data-types/bitfields/)
* [`BlockingCommands`](BlockingCommands): Commands that block the connection until the Redis server
  has a new element to send. This trait is implemented only by the [`Client`](crate::client::Client) struct.
* [`ClusterCommands`](ClusterCommands): [Redis cluster](https://redis.io/docs/reference/cluster-spec/)
* [`ConnectionCommands`](ConnectionCommands): Connection management like authentication or RESP version management
* [`GenericCommands`](GenericCommands): Generic commands like deleting, renaming or expiring keys
* [`GeoCommands`](GeoCommands): [Geospatial](https://redis.io/docs/data-types/geospatial/) indices
* [`HashCommands`](HashCommands): [Hashes](https://redis.io/docs/data-types/hashes/)
* [`HyperLogLogCommands`](HyperLogLogCommands): [HyperLogLog](https://redis.io/docs/data-types/hyperloglogs/)
* [`ListCommands`](ListCommands): [Lists](https://redis.io/docs/data-types/lists/)
* [`PubSubCommands`](PubSubCommands): [Pub/Sub](https://redis.io/docs/manual/pubsub/)
* [`ScriptingCommands`](ScriptingCommands): [Scripts](https://redis.io/docs/manual/programmability/eval-intro/) &
  [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
* [`SentinelCommands`](SentinelCommands): [Sentinel](https://redis.io/docs/management/sentinel/)
* [`ServerCommands`](ServerCommands): Server management like [Access Control Lists](https://redis.io/docs/management/security/acl/) or monitoring
* [`SetCommands`](SetCommands): [Sets](https://redis.io/docs/data-types/sets/)
* [`SortedSetCommands`](SortedSetCommands): [Sorted sets](https://redis.io/docs/data-types/sorted-sets/)
* [`StreamCommands`](StreamCommands): [Streams](https://redis.io/docs/data-types/streams/)
* [`StringCommands`](StringCommands): [Strings](https://redis.io/docs/data-types/strings/)
* [`TransactionCommands`](TransactionCommands): [Transactions](https://redis.io/docs/manual/transactions/)

Redis Stack commands:
* [`BloomCommands`](BloomCommands): [Bloom filters](https://redis.io/docs/stack/bloom/)
* [`CuckooCommands`](CuckooCommands): [Cuckoo filters](https://redis.io/docs/stack/bloom/)
* [`CountMinSketchCommands`](CountMinSketchCommands): [Count min-sketch](https://redis.io/docs/stack/bloom/)
* [`GraphCommands`](GraphCommands): [RedisGraph](https://redis.io/docs/stack/graph/)
* [`JsonCommands`](JsonCommands): [RedisJson](https://redis.io/docs/stack/json/)
* [`SearchCommands`](SearchCommands): [RedisSearch](https://redis.io/docs/stack/search/)
* [`TDigestCommands`](TDigestCommands): [T-Digest](https://redis.io/docs/stack/bloom/)
* [`TimeSeriesCommands`](TimeSeriesCommands): [Time Series](https://redis.io/docs/stack/timeseries/)
* [`TopKCommands`](TopKCommands): [Top-K](https://redis.io/docs/stack/bloom/)

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
    let mut client = Client::connect("127.0.0.1:6379").await?;

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
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
mod bloom_commands;
mod cluster_commands;
mod connection_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
mod count_min_sktech_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
mod cuckoo_commands;
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
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
mod json_commands;
mod list_commands;
mod pub_sub_commands;
mod scripting_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
mod search_commands;
mod sentinel_commands;
mod server_commands;
mod set_commands;
mod sorted_set_commands;
mod stream_commands;
mod string_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
mod t_disgest_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
mod time_series_commands;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
mod top_k_commands;
mod transaction_commands;

pub use bitmap_commands::*;
pub use blocking_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
pub use bloom_commands::*;
pub use cluster_commands::*;
pub use connection_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
pub use count_min_sktech_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
pub use cuckoo_commands::*;
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
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
pub use json_commands::*;
pub use list_commands::*;
pub use pub_sub_commands::*;
pub use scripting_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
pub use search_commands::*;
pub use sentinel_commands::*;
pub use server_commands::*;
pub use set_commands::*;
pub use sorted_set_commands::*;
pub use stream_commands::*;
pub use string_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
pub use t_disgest_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
pub use time_series_commands::*;
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
pub use top_k_commands::*;
pub use transaction_commands::*;
