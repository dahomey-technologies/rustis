use crate::{
    client::{Client, ClientPreparedCommand},
    commands::InternalPubSubCommands,
    network::PubSubSender,
    resp::{CommandArgs, SingleArg, SingleArgCollection, Value},
    Error, PubSubReceiver, Result,
};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Pub/Sub Message that can be streamed from [`PubSubStream`](PubSubStream)
pub struct PubSubMessage {
    pub pattern: Vec<u8>,
    pub channel: Vec<u8>,
    pub payload: Vec<u8>,
}

impl PubSubMessage {
    pub(crate) fn from_message(channel: Vec<u8>, payload: Vec<u8>) -> Self {
        Self {
            pattern: vec![],
            channel,
            payload,
        }
    }

    pub(crate) fn from_pmessage(pattern: Vec<u8>, channel: Vec<u8>, payload: Vec<u8>) -> Self {
        Self {
            pattern,
            channel,
            payload,
        }
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
///     let channel: String = message.get_channel()?;
///     let payload: String = message.get_payload()?;
///
///     assert_eq!("mychannel", channel);
///     assert_eq!("mymessage", payload);
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
            patterns: CommandArgs::Empty,
            shardchannels: CommandArgs::Empty,
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
            channels: CommandArgs::Empty,
            patterns,
            shardchannels: CommandArgs::Empty,
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
            channels: CommandArgs::Empty,
            patterns: CommandArgs::Empty,
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
        let channels = channels.into_args(CommandArgs::Empty);

        self.client
            .subscribe_from_pub_sub_sender(&channels, &self.sender)
            .await?;

        let mut existing_channels = CommandArgs::Empty;
        std::mem::swap(&mut existing_channels, &mut self.channels);
        self.channels = existing_channels.arg(channels);

        Ok(())
    }

    /// Subscribe to additional patterns
    pub async fn psubscribe<P, PP>(&mut self, patterns: PP) -> Result<()>
    where
        P: SingleArg + Send,
        PP: SingleArgCollection<P>,
    {
        let patterns = patterns.into_args(CommandArgs::Empty);

        self.client
            .psubscribe_from_pub_sub_sender(&patterns, &self.sender)
            .await?;

        let mut existing_patterns = CommandArgs::Empty;
        std::mem::swap(&mut existing_patterns, &mut self.patterns);
        self.patterns = existing_patterns.arg(patterns);

        Ok(())
    }

    /// Subscribe to additional shardchannels
    pub async fn ssubscribe<C, CC>(&mut self, shardchannels: CC) -> Result<()>
    where
        C: SingleArg + Send,
        CC: SingleArgCollection<C>,
    {
        let shardchannels = shardchannels.into_args(CommandArgs::Empty);

        self.client
            .ssubscribe_from_pub_sub_sender(&shardchannels, &self.sender)
            .await?;

        let mut existing_shardchannels = CommandArgs::Empty;
        std::mem::swap(&mut existing_shardchannels, &mut self.shardchannels);
        self.shardchannels = existing_shardchannels.arg(shardchannels);

        Ok(())
    }

    /// Close the stream by cancelling all subscriptions
    /// Calling `close` allows to wait for all the unsubscriptions.
    /// `drop` will achieve the same process but silently in background
    pub async fn close(mut self) -> Result<()> {
        let mut channels = CommandArgs::Empty;
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            self.client.unsubscribe(channels).await?;
        }

        let mut patterns = CommandArgs::Empty;
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            self.client.punsubscribe(patterns).await?;
        }

        let mut shardchannels = CommandArgs::Empty;
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
        fn extract_message(message: Value) -> Result<PubSubMessage> {
            let parts: Vec<Value> = message.into()?;
            let mut iter = parts.into_iter();

            match (
                iter.next(),
                iter.next(),
                iter.next(),
                iter.next(),
                iter.next(),
            ) {
                (
                    Some(Value::BulkString(command)),
                    Some(Value::BulkString(pattern)),
                    Some(Value::BulkString(channel)),
                    Some(Value::BulkString(payload)),
                    None,
                ) if command.as_slice() == b"pmessage" => {
                    Ok(PubSubMessage::from_pmessage(pattern, channel, payload))
                }
                (
                    Some(Value::BulkString(command)),
                    Some(Value::BulkString(channel)),
                    Some(Value::BulkString(payload)),
                    None,
                    None,
                ) if command.as_slice() == b"message" || command.as_slice() == b"smessage" => {
                    Ok(PubSubMessage::from_message(channel, payload))
                }
                message => Err(Error::Client(format!("Cannot parse PubSubMessage: {message:?}"))),
            }
        }

        if self.closed {
            Poll::Ready(None)
        } else {
            match self.get_mut().receiver.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(message))) => Poll::Ready(Some(extract_message(message))),
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

        let mut channels = CommandArgs::Empty;
        std::mem::swap(&mut channels, &mut self.channels);
        if !channels.is_empty() {
            let _result = self.client.unsubscribe(channels).forget();
        }

        let mut patterns = CommandArgs::Empty;
        std::mem::swap(&mut patterns, &mut self.patterns);
        if !patterns.is_empty() {
            let _result = self.client.punsubscribe(patterns).forget();
        }

        let mut shardchannels = CommandArgs::Empty;
        std::mem::swap(&mut shardchannels, &mut self.shardchannels);
        if !shardchannels.is_empty() {
            let _result = self.client.sunsubscribe(shardchannels).forget();
        }
    }
}
