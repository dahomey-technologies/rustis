use crate::{
    ClientError, Error, PubSubReceiver, Result,
    client::{Client, ClientPreparedCommand},
    commands::InternalPubSubCommands,
    network::PubSubSender,
    resp::{ByteBufSeed, CommandArgs, CommandArgsMut},
};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use std::{
    collections::HashSet,
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

fn extract_args_to_set(args: CommandArgs, set: &mut HashSet<Bytes>) {
    for arg in &args {
        set.insert(arg);
    }
}

/// A pub sub `Sink` part of the [`split`](PubSubStream::split) pair.
/// It allows to subscribe/unsubscribe to/from channels or patterns
pub struct PubSubSplitSink {
    closed: bool,
    channels: HashSet<Bytes>,
    patterns: HashSet<Bytes>,
    shardchannels: HashSet<Bytes>,
    sender: PubSubSender,
    client: Client,
}

impl PubSubSplitSink {
    /// Subscribe to additional channels
    pub async fn subscribe(&mut self, channels: impl Serialize) -> Result<()> {
        let channels = CommandArgsMut::default().arg(channels).freeze();

        for channel in &channels {
            if self.channels.contains(&channel) {
                return Err(Error::Client(ClientError::AlreadySubscribed));
            }
        }

        self.client
            .subscribe_from_pub_sub_sender(&channels, &self.sender)
            .await?;

        extract_args_to_set(channels, &mut self.channels);

        Ok(())
    }

    /// Subscribe to additional patterns
    pub async fn psubscribe(&mut self, patterns: impl Serialize) -> Result<()> {
        let patterns = CommandArgsMut::default().arg(patterns).freeze();

        for pattern in &patterns {
            if self.patterns.contains(&pattern) {
                return Err(Error::Client(ClientError::AlreadySubscribed));
            }
        }

        self.client
            .psubscribe_from_pub_sub_sender(&patterns, &self.sender)
            .await?;

        extract_args_to_set(patterns, &mut self.patterns);

        Ok(())
    }

    /// Subscribe to additional shardchannels
    pub async fn ssubscribe(&mut self, shardchannels: impl Serialize) -> Result<()> {
        let shardchannels = CommandArgsMut::default().arg(shardchannels).freeze();

        for shardchannel in &shardchannels {
            if self.shardchannels.contains(&shardchannel) {
                return Err(Error::Client(ClientError::AlreadySubscribed));
            }
        }

        self.client
            .ssubscribe_from_pub_sub_sender(&shardchannels, &self.sender)
            .await?;

        extract_args_to_set(shardchannels, &mut self.shardchannels);
        Ok(())
    }

    /// Unsubscribe from the given channels
    pub async fn unsubscribe(&mut self, channels: impl Serialize) -> Result<()> {
        let channels = CommandArgsMut::default().arg(channels).freeze();

        for channel in &channels {
            self.channels.remove(&channel);
        }

        self.client.unsubscribe(channels).await?;

        Ok(())
    }

    /// Unsubscribe from the given patterns
    pub async fn punsubscribe(&mut self, patterns: impl Serialize) -> Result<()> {
        let patterns = CommandArgsMut::default().arg(patterns).freeze();

        for pattern in &patterns {
            self.patterns.remove(&pattern);
        }

        self.client.punsubscribe(patterns).await?;

        Ok(())
    }

    /// Unsubscribe from the given patterns
    pub async fn sunsubscribe(&mut self, shardchannels: impl Serialize) -> Result<()> {
        let shardchannels = CommandArgsMut::default().arg(shardchannels).freeze();

        for shardchannel in &shardchannels {
            self.shardchannels.remove(&shardchannel);
        }

        self.client.sunsubscribe(shardchannels).await?;

        Ok(())
    }

    /// Close the stream by cancelling all subscriptions
    /// Calling `close` allows to wait for all the unsubscriptions.
    /// `drop` will achieve the same process but silently in background
    pub async fn close(mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }

        if !self.channels.is_empty() {
            let mut args = CommandArgsMut::default();
            for channel in &self.channels {
                args = args.arg(channel);
            }
            self.client.unsubscribe(args).await?;
            self.channels.clear();
        }

        if !self.patterns.is_empty() {
            let mut args = CommandArgsMut::default();
            for pattern in &self.patterns {
                args = args.arg(pattern);
            }
            self.client.punsubscribe(args).await?;
            self.patterns.clear();
        }

        if !self.shardchannels.is_empty() {
            let mut args = CommandArgsMut::default();
            for shardchannel in &self.shardchannels {
                args = args.arg(shardchannel);
            }
            self.client.sunsubscribe(args).await?;
            self.shardchannels.clear();
        }

        self.closed = true;

        Ok(())
    }
}

impl Drop for PubSubSplitSink {
    /// Cancel all subscriptions before dropping
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        if !self.channels.is_empty() {
            let mut args = CommandArgsMut::default();
            for channel in &self.channels {
                args = args.arg(channel.as_ref());
            }
            let _result = self.client.unsubscribe(args).forget();
            self.channels.clear();
        }

