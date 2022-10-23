use crate::{
    prepare_command,
    resp::{
        cmd, BulkString, CommandArgs, FromKeyValueValueArray, FromSingleValueArray, FromValue,
        IntoArgs, SingleArgOrCollection,
    },
    PreparedCommand,
};

/// A group of Redis commands related to [`Pub/Sub`](https://redis.io/docs/manual/pubsub/)
/// # See Also
/// [Redis Pub/Sub Commands](https://redis.io/commands/?group=pubsub)
pub trait PubSubCommands {
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
        C: Into<BulkString>,
        M: Into<BulkString>,
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
        C: Into<BulkString>,
        CC: SingleArgOrCollection<C>,
        R: FromValue,
        RR: FromKeyValueValueArray<R, usize>,
    {
        prepare_command(self, cmd("PUBSUB").arg("NUMSUB").arg(channels))
    }
}

/// Options for the [`pub_sub_channels`](crate::PubSubCommands::pub_sub_channels) command
#[derive(Default)]
pub struct PubSubChannelsOptions {
    command_args: CommandArgs,
}

impl PubSubChannelsOptions {
    pub fn pattern<P: Into<BulkString>>(self, pattern: P) -> Self {
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
