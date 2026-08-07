use crate::{
    ClientError, Error, PubSubReceiver, Result,
    client::{Client, ClientPreparedCommand},
    commands::InternalPubSubCommands,
    network::{PubSubPush, PubSubSender},
    resp::{CommandArgs, CommandArgsMut, RefBulkString, RespResponse},
};
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use std::{
    collections::HashSet,
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::warn;

/// Pub/Sub Message that can be streamed from [`PubSubStream`](PubSubStream)
///
/// The three segments — pattern, channel and payload — live end to end in one
/// exactly-sized block, so a message costs a single allocation whatever its
/// shape.
///
/// The bytes are copied out of the network read buffer as the message is
/// delivered. That buffer is a block the network task recycles across replies; a
/// message that borrowed from it would pin the whole block for as long as the
/// subscriber held the message, which is why the segments are owned rather than
/// shared.
pub struct PubSubMessage {
    /// pattern ‖ channel ‖ payload, contiguous.
    buf: Box<[u8]>,
    channel_start: usize,
    payload_start: usize,
}

impl PubSubMessage {
    /// The pattern the message matched, empty unless it was delivered through
    /// [`psubscribe`](crate::commands::PubSubCommands::psubscribe).
    #[inline]
    pub fn pattern(&self) -> &[u8] {
        &self.buf[..self.channel_start]
    }

    /// The channel the message was published to.
    #[inline]
    pub fn channel(&self) -> &[u8] {
        &self.buf[self.channel_start..self.payload_start]
    }

    /// The published payload.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.buf[self.payload_start..]
    }

    #[inline]
    fn from_segments(pattern: &[u8], channel: &[u8], payload: &[u8]) -> Self {
        let channel_start = pattern.len();
        let payload_start = channel_start.saturating_add(channel.len());
        let mut buf = Vec::with_capacity(payload_start.saturating_add(payload.len()));
        buf.extend_from_slice(pattern);
        buf.extend_from_slice(channel);
        buf.extend_from_slice(payload);

        Self {
            // The capacity is exact, so this hands the block over as it is.
            buf: buf.into_boxed_slice(),
            channel_start,
            payload_start,
        }
    }
}

impl TryFrom<&RespResponse> for PubSubMessage {
    type Error = Error;

    #[inline]
    fn try_from(response: &RespResponse) -> Result<Self> {
        match PubSubPush::try_from(response) {
            Ok(PubSubPush::Message(channel, payload) | PubSubPush::SMessage(channel, payload)) => {
                Ok(Self::from_segments(&[], channel, payload))
            }
            Ok(PubSubPush::PMessage(pattern, channel, payload)) => {
                Ok(Self::from_segments(pattern, channel, payload))
            }
            _ => Err(Error::from(ClientError::UnexpectedPubSubMessage)),
        }
    }
}

