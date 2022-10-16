use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub(crate) trait InternalPubSubCommands<T>: PrepareCommand<T> {
    /// Unsubscribes the client from the given patterns, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/punsubscribe/>](https://redis.io/commands/punsubscribe/)            
    fn punsubscribe<P, PP>(&self, patterns: PP) -> CommandResult<T, ()>
    where
        P: Into<BulkString> + Send,
        PP: SingleArgOrCollection<P>,
    {
        self.prepare_command(cmd("PUNSUBSCRIBE").arg(patterns))
    }

    /// Unsubscribes the client from the given channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unsubscribe/>](https://redis.io/commands/unsubscribe/)            
    fn unsubscribe<C, CC>(&self, channels: CC) -> CommandResult<T, ()>
    where
        C: Into<BulkString>,
        CC: SingleArgOrCollection<C>,
    {
        self.prepare_command(cmd("UNSUBSCRIBE").arg(channels))
    }
}
