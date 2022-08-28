use crate::{resp::BulkString, PubSub, PubSubStream, Result, cmd};
use async_trait::async_trait;

/// A redis connection used in a pub/sub scenario.
#[async_trait]
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
    async fn publish<C, M>(&self, channel: C, message: M) -> Result<usize>
    where
        C: Into<BulkString> + Send,
        M: Into<BulkString> + Send;

    /// Subscribes the client to the specified channels.
    ///
    /// # See Also
    /// [https://redis.io/commands/subscribe/](https://redis.io/commands/subscribe/)
    async fn subscribe<C>(&self, channel: C) -> Result<PubSubStream>
    where
        C: Into<BulkString> + Send;
}

#[async_trait]
impl PubSubCommands for PubSub {
    async fn publish<C, M>(&self, channel: C, message: M) -> Result<usize>
    where
        C: Into<BulkString> + Send,
        M: Into<BulkString> + Send,
    {
        self.multiplexer
            .send(0, cmd("PUBLISH").arg(channel).arg(message))
            .await?
            .into()
    }

    async fn subscribe<C>(&self, channel: C) -> Result<PubSubStream>
    where
        C: Into<BulkString> + Send,
    {
        self.multiplexer.subscribe(channel.into()).await
    }
}
