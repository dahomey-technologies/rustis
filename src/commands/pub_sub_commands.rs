use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    CommandResult, Future, PrepareCommand, PubSubStream,
};

/// A redis connection used in a pub/sub scenario.
pub trait PubSubCommands<T>: PrepareCommand<T> {
    /// Posts a message to the given channel.
    ///
    /// # Return
    /// The number of clients that received the message.
    ///
    /// Note that in a Redis Cluster, only clients that are connected
    /// to the same node as the publishing client are included in the count.
    ///
    /// # See Also
    /// [https://redis.io/commands/publish/](https://redis.io/commands/publish/)
    fn publish<C, M>(&self, channel: C, message: M) -> CommandResult<T, usize>
    where
        C: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("PUBLISH").arg(channel).arg(message))
    }

    /// Subscribes the client to the specified channels.
    ///
    /// # See Also
    /// [https://redis.io/commands/subscribe/](https://redis.io/commands/subscribe/)
    fn subscribe<'a, C>(&'a self, channel: C) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + Send + 'a;

    /// Unsubscribes the client from the given channels, or from all of them if none is given.
    ///
    /// # See Also
    /// [https://redis.io/commands/unsubscribe/](https://redis.io/commands/unsubscribe/)            
    fn unsubscribe<C, CC>(&self, channels: CC) -> CommandResult<T, ()>
    where
        C: Into<BulkString>,
        CC: SingleArgOrCollection<C>,
    {
        self.prepare_command(cmd("UNSUBSCRIBE").arg(channels))
    }
}