impl fmt::Debug for PubSubMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PubSubMessage")
            .field("pattern", &String::from_utf8_lossy(self.pattern()))
            .field("channel", &String::from_utf8_lossy(self.channel()))
            .field("payload", &String::from_utf8_lossy(self.payload()))
            .finish()
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
                return Err(Error::from(ClientError::AlreadySubscribed));
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
                return Err(Error::from(ClientError::AlreadySubscribed));
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
                return Err(Error::from(ClientError::AlreadySubscribed));
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

        self.client.unsubscribe(&channels).await?;

        // Forget the channels only once the server has confirmed: on a send
        // failure the subscription still stands server-side, so dropping it from
        // local tracking here would leave a ghost the stream keeps receiving and
        // `close`/`Drop` no longer clean up. `subscribe` is the reference — it
        // inserts only after success.
        for channel in &channels {
            self.channels.remove(&channel);
        }

        Ok(())
    }

    /// Unsubscribe from the given patterns
    pub async fn punsubscribe(&mut self, patterns: impl Serialize) -> Result<()> {
        let patterns = CommandArgsMut::default().arg(patterns).freeze();

        self.client.punsubscribe(&patterns).await?;

        // Forget only after the server confirms — see `unsubscribe`.
        for pattern in &patterns {
            self.patterns.remove(&pattern);
        }

        Ok(())
    }

    /// Unsubscribe from the given patterns
    pub async fn sunsubscribe(&mut self, shardchannels: impl Serialize) -> Result<()> {
        let shardchannels = CommandArgsMut::default().arg(shardchannels).freeze();

        self.client.sunsubscribe(&shardchannels).await?;

        // Forget only after the server confirms — see `unsubscribe`.
        for shardchannel in &shardchannels {
            self.shardchannels.remove(&shardchannel);
        }

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

        // Each name is wrapped in `RefBulkString`: a bare `&[u8]` serializes as a sequence of
        // integers, so `UNSUBSCRIBE` was sent the decimal byte values of the channel name -- `49 49`
        // for the channel `11` -- which unsubscribes from channels nobody subscribed to and leaves
        // the real one registered for the life of the connection. `close` never had the bug because
        // it passes the `Bytes` itself.
        if !self.channels.is_empty() {
            let mut args = CommandArgsMut::default();
            for channel in &self.channels {
                args = args.arg(RefBulkString::new(channel.as_ref()));
            }
            if let Err(e) = self.client.unsubscribe(args).forget() {
                warn!("Error while unsubscribing from the dropped channels: {e}");
            }
            self.channels.clear();
        }

        if !self.patterns.is_empty() {
            let mut args = CommandArgsMut::default();
            for pattern in &self.patterns {
                args = args.arg(RefBulkString::new(pattern.as_ref()));
            }
            if let Err(e) = self.client.punsubscribe(args).forget() {
                warn!("Error while unsubscribing from the dropped patterns: {e}");
            }
            self.patterns.clear();
        }

        if !self.shardchannels.is_empty() {
            let mut args = CommandArgsMut::default();
            for shardchannel in &self.shardchannels {
                args = args.arg(RefBulkString::new(shardchannel.as_ref()));
            }
            if let Err(e) = self.client.sunsubscribe(args).forget() {
                warn!("Error while unsubscribing from the dropped shard channels: {e}");
            }
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

impl PubSubSplitStream {
    /// Number of messages dropped so far because this stream fell behind its
    /// memory budget.
    ///
    /// See [`PubSubStream::dropped_messages`].
    pub fn dropped_messages(&self) -> usize {
        self.receiver.dropped_messages()
    }
}

impl Stream for PubSubSplitStream {
    type Item = Result<PubSubMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.get_mut().receiver.poll_next_unpin(cx) {
            // The response is dropped as this returns, releasing the recycled
            // network block the message's bytes were just copied out of.
            Poll::Ready(Some(Ok(response))) => {
                Poll::Ready(Some(PubSubMessage::try_from(&response)))
            }
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
///     let mut message = pub_sub_stream.next().await.unwrap()?;
///     assert_eq!(b"mychannel", message.channel());
///     assert_eq!(b"mymessage", message.payload());
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
    /// Number of messages dropped so far because this stream fell behind its
    /// memory budget.
    ///
    /// The client holds at most
    /// [`BackpressureConfig::max_pubsub_bytes`](crate::client::BackpressureConfig::max_pubsub_bytes)
    /// of undelivered messages per stream. Past that it discards the **oldest**
    /// ones, so a stream that resumes reading sees current data rather than a
    /// stale prefix. The count is cumulative and never resets; sample it to tell
    /// whether a consumer is keeping up.
    ///
    /// It stays at `0` for a consumer that keeps up, and for any stream if the
    /// budget is disabled.
    pub fn dropped_messages(&self) -> usize {
        self.split_stream.dropped_messages()
    }

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
    ///
    /// # Semantics
    /// Once closed, the stream terminates immediately: any message that was
    /// delivered to the internal receiver but not yet polled is **discarded**.
    /// If draining those pending messages matters, use [`split`](Self::split)
    /// and keep polling the [`PubSubSplitStream`], which drains its receiver
    /// naturally before ending.
    pub async fn close(self) -> Result<()> {
        self.split_sink.close().await
    }
}

impl Stream for PubSubStream {
    type Item = Result<PubSubMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        // Terminating on `closed` intentionally drops any buffered-but-unread
        // messages: draining the receiver here could block indefinitely when the
        // sender has not been released. See `close` for the documented semantics
        // and the split-stream drain path.
        if self.split_sink.closed {
            Poll::Ready(None)
        } else {
            let pinned = std::pin::pin!(&mut self.get_mut().split_stream);
            pinned.poll_next(cx)
        }
    }
}
