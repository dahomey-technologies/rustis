use crate::{
    client::{prepare_command, PreparedCommand, PubSubStream},
    resp::{
        cmd, CommandArg, CommandArgs, FromKeyValueValueArray, FromSingleValueArray, FromValue,
        IntoArgs, SingleArgOrCollection,
    },
    Future,
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub trait PubSubCommands {
    /// Subscribes the client to the given patterns.
    ///
    /// # Example
    /// ```
    /// use rustis::{
    ///     client::{Client, ClientPreparedCommand},
    ///     commands::{FlushingMode, PubSubCommands, ServerCommands},
    ///     resp::cmd,
    ///     Result,
    /// };
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut pub_sub_client = Client::connect("127.0.0.1:6379").await?;
    ///     let mut regular_client = Client::connect("127.0.0.1:6379").await?;
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
    /// [<https://redis.io/commands/psubscribe/>](https://redis.io/commands/psubscribe/)
    fn psubscribe<'a, P, PP>(&'a mut self, patterns: PP) -> Future<'a, PubSubStream>
    where
        P: Into<CommandArg> + Send + 'a,
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
    /// [<https://redis.io/commands/publish/>](https://redis.io/commands/publish/)
    fn publish<C, M>(&mut self, channel: C, message: M) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        C: Into<CommandArg>,
        M: Into<CommandArg>,
    {
        prepare_command(self, cmd("PUBLISH").arg(channel).arg(message))
    }

    /// Lists the currently active channels.
    ///
    /// # Return
    /// A collection of active channels, optionally matching the specified pattern.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-channels/>](https://redis.io/commands/pubsub-channels/)
    fn pub_sub_channels<C, CC>(
        &mut self,
        options: PubSubChannelsOptions,
    ) -> PreparedCommand<Self, CC>
    where
        Self: Sized,
        C: FromValue,
        CC: FromSingleValueArray<C>,
    {
        prepare_command(self, cmd("PUBSUB").arg("CHANNELS").arg(options))
    }

    /// Returns the number of unique patterns that are subscribed to by clients
    /// (that are performed using the PSUBSCRIBE command).
    ///
    /// # Return
    /// The number of patterns all the clients are subscribed to.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-numpat/>](https://redis.io/commands/pubsub-numpat/)
    fn pub_sub_numpat(&mut self) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("PUBSUB").arg("NUMPAT"))
    }

    /// Returns the number of subscribers (exclusive of clients subscribed to patterns)
    ///  for the specified channels.
    ///
    /// # Return
    /// A collection of channels and number of subscribers for every channel.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-numsub/>](https://redis.io/commands/pubsub-numsub/)
    fn pub_sub_numsub<C, CC, R, RR>(&mut self, channels: CC) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
        C: Into<CommandArg>,
        CC: SingleArgOrCollection<C>,
        R: FromValue,
        RR: FromKeyValueValueArray<R, usize>,
    {
        prepare_command(self, cmd("PUBSUB").arg("NUMSUB").arg(channels))
    }

    /// Lists the currently active shard channels.
    ///
    /// # Return
    /// A collection of active channels, optionally matching the specified pattern.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-shardchannels/>](https://redis.io/commands/pubsub-shardchannels/)
    fn pub_sub_shardchannels<C, CC>(
        &mut self,
        options: PubSubChannelsOptions,
    ) -> PreparedCommand<Self, CC>
    where
        Self: Sized,
        C: FromValue,
        CC: FromSingleValueArray<C>,
    {
        prepare_command(self, cmd("PUBSUB").arg("SHARDCHANNELS").arg(options))
    }

    /// Returns the number of subscribers for the specified shard channels.
    ///
    /// # Return
    /// A collection of channels and number of subscribers for every channel.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-shardnumsub/>](https://redis.io/commands/pubsub-shardnumsub/)
    fn pub_sub_shardnumsub<C, CC, R, RR>(&mut self, channels: CC) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
        C: Into<CommandArg>,
        CC: SingleArgOrCollection<C>,
        R: FromValue,
        RR: FromKeyValueValueArray<R, usize>,
    {
        prepare_command(self, cmd("PUBSUB").arg("SHARDNUMSUB").arg(channels))
    }

    /// Posts a message to the given shard channel.
    ///
    /// # Return
    /// The number of clients that received the message.
    ///
    /// # See Also
    /// [<https://redis.io/commands/spublish/>](https://redis.io/commands/spublish/)
    fn spublish<C, M>(&mut self, shardchannel: C, message: M) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        C: Into<CommandArg>,
        M: Into<CommandArg>,
    {
        prepare_command(self, cmd("SPUBLISH").arg(shardchannel).arg(message))
    }

    /// Subscribes the client to the specified channels.
    ///
    /// # See Also
    /// [<https://redis.io/commands/subscribe/>](https://redis.io/commands/subscribe/)
    fn ssubscribe<'a, C, CC>(&'a mut self, shardchannels: CC) -> Future<'a, PubSubStream>
    where
        C: Into<CommandArg> + Send + 'a,
        CC: SingleArgOrCollection<C>;

    /// Subscribes the client to the specified channels.
    ///
    /// # Example
    /// ```
    /// use rustis::{
    ///     client::{Client, ClientPreparedCommand}, 
    ///     commands::{FlushingMode, PubSubCommands, ServerCommands},
    ///     resp::cmd, 
    ///     Result,
    /// };
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut pub_sub_client = Client::connect("127.0.0.1:6379").await?;
    ///     let mut regular_client = Client::connect("127.0.0.1:6379").await?;
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
    /// [<https://redis.io/commands/subscribe/>](https://redis.io/commands/subscribe/)
    fn subscribe<'a, C, CC>(&'a mut self, channels: CC) -> Future<'a, PubSubStream>
    where
        C: Into<CommandArg> + Send + 'a,
        CC: SingleArgOrCollection<C>;
}

/// Options for the [`pub_sub_channels`](crate::PubSubCommands::pub_sub_channels) command
#[derive(Default)]
pub struct PubSubChannelsOptions {
    command_args: CommandArgs,
}

impl PubSubChannelsOptions {
    pub fn pattern<P: Into<CommandArg>>(self, pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg(pattern),
        }
    }
}

impl IntoArgs for PubSubChannelsOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
