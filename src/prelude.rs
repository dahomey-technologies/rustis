/*!
The traits and types a program needs in scope, in one `use`.

Every command lives on a trait — [`StringCommands`] holds `SET`,
[`StreamCommands`] holds `XADD` — so calling one needs its trait imported, and a
program touching several families collects one import per family. This module
re-exports every command family, the two traits that carry
[`forget`](crate::client::ClientPreparedCommand::forget) and
[`queue`](crate::client::BatchPreparedCommand::queue), and the types a program
writes into its own signatures: the four executors — [`Client`],
[`ExclusiveClient`], [`Pipeline`] and [`Transaction`] — and the pub/sub surface,
[`PubSubStream`], [`PubSubSplitSink`], [`PubSubSplitStream`] and
[`PubSubMessage`].

```
use rustis::prelude::*;

# async fn example() -> rustis::Result<()> {
let client = Client::connect("127.0.0.1:6379").await?;
client.set("key", "value").await?;
let value: String = client.get("key").await?;
# Ok(())
# }
```

Four things stay behind their own paths on purpose.

[`Result`](crate::Result), because a glob import shadows the one in the standard
prelude: the two-parameter `Result<T, E>` would stop naming anything in the
importing module.

The command options and reply types under [`commands`](crate::commands), over 240
of them, of which a program names two or three. Their names would collide for no
gain: a missing one is a type error, which rustc answers with the exact `use`
line.

[`StreamExt`](https://docs.rs/futures-util/latest/futures_util/stream/trait.StreamExt.html),
which a pub/sub consumer calls `next()` from. It belongs to `futures-util`, and
re-exporting a trait from another crate would tie this one's public API to that
crate's major version.

What sets a client up or reports on it: [`Config`](crate::client::Config) and its
parts, [`ReconnectionPolicy`](crate::client::ReconnectionPolicy),
[`CredentialsProvider`](crate::client::CredentialsProvider),
[`CommandInterceptor`](crate::client::CommandInterceptor),
[`ClientStats`](crate::client::ClientStats),
[`CloseOutcome`](crate::client::CloseOutcome) and
[`MonitorStream`](crate::client::MonitorStream). Those are named where a program
configures or inspects, which is one place, rather than where it talks to Redis.
*/

pub use crate::client::{
    BatchPreparedCommand, Client, ClientPreparedCommand, ExclusiveClient, Pipeline, PubSubMessage,
    PubSubSplitSink, PubSubSplitStream, PubSubStream, Transaction,
};
pub use crate::commands::{
    ArrayCommands, BitmapCommands, BlockingCommands, BloomCommands, ClusterCommands,
    ConnectionCommands, CountMinSketchCommands, CuckooCommands, GenericCommands, GeoCommands,
    HashCommands, HyperLogLogCommands, JsonCommands, ListCommands, PubSubCommands,
    ScriptingCommands, SearchCommands, SentinelCommands, ServerCommands, SetCommands,
    SortedSetCommands, StreamCommands, StringCommands, TDigestCommands, TimeSeriesCommands,
    TopKCommands, TransactionCommands, VectorSetCommands,
};
