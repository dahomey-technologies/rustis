use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{cmd, SingleArg, SingleArgCollection, Value},
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub(crate) trait InternalPubSubCommands {
    fn psubscribe<P, PP>(&mut self, patterns: PP) -> PreparedCommand<Self, Value>
    where
        Self: Sized,
        P: SingleArg,
        PP: SingleArgCollection<P>
    {
        prepare_command(self, cmd("PSUBSCRIBE").arg(patterns))
    }

    /// Unsubscribes the client from the given patterns, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/punsubscribe/>](https://redis.io/commands/punsubscribe/)            
    fn punsubscribe<P, PP>(&mut self, patterns: PP) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        P: SingleArg + Send,
        PP: SingleArgCollection<P>,
    {
        prepare_command(self, cmd("PUNSUBSCRIBE").arg(patterns))
    }

    fn ssubscribe<C, CC>(&mut self, shardchannels: CC) -> PreparedCommand<Self, Value>
    where
        Self: Sized,
        C: SingleArg,
        CC: SingleArgCollection<C>
    {
        prepare_command(self, cmd("SSUBSCRIBE").arg(shardchannels))
    }

    fn subscribe<C, CC>(&mut self, channels: CC) -> PreparedCommand<Self, Value>
    where
        Self: Sized,
        C: SingleArg,
        CC: SingleArgCollection<C>
    {
        prepare_command(self, cmd("SUBSCRIBE").arg(channels))
    }

    /// Unsubscribes the client from the given shard channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sunsubscribe//>](https://redis.io/commands/sunsubscribe//)            
    fn sunsubscribe<C, CC>(&mut self, shardchannels: CC) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        C: SingleArg,
        CC: SingleArgCollection<C>,
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
        C: SingleArg,
        CC: SingleArgCollection<C>,
    {
        prepare_command(self, cmd("UNSUBSCRIBE").arg(channels))
    }
}
