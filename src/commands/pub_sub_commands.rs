use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    CommandResult, Future, PrepareCommand, PubSubStream,
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub trait PubSubCommands<T>: PrepareCommand<T> {
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
    ///     regular_client.flushdb(FlushingMode::Sync).await?;
    ///
    ///     let mut pub_sub_stream = pub_sub_client.psubscribe("mychannel*").await?;
    ///
    ///     regular_client.publish("mychannel1", "mymessage").await?;
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
    ///     regular_client.flushdb(FlushingMode::Sync).await?;
    ///
    ///     let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    ///
    ///     regular_client.publish("mychannel", "mymessage").await?;
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
}
