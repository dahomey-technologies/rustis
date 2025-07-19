use crate::{
    Result,
    client::{PreparedCommand, PubSubStream, prepare_command},
    resp::{
        CollectionResponse, CommandArgs, KeyValueCollectionResponse, PrimitiveResponse, SingleArg,
        SingleArgCollection, ToArgs, cmd,
    },
};
use serde::de::DeserializeOwned;

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub trait PubSubCommands<'a> {
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
    /// use futures_util::StreamExt;
    ///
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
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
    ///     let message = pub_sub_stream.next().await.unwrap()?;
    ///     assert_eq!(b"mychannel*".to_vec(), message.pattern);
    ///     assert_eq!(b"mychannel1".to_vec(), message.channel);
    ///     assert_eq!(b"mymessage".to_vec(), message.payload);
    ///
    ///     pub_sub_stream.close().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/psubscribe/>](https://redis.io/commands/psubscribe/)
    #[allow(async_fn_in_trait)]
    async fn psubscribe<P, PP>(self, patterns: PP) -> Result<PubSubStream>
    where
        P: SingleArg + Send + 'a,
        PP: SingleArgCollection<P>;

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
    fn publish<C, M>(self, channel: C, message: M) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        C: SingleArg,
        M: SingleArg,
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
        self,
        options: PubSubChannelsOptions,
    ) -> PreparedCommand<'a, Self, CC>
    where
        Self: Sized,
        C: PrimitiveResponse + DeserializeOwned,
        CC: CollectionResponse<C>,
    {
        prepare_command(self, cmd("PUBSUB").arg("CHANNELS").arg(options))
    }

    /// The command returns a helpful text describing the different PUBSUB subcommands.
    ///
    /// # Return
    /// An array of strings.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::PubSubCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// # #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// # async fn main() -> Result<()> {
    /// #     let client = Client::connect("127.0.0.1:6379").await?;
    /// let result: Vec<String> = client.pub_sub_help().await?;
    /// assert!(result.iter().any(|e| e == "HELP"));
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-help/>](https://redis.io/commands/pubsub-help/)
    #[must_use]
    fn pub_sub_help(self) -> PreparedCommand<'a, Self, Vec<String>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("PUBSUB").arg("HELP"))
    }

    /// Returns the number of unique patterns that are subscribed to by clients
    /// (that are performed using the PSUBSCRIBE command).
    ///
    /// # Return
    /// The number of patterns all the clients are subscribed to.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pubsub-numpat/>](https://redis.io/commands/pubsub-numpat/)
    fn pub_sub_numpat(self) -> PreparedCommand<'a, Self, usize>
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
    fn pub_sub_numsub<C, CC, R, RR>(self, channels: CC) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        C: SingleArg,
        CC: SingleArgCollection<C>,
        R: PrimitiveResponse,
        RR: KeyValueCollectionResponse<R, usize>,
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
        self,
        options: PubSubChannelsOptions,
    ) -> PreparedCommand<'a, Self, CC>
    where
        Self: Sized,
        C: PrimitiveResponse + DeserializeOwned,
        CC: CollectionResponse<C>,
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
    fn pub_sub_shardnumsub<C, CC, R, RR>(self, channels: CC) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        C: SingleArg,
        CC: SingleArgCollection<C>,
        R: PrimitiveResponse,
        RR: KeyValueCollectionResponse<R, usize>,
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
    fn spublish<C, M>(self, shardchannel: C, message: M) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        C: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("SPUBLISH").arg(shardchannel).arg(message))
    }

    /// Subscribes the client to the specified channels.
    ///
    /// # See Also
    /// [<https://redis.io/commands/subscribe/>](https://redis.io/commands/subscribe/)
    #[allow(async_fn_in_trait)]
    async fn ssubscribe<C, CC>(self, shardchannels: CC) -> Result<PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgCollection<C>;

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
    /// use futures_util::StreamExt;
    ///
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
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
    ///     let message = pub_sub_stream.next().await.unwrap()?;
    ///     assert_eq!(b"mychannel".to_vec(), message.channel);
    ///     assert_eq!(b"mymessage".to_vec(), message.payload);
    ///
    ///     pub_sub_stream.close().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/subscribe/>](https://redis.io/commands/subscribe/)
    #[allow(async_fn_in_trait)]
    async fn subscribe<C, CC>(self, channels: CC) -> Result<PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgCollection<C>;
}

/// Options for the [`pub_sub_channels`](PubSubCommands::pub_sub_channels) command
#[derive(Default)]
pub struct PubSubChannelsOptions {
    command_args: CommandArgs,
}

impl PubSubChannelsOptions {
    pub fn pattern<P: SingleArg>(mut self, pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg(pattern).build(),
        }
    }
}

impl ToArgs for PubSubChannelsOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}
