use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    CommandResult, Future, PrepareCommand, PubSubStream,
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
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
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     resp::cmd, Client, ClientCommandResult, FlushingMode,
    ///     PubSubCommands, ServerCommands, Result
    /// };
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let pub_sub_client = Client::connect("127.0.0.1:6379").await?;
    ///     let regular_client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     regular_client.flushdb(FlushingMode::Sync).send().await?;
    ///
    ///     let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    /// 
    ///     regular_client
    ///         .publish("mychannel", "mymessage")
    ///         .send()
    ///         .await?;
    ///
    ///     let (channel, message): (String, String) = pub_sub_stream
    ///         .next()
    ///         .await
    ///         .unwrap()?
    ///         .into()?;
    /// 
    ///     assert_eq!("mychannel", channel);
    ///     assert_eq!("mymessage", message);
    /// 
    ///     pub_sub_stream.close().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [https://redis.io/commands/subscribe/](https://redis.io/commands/subscribe/)
    fn subscribe<'a, C, CC>(&'a self, channels: CC) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + Send + 'a,
        CC: SingleArgOrCollection<C>;

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

    /// Subscribes the client to the given patterns.
    ///
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     resp::cmd, Client, ClientCommandResult, FlushingMode,
    ///     PubSubCommands, ServerCommands, Result
    /// };
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let pub_sub_client = Client::connect("127.0.0.1:6379").await?;
    ///     let regular_client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     regular_client.flushdb(FlushingMode::Sync).send().await?;
    ///
    ///     let mut pub_sub_stream = pub_sub_client.psubscribe("mychannel*").await?;
    /// 
    ///     regular_client
    ///         .publish("mychannel1", "mymessage")
    ///         .send()
    ///         .await?;
    ///
    ///     let (pattern, channel, message): (String, String, String) = pub_sub_stream
    ///         .next()
    ///         .await
    ///         .unwrap()?
    ///         .into()?;
    /// 
    ///     assert_eq!("mychannel*", pattern);
    ///     assert_eq!("mychannel1", channel);
    ///     assert_eq!("mymessage", message);
    /// 
    ///     pub_sub_stream.close().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    /// 
    /// # See Also
    /// [https://redis.io/commands/psubscribe/](https://redis.io/commands/psubscribe/)
    fn psubscribe<'a, P, PP>(&'a self, patterns: PP) -> Future<'a, PubSubStream>
    where
        P: Into<BulkString> + Send + 'a,
        PP: SingleArgOrCollection<P>;

    /// Unsubscribes the client from the given patterns, or from all of them if none is given.
    ///
    /// # See Also
    /// [https://redis.io/commands/punsubscribe/](https://redis.io/commands/punsubscribe/)            
    fn punsubscribe<P, PP>(&self, patterns: PP) -> CommandResult<T, ()>
    where
        P: Into<BulkString> + Send,
        PP: SingleArgOrCollection<P>
    {
        self.prepare_command(cmd("PUNSUBSCRIBE").arg(patterns))
    }
}
