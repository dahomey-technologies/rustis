use crate::{cmd, resp::BulkString, Future, PubSub, PubSubStream};

/// A redis connection used in a pub/sub scenario.
pub trait PubSubCommands {
    /// Posts a message to the given channel.
    ///
    /// # Return
    /// Integer reply: the number of clients that received the message.
    /// Note that in a Redis Cluster, only clients that are connected
    /// to the same node as the publishing client are included in the count.
    ///
    /// # See Also
    /// [https://redis.io/commands/publish/](https://redis.io/commands/publish/)
    fn publish<'a, C, M>(&'a self, channel: C, message: M) -> Future<'a, usize>
    where
        C: Into<BulkString> + 'a + Send,
        M: Into<BulkString> + 'a + Send;

    /// Subscribes the client to the specified channels.
    ///
    /// # See Also
    /// [https://redis.io/commands/subscribe/](https://redis.io/commands/subscribe/)
    fn subscribe<'a, C>(&'a self, channel: C) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + 'a + Send;
}

impl PubSubCommands for PubSub {
    fn publish<'a, C, M>(&'a self, channel: C, message: M) -> Future<'a, usize>
    where
        C: Into<BulkString> + 'a + Send,
        M: Into<BulkString> + 'a + Send,
    {
        Box::pin(async move {
            self.multiplexer
                .send(0, cmd("PUBLISH").arg(channel).arg(message))
                .await?
                .into()
        })
    }

    fn subscribe<'a, C>(&'a self, channel: C) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + 'a + Send,
    {
        Box::pin(async move { self.multiplexer.subscribe(channel.into()).await })
    }
}
