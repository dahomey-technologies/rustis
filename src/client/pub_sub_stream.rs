use crate::{
    client::{Client, ClientPreparedCommand},
    commands::InternalPubSubCommands,
    network::PubSubSender,
    resp::{ByteBufSeed, CommandArgs, SingleArg, SingleArgCollection},
    PubSubReceiver, Result,
};
use futures::{Stream, StreamExt};
use serde::{
    de::{self, Visitor},
    Deserialize,
};
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

/// Pub/Sub Message that can be streamed from [`PubSubStream`](PubSubStream)
#[derive(Debug)]
pub struct PubSubMessage {
    pub pattern: Vec<u8>,
    pub channel: Vec<u8>,
    pub payload: Vec<u8>,
}

impl<'de> Deserialize<'de> for PubSubMessage {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PubSubMessageVisitor;

        impl<'de> Visitor<'de> for PubSubMessageVisitor {
            type Value = PubSubMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("PubSubMessage")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let Some(kind) = seq.next_element::<&str>()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let Ok(Some(channel_or_pattern)) = seq.next_element_seed(ByteBufSeed) else {
                    return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                };

                let Ok(Some(channel_or_payload)) = seq.next_element_seed(ByteBufSeed) else {
                    return Err(de::Error::invalid_length(2, &"more elements in sequence"));
                };

                match kind {
                    "message" | "smessage" => Ok(PubSubMessage {
                        pattern: vec![],
                        channel: channel_or_pattern,
                        payload: channel_or_payload,
                    }),
                    "pmessage" => {
                        let Ok(Some(payload)) = seq.next_element_seed(ByteBufSeed) else {
                            return Err(de::Error::invalid_length(3, &"more elements in sequence"));
                        };

                        Ok(PubSubMessage {
                            pattern: channel_or_pattern,
                            channel: channel_or_payload,
                            payload,
                        })
                    }
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(kind),
                        &"message, smessage or pmessage",
                    )),
                }
            }
        }

        deserializer.deserialize_seq(PubSubMessageVisitor)
    }
}

/// Stream to get messages from the channels or patterns [`subscribed`](https://redis.io/docs/manual/pubsub/) to
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
/// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
/// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
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
///     let mut message = pub_sub_stream.next().await.unwrap()?;
///     assert_eq!(b"mychannel".to_vec(), message.channel);
///     assert_eq!(b"mymessage".to_vec(), message.payload);
///
///     pub_sub_stream.close().await?;
///
///     Ok(())
/// }
/// ```
pub struct PubSubStream {
    closed: bool,
    channels: CommandArgs,
    patterns: CommandArgs,
    shardchannels: CommandArgs,
    sender: PubSubSender,
    receiver: PubSubReceiver,
    client: Client,
}

impl PubSubStream {
    pub(crate) fn from_channels(
        channels: CommandArgs,
        sender: PubSubSender,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        Self {
            closed: false,
            channels,
            patterns: CommandArgs::default(),
            shardchannels: CommandArgs::default(),
            sender,
            receiver,
            client,
        }
    }

    pub(crate) fn from_patterns(
        patterns: CommandArgs,
        sender: PubSubSender,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        Self {
            closed: false,
            channels: CommandArgs::default(),
            patterns,
            shardchannels: CommandArgs::default(),
            sender,
            receiver,
            client,
        }
    }

    pub(crate) fn from_shardchannels(
        shardchannels: CommandArgs,
        sender: PubSubSender,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        Self {
            closed: false,
            channels: CommandArgs::default(),
            patterns: CommandArgs::default(),
            shardchannels,
            sender,
            receiver,
            client,
        }
    }

    /// Subscribe to additional channels
    pub async fn subscribe<C, CC>(&mut self, channels: CC) -> Result<()>
    where
        C: SingleArg + Send,
        CC: SingleArgCollection<C>,
    {
        let channels = CommandArgs::default().arg(channels).build();

        self.client
            .subscribe_from_pub_sub_sender(&channels, &self.sender)
            .await?;

        let mut existing_channels = CommandArgs::default();
        std::mem::swap(&mut existing_channels, &mut self.channels);
        self.channels = existing_channels.arg(channels).build();

        Ok(())
    }

    /// Subscribe to additional patterns
    pub async fn psubscribe<P, PP>(&mut self, patterns: PP) -> Result<()>
    where
        P: SingleArg + Send,
        PP: SingleArgCollection<P>,
    {
        let patterns = CommandArgs::default().arg(patterns).build();

        self.client
            .psubscribe_from_pub_sub_sender(&patterns, &self.sender)
            .await?;

        let mut existing_patterns = CommandArgs::default();
        std::mem::swap(&mut existing_patterns, &mut self.patterns);
        self.patterns = existing_patterns.arg(patterns).build();

        Ok(())
    }

    /// Subscribe to additional shardchannels
    pub async fn ssubscribe<C, CC>(&mut self, shardchannels: CC) -> Result<()>
    where
        C: SingleArg + Send,
        CC: SingleArgCollection<C>,
    {
        let shardchannels = CommandArgs::default().arg(shardchannels).build();

        self.client
            .ssubscribe_from_pub_sub_sender(&shardchannels, &self.sender)
            .await?;

        let mut existing_shardchannels = CommandArgs::default();
        std::mem::swap(&mut existing_shardchannels, &mut self.shardchannels);
        self.shardchannels = existing_shardchannels.arg(shardchannels).build();

        Ok(())
    }

    /// Close the stream by cancelling all subscriptions
    /// Calling `close` allows to wait for all the unsubscriptions.
    /// `drop` will achieve the same process but silently in background
    pub async fn close(mut self) -> Result<()> {
        let mut channels = CommandArgs::default();
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            self.client.unsubscribe(channels).await?;
        }

        let mut patterns = CommandArgs::default();
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            self.client.punsubscribe(patterns).await?;
        }

        let mut shardchannels = CommandArgs::default();
        std::mem::swap(&mut shardchannels, &mut self.shardchannels);
        if !shardchannels.is_empty() {
            self.client.sunsubscribe(shardchannels).await?;
        }

        self.closed = true;

        Ok(())
    }
}

impl Stream for PubSubStream {
    type Item = Result<PubSubMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.closed {
            Poll::Ready(None)
        } else {
            match self.get_mut().receiver.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(message))) => Poll::Ready(Some(message.to())),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

impl Drop for PubSubStream {
    /// Cancel all subscriptions before dropping
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        let mut channels = CommandArgs::default();
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            let _result = self.client.unsubscribe(channels).forget();
        }

        let mut patterns = CommandArgs::default();
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            let _result = self.client.punsubscribe(patterns).forget();
        }

        let mut shardchannels = CommandArgs::default();
        std::mem::swap(&mut shardchannels, &mut self.shardchannels);
        if !shardchannels.is_empty() {
            let _result = self.client.sunsubscribe(shardchannels).forget();
        }
    }
}
