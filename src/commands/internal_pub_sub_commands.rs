use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    prepare_command, PreparedCommand,
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub(crate) trait InternalPubSubCommands {
    /// Unsubscribes the client from the given patterns, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/punsubscribe/>](https://redis.io/commands/punsubscribe/)            
    fn punsubscribe<P, PP>(&mut self, patterns: PP) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        P: Into<BulkString> + Send,
        PP: SingleArgOrCollection<P>,
    {
        prepare_command(self, cmd("PUNSUBSCRIBE").arg(patterns))
    }

    /// Unsubscribes the client from the given shard channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sunsubscribe//>](https://redis.io/commands/sunsubscribe//)            
    fn sunsubscribe<C, CC>(&mut self, shardchannels: CC) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        C: Into<BulkString>,
        CC: SingleArgOrCollection<C>,
    {
        prepare_command(self, cmd("SUNSUBSCRIBE").arg(shardchannels))
    }

    /// Unsubscribes the client from the given channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unsubscribe/>](https://redis.io/commands/unsubscribe/)            
    fn unsubscribe<C, CC>(&mut self, channels: CC) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        C: Into<BulkString>,
        CC: SingleArgOrCollection<C>,
    {
        prepare_command(self, cmd("UNSUBSCRIBE").arg(channels))
    }
}
