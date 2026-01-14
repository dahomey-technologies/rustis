use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Value, cmd},
};
use serde::Serialize;

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub(crate) trait InternalPubSubCommands<'a>: Sized {
    fn psubscribe(self, patterns: impl Serialize) -> PreparedCommand<'a, Self, Value> {
        prepare_command(self, cmd("PSUBSCRIBE").arg(patterns))
    }

    /// Unsubscribes the client from the given patterns, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/punsubscribe/>](https://redis.io/commands/punsubscribe/)            
    fn punsubscribe(self, patterns: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("PUNSUBSCRIBE").arg(patterns))
    }

    fn ssubscribe(self, shardchannels: impl Serialize) -> PreparedCommand<'a, Self, Value> {
        prepare_command(self, cmd("SSUBSCRIBE").arg(shardchannels))
    }

    fn subscribe(self, channels: impl Serialize) -> PreparedCommand<'a, Self, Value> {
        prepare_command(self, cmd("SUBSCRIBE").arg(channels))
    }

    /// Unsubscribes the client from the given shard channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sunsubscribe//>](https://redis.io/commands/sunsubscribe//)            
    fn sunsubscribe(self, shardchannels: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("SUNSUBSCRIBE").key(shardchannels))
    }

    /// Unsubscribes the client from the given channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unsubscribe/>](https://redis.io/commands/unsubscribe/)            
    fn unsubscribe(self, channels: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("UNSUBSCRIBE").arg(channels))
    }
}