        if !self.patterns.is_empty() {
            let mut args = CommandArgsMut::default();
            for pattern in &self.patterns {
                args = args.arg(pattern.as_ref());
            }
            let _result = self.client.punsubscribe(args).forget();
            self.patterns.clear();
        }

        if !self.shardchannels.is_empty() {
            let mut args = CommandArgsMut::default();
            for shardchannel in &self.shardchannels {
                args = args.arg(shardchannel.as_ref());
            }
            let _result = self.client.sunsubscribe(args).forget();
            self.shardchannels.clear();
        }

        self.closed = true;
    }
}

/// A pub sub `Stream` part of the [`split`](PubSubStream::split) pair.
/// It allows to get messages from the channels or patterns subscribed to
pub struct PubSubSplitStream {
    receiver: PubSubReceiver,
}

impl Stream for PubSubSplitStream {
    type Item = Result<PubSubMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.get_mut().receiver.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(message))) => Poll::Ready(Some(message.to())),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Stream to get messages from the channels or patterns [`subscribed`](https://redis.io/docs/manual/pubsub/) to
/// It allows also to subscribe/unsubscribe to/from channels or patterns
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
    split_sink: PubSubSplitSink,
    split_stream: PubSubSplitStream,
}

impl PubSubStream {
    pub(crate) fn new(sender: PubSubSender, receiver: PubSubReceiver, client: Client) -> Self {
        Self {
            split_sink: PubSubSplitSink {
                closed: false,
                channels: HashSet::default(),
                patterns: HashSet::default(),
                shardchannels: HashSet::default(),
                sender,
                client,
            },
            split_stream: PubSubSplitStream { receiver },
        }
    }

    pub(crate) fn from_channels(
        channels: CommandArgs,
        sender: PubSubSender,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        let mut set = HashSet::with_capacity(channels.len());
        extract_args_to_set(channels, &mut set);
        Self {
            split_sink: PubSubSplitSink {
                closed: false,
                channels: set,
                patterns: HashSet::default(),
                shardchannels: HashSet::default(),
                sender,
                client,
            },
            split_stream: PubSubSplitStream { receiver },
        }
    }

    pub(crate) fn from_patterns(
        patterns: CommandArgs,
        sender: PubSubSender,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        let mut set: HashSet<Bytes> = HashSet::with_capacity(patterns.len());
        extract_args_to_set(patterns, &mut set);
        Self {
            split_sink: PubSubSplitSink {
                closed: false,
                channels: HashSet::default(),
                patterns: set,
                shardchannels: HashSet::default(),
                sender,
                client,
            },
            split_stream: PubSubSplitStream { receiver },
        }
    }

    pub(crate) fn from_shardchannels(
        shardchannels: CommandArgs,
        sender: PubSubSender,
        receiver: PubSubReceiver,
        client: Client,
    ) -> Self {
        let mut set: HashSet<Bytes> = HashSet::with_capacity(shardchannels.len());
        extract_args_to_set(shardchannels, &mut set);
        Self {
            split_sink: PubSubSplitSink {
                closed: false,
                channels: HashSet::default(),
                patterns: HashSet::default(),
                shardchannels: set,
                sender,
                client,
            },
            split_stream: PubSubSplitStream { receiver },
        }
    }

    /// Subscribe to additional channels
    pub async fn subscribe(&mut self, channels: impl Serialize) -> Result<()> {
        self.split_sink.subscribe(channels).await
    }

    /// Subscribe to additional patterns
    pub async fn psubscribe(&mut self, patterns: impl Serialize) -> Result<()> {
        self.split_sink.psubscribe(patterns).await
    }

    /// Subscribe to additional shardchannels
    pub async fn ssubscribe(&mut self, shardchannels: impl Serialize) -> Result<()> {
        self.split_sink.ssubscribe(shardchannels).await
    }

    /// Unsubscribe from the given channels
    pub async fn unsubscribe(&mut self, channels: impl Serialize) -> Result<()> {
        self.split_sink.unsubscribe(channels).await
    }

    /// Unsubscribe from the given patterns
    pub async fn punsubscribe(&mut self, patterns: impl Serialize) -> Result<()> {
        self.split_sink.punsubscribe(patterns).await
    }

    /// Unsubscribe from the given patterns
    pub async fn sunsubscribe(&mut self, shardchannels: impl Serialize) -> Result<()> {
        self.split_sink.sunsubscribe(shardchannels).await
    }

    /// Splits this object into separate [`Sink`](PubSubSplitSink) and [`Stream`](PubSubSplitStream) objects.
    /// This can be useful when you want to split ownership between tasks.
    pub fn split(self) -> (PubSubSplitSink, PubSubSplitStream) {
        (self.split_sink, self.split_stream)
    }

    /// Close the stream by cancelling all subscriptions
    /// Calling `close` allows to wait for all the unsubscriptions.
    /// `drop` will achieve the same process but silently in background
    pub async fn close(self) -> Result<()> {
        self.split_sink.close().await
    }
}

impl Stream for PubSubStream {
    type Item = Result<PubSubMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.split_sink.closed {
            Poll::Ready(None)
        } else {
            let pinned = std::pin::pin!(&mut self.get_mut().split_stream);
            pinned.poll_next(cx)
        }
    }
}
