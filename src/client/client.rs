#[cfg(test)]
use crate::commands::DebugCommands;
use crate::{
    ClientError, Error, Future, Result,
    client::{
        ClientTrackingInvalidationStream, IntoConfig, Message, MonitorStream, Pipeline,
        PreparedCommand, PubSubStream, ServerConfig, Transaction, bounded_channel,
    },
    commands::{
        ArrayCommands, BitmapCommands, BlockingCommands, BloomCommands, ClusterCommands,
        ConnectionCommands, CountMinSketchCommands, CuckooCommands, GenericCommands, GeoCommands,
        HashCommands, HyperLogLogCommands, InternalPubSubCommands, JsonCommands, ListCommands,
        PubSubCommands, ScriptingCommands, SearchCommands, SentinelCommands, ServerCommands,
        SetCommands, SortedSetCommands, StreamCommands, StringCommands, TDigestCommands,
        TimeSeriesCommands, TopKCommands, TransactionCommands, VectorSetCommands,
    },
    network::{
        JoinHandle, MsgSender, NetworkHandler, PubSubReceiver, PubSubSender, PushReceiver,
        PushSender, ReconnectReceiver, ReconnectSender, ResultReceiver, ResultSender,
        ResultsReceiver, ResultsSender, timeout,
    },
    resp::{Command, CommandArgs, CommandArgsMut, RespResponse, Response, SubscriptionType, cmd},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{future::IntoFuture, sync::Arc, time::Duration};
use tracing::{info, trace};

/// Client with a unique connection to a Redis server.
/// State shared by every clone of a [`Client`] over a single connection.
///
/// The message sender and the network task join handle live behind **one**
/// `Arc`, so a single reference count governs both. Shutting the connection
/// down is then gated on [`Arc::into_inner`], which hands the last owner
/// exclusive access and — crucially — returns `Some` to exactly one caller even
/// when several clones drop (or `close`) concurrently. Two independent `Arc`s
/// decided with `try_unwrap` allowed two threads to each observe strong-count 2
/// and both back off, leaking the task, socket and buffers forever.
struct ClientShared {
    msg_sender: MsgSender,
    network_task_join_handle: JoinHandle<()>,
}

#[derive(Clone)]
pub struct Client {
    /// `Option` only so a dropping/closing clone can swap its reference out of
    /// `&mut self` before calling [`Arc::into_inner`]; a live client always
    /// holds `Some`.
    shared: Arc<Option<ClientShared>>,
    reconnect_sender: ReconnectSender,
    command_timeout: Duration,
    retry_on_error: bool,
    connection_tag: Arc<str>,
    /// Whether this client talks to a Redis Cluster, which constrains what a
    /// transaction may contain.
    is_cluster: bool,
    /// Memory budget handed to each pub/sub stream this client opens, from
    /// `Config::backpressure.max_pubsub_bytes`.
    max_pubsub_bytes: usize,
    /// Memory budget handed to each push sink this client opens, from
    /// `Config::backpressure.max_push_bytes`.
    max_push_bytes: usize,
}

impl Client {
    /// Connects asynchronously to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    #[inline]
    pub async fn connect(config: impl IntoConfig) -> Result<Self> {
        let config = config.into_config()?;
        let command_timeout = config.command_timeout;
        let retry_on_error = config.retry_on_error;
        let is_cluster = matches!(config.server, ServerConfig::Cluster(_));
        let max_pubsub_bytes = config.backpressure.max_pubsub_bytes;
        let max_push_bytes = config.backpressure.max_push_bytes;
        let (msg_sender, network_task_join_handle, reconnect_sender, connection_tag) =
            NetworkHandler::connect(config.into_config()?).await?;

        Ok(Self {
            shared: Arc::new(Some(ClientShared {
                msg_sender,
                network_task_join_handle,
            })),
            reconnect_sender,
            command_timeout,
            retry_on_error,
            connection_tag,
            is_cluster,
            max_pubsub_bytes,
            max_push_bytes,
        })
    }

    /// Whether this client is connected to a Redis Cluster.
    pub(crate) fn is_cluster(&self) -> bool {
        self.is_cluster
    }

    #[allow(dead_code)]
    pub(crate) fn connection_tag(&self) -> &str {
        &self.connection_tag
    }

    /// Whether the network task behind this client has ended.
    ///
    /// It ends when the connection is gone for good — the reconnection budget
    /// exhausted, or the last sender dropped — after which the client can no
    /// longer answer anything. Reading the join handle is non-blocking and says
    /// nothing about a connection that is merely idle.
    ///
    /// Only the pool needs this: it is how a dead client is evicted instead of
    /// being handed to the next borrower.
    #[cfg(feature = "pool")]
    pub(crate) fn is_network_task_finished(&self) -> bool {
        self.shared
            .as_ref()
            .as_ref()
            .is_some_and(|shared| shared.network_task_join_handle.is_finished())
    }

    /// if this client is the last client on the shared connection, the channel to send messages
    /// to the underlying network handler will be closed explicitely.
    ///
    /// Then, this function will await for the network handler to be ended
    pub async fn close(mut self) -> Result<()> {
        let mut shared: Arc<Option<ClientShared>> = Arc::new(None);
        std::mem::swap(&mut shared, &mut self.shared);

        // stop the network loop if we are the last owner of the shared state;
        // `into_inner` makes that determination race-free against a concurrent
        // `close`/`Drop` (see `ClientShared`).
        if let Some(Some(shared)) = Arc::into_inner(shared) {
            let ClientShared {
                msg_sender,
                network_task_join_handle,
            } = shared;
            // Dropping the last strong sender closes the channel, which is what
            // ends the network loop; the task keeps a weak handle only, so this
            // must happen before awaiting it, otherwise the loop never sees the
            // channel close and the await deadlocks.
            drop(msg_sender);
            network_task_join_handle.await?;
        }

        Ok(())
    }

    /// Used to receive notifications when the client reconnects to the Redis server.
    ///
    /// To turn this receiver into a Stream, you can use the
    /// [`BroadcastStream`](https://docs.rs/tokio-stream/latest/tokio_stream/wrappers/struct.BroadcastStream.html) wrapper.
    pub fn on_reconnect(&self) -> ReconnectReceiver {
        self.reconnect_sender.subscribe()
    }

    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    /// * `retry_on_error` - retry to send the command on network error.
    ///   * `None` - default behaviour defined in [`Config::retry_on_error`](crate::client::Config::retry_on_error)
    ///   * `Some(true)` - retry sending command on network error
    ///   * `Some(false)` - do not retry sending command on network error
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    ///
    /// # Warning
    /// In Cluster mode, the arguments that are Redis keys must be added with
    /// [`CommandBuilder::key`](crate::resp::CommandBuilder::key): a command built with
    /// [`arg`](crate::resp::CommandBuilder::arg) alone carries no slot and is sent to a
    /// **random node**. A multi-key command such as `MSET` also requires all its keys to hash
    /// to the same slot, which the `{my}` hash tag guarantees in the example below.
    ///
    /// Unless `R` is an [`Option`], a `nil` reply decodes as the neutral value of `R`
    /// (`0`, `0.0`, `false`, `""`) instead of being rejected. Declare `Option<R>` when the
    /// command can reply `nil`. See [Command results](crate::resp#command-results).
    ///
    /// # Example
    /// ```
    /// use rustis::{client::Client, commands::{FlushingMode, ServerCommands}, resp::cmd, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     client.flushall(FlushingMode::Sync).await?;
    ///
    ///     client
    ///         .send::<()>(
    ///             cmd("MSET")
    ///                 .key("{my}key1")
    ///                 .arg("value1")
    ///                 .key("{my}key2")
    ///                 .arg("value2")
    ///                 .key("{my}key3")
    ///                 .arg("value3")
    ///                 .key("{my}key4")
    ///                 .arg("value4"),
    ///             None,
    ///         )
    ///         .await?;
    ///
    ///     let values: Vec<String> = client
    ///         .send(
    ///             cmd("MGET")
    ///                 .key("{my}key1")
    ///                 .key("{my}key2")
    ///                 .key("{my}key3")
    ///                 .key("{my}key4"),
    ///             None,
    ///         )
    ///         .await?;
    ///
    ///     assert_eq!(vec!["value1".to_owned(), "value2".to_owned(), "value3".to_owned(), "value4".to_owned()], values);
    ///
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub async fn send<T: DeserializeOwned>(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<T> {
        // The one place holding both the command and the type the caller
        // declared for its reply, so it is where the two can be confronted with
        // what the server actually answers.
        #[cfg(test)]
        let command = command.into();
        #[cfg(test)]
        let probe_label = crate::tests::response_probe::label(&command);

        let response = self.internal_send(command, retry_on_error).await?;

        #[cfg(not(test))]
        return response.to();

        // The outcome is recorded alongside the shape: a mismatch the decoder
        // refuses is a mismatch the caller was told about, where a mismatch it
        // coerces is the silent one this probe exists for.
        #[cfg(test)]
        {
            let result = response.to();
            crate::tests::response_probe::record(
                probe_label,
                std::any::type_name::<T>(),
                &response,
                result.is_ok(),
            );
            result
        }
    }

    #[inline]
    pub(crate) async fn internal_send(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<RespResponse> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();
        let message = Message::single(
            command.into(),
            result_sender,
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        self.send_message(message)?;

        self.await_result(result_receiver).await
    }

    /// Await a single-response oneshot, applying the configured `command_timeout`
    /// so subscribe/monitor callers honour the same contract as regular sends.
    #[inline]
    async fn await_result(&self, result_receiver: ResultReceiver) -> Result<RespResponse> {
        if self.command_timeout != Duration::ZERO {
            timeout(self.command_timeout, result_receiver).await??
        } else {
            result_receiver.await?
        }
    }

    /// Send command to the Redis server and forget its response.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    /// * `retry_on_error` - retry to send the command on network error.
    ///   * `None` - default behaviour defined in [`Config::retry_on_error`](crate::client::Config::retry_on_error)
    ///   * `Some(true)` - retry sending command on network error
    ///   * `Some(false)` - do not retry sending command on network error
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    #[inline]
    pub fn send_and_forget(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<()> {
        let message = Message::single_forget(
            command.into(),
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        self.send_message(message)?;
        Ok(())
    }

    /// Send a batch of commands to the Redis server.
    ///
    /// # Arguments
    /// * `commands` - batch of generic [`Command`](crate::resp::Command)s meant to be sent to the Redis server.
    /// * `retry_on_error` - retry to send the command batch on network error.
    ///   * `None` - default behaviour defined in [`Config::retry_on_error`](crate::client::Config::retry_on_error)
    ///   * `Some(true)` - retry sending batch on network error
    ///   * `Some(false)` - do not retry sending batch on network error
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    #[inline]
    pub(crate) async fn internal_send_batch(
        &self,
        commands: Vec<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<Vec<RespResponse>> {
        let (results_sender, results_receiver): (ResultsSender, ResultsReceiver) =
            tokio::sync::oneshot::channel();
        let message = Message::batch(
            commands,
            results_sender,
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        self.send_message(message)?;

        if self.command_timeout != Duration::ZERO {
            timeout(self.command_timeout, results_receiver).await??
        } else {
            results_receiver.await?
        }
    }

    #[inline]
    fn send_message(&self, mut message: Message) -> Result<()> {
        // Surface any serialization error deferred by the fluent builder (a
        // failing user `Serialize` impl) before the command reaches the network
        // layer, so the caller gets a clean error instead of a panic.
        for command in message.commands_mut() {
            if let Some(error) = command.take_serialization_error() {
                return Err(error);
            }
        }

        // Compute cluster hash slots here, on the caller thread, and only in
        // cluster mode. This keeps CRC16 off the shared network thread (the
        // multiplexer domain) while sparing standalone clients the cost.
        if self.is_cluster {
            for command in message.commands_mut() {
                command.compute_slots();
            }
        }

        if let Some(shared) = self.shared.as_ref() {
            trace!(
                tag = %self.connection_tag,
                "Will enqueue message: {message:?}"
            );
            Ok(shared.msg_sender.send(message).map_err(|e| {
                info!("{e}");
                Error::Client(ClientError::DisconnectedFromServer)
            })?)
        } else {
            Err(Error::Client(ClientError::InvalidChannel))
        }
    }

    /// Create a new transaction
    #[inline]
    pub fn create_transaction(&self) -> Transaction {
        Transaction::new(self.clone())
    }

    /// Create a new pipeline
    #[inline]
    pub fn create_pipeline<'a>(&'a self) -> Pipeline<'a> {
        Pipeline::new(self)
    }

    /// Create a new pub sub stream with no upfront subscription
    #[inline]
    pub fn create_pub_sub(&self) -> PubSubStream {
        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);
        PubSubStream::new(pub_sub_sender, pub_sub_receiver, self.clone())
    }

    /// Create a stream of client-side caching invalidations.
    ///
    /// The stream yields the keys Redis has invalidated, as
    /// [`BulkString`](crate::resp::BulkString) — Redis keys are binary-safe.
    /// Enable tracking on the same client with
    /// [`client_tracking`](crate::commands::ConnectionCommands::client_tracking)
    /// for the server to start sending them.
    ///
    /// ```
    /// use rustis::{client::{Client, ClientTrackingInvalidationStream}, Result};
    ///
    /// async fn watch(client: &Client) -> Result<ClientTrackingInvalidationStream> {
    ///     client.create_client_tracking_invalidation_stream()
    /// }
    /// ```
    pub fn create_client_tracking_invalidation_stream(
        &self,
    ) -> Result<ClientTrackingInvalidationStream> {
        let (push_sender, push_receiver): (PushSender, PushReceiver) =
            bounded_channel(self.max_push_bytes);
        let message = Message::client_tracking_invalidation(push_sender);
        self.send_message(message)?;
        Ok(ClientTrackingInvalidationStream::new(push_receiver))
    }

    pub(crate) async fn subscribe_from_pub_sub_sender(
        &self,
        channels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();

        let pub_sub_senders = channels
            .into_iter()
            .map(|c| (c, pub_sub_sender.clone()))
            .collect();

        let message = Message::pub_sub(
            cmd("SUBSCRIBE").arg(channels).into(),
            result_sender,
            SubscriptionType::Channel,
            pub_sub_senders,
        );

        self.send_message(message)?;

        self.await_result(result_receiver).await?.to::<()>()
    }

    pub(crate) async fn psubscribe_from_pub_sub_sender(
        &self,
        patterns: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();

        let pub_sub_senders = patterns
            .into_iter()
            .map(|c| (c, pub_sub_sender.clone()))
            .collect();

        let message = Message::pub_sub(
            cmd("PSUBSCRIBE").arg(patterns).into(),
            result_sender,
            SubscriptionType::Pattern,
            pub_sub_senders,
        );

        self.send_message(message)?;

        self.await_result(result_receiver).await?.to::<()>()
    }

    pub(crate) async fn ssubscribe_from_pub_sub_sender(
        &self,
        shardchannels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();

        let pub_sub_senders = shardchannels
            .into_iter()
            .map(|c| (c, pub_sub_sender.clone()))
            .collect();

        let message = Message::pub_sub(
            cmd("SSUBSCRIBE").key(shardchannels).into(),
            result_sender,
            SubscriptionType::ShardChannel,
            pub_sub_senders,
        );

        self.send_message(message)?;

        self.await_result(result_receiver).await?.to::<()>()
    }
}

/// Extension trait dedicated to [`PreparedCommand`](crate::client::PreparedCommand)
/// to add specific methods for the [`Client`](crate::client::Client) executor
pub trait ClientPreparedCommand<'a, R> {
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R: Response> ClientPreparedCommand<'a, R> for PreparedCommand<'a, &'a Client, R> {
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()> {
        self.executor
            .send_and_forget(self.command, self.retry_on_error)
    }
}

impl<'a, R: Response + DeserializeOwned + 'a> IntoFuture for PreparedCommand<'a, &'a Client, R> {
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.executor.send(self.command, self.retry_on_error).await })
    }
}

impl<'a> ArrayCommands<'a> for &'a Client {}
impl<'a> BitmapCommands<'a> for &'a Client {}
impl<'a> BloomCommands<'a> for &'a Client {}
impl<'a> ClusterCommands<'a> for &'a Client {}
impl<'a> CountMinSketchCommands<'a> for &'a Client {}
impl<'a> CuckooCommands<'a> for &'a Client {}
impl<'a> ConnectionCommands<'a> for &'a Client {}
#[cfg(test)]
impl<'a> DebugCommands<'a> for &'a Client {}
impl<'a> GenericCommands<'a> for &'a Client {}
impl<'a> GeoCommands<'a> for &'a Client {}
impl<'a> HashCommands<'a> for &'a Client {}
impl<'a> HyperLogLogCommands<'a> for &'a Client {}
impl<'a> InternalPubSubCommands<'a> for &'a Client {}
impl<'a> JsonCommands<'a> for &'a Client {}
impl<'a> ListCommands<'a> for &'a Client {}
impl<'a> ScriptingCommands<'a> for &'a Client {}
impl<'a> SearchCommands<'a> for &'a Client {}
impl<'a> SentinelCommands<'a> for &'a Client {}
impl<'a> ServerCommands<'a> for &'a Client {}
impl<'a> SetCommands<'a> for &'a Client {}
impl<'a> SortedSetCommands<'a> for &'a Client {}
impl<'a> StreamCommands<'a> for &'a Client {}
impl<'a> StringCommands<'a> for &'a Client {}
impl<'a> TDigestCommands<'a> for &'a Client {}
impl<'a> TimeSeriesCommands<'a> for &'a Client {}
impl<'a> TransactionCommands<'a> for &'a Client {}
impl<'a> TopKCommands<'a> for &'a Client {}
impl<'a> VectorSetCommands<'a> for &'a Client {}

impl<'a> PubSubCommands<'a> for &'a Client {
    #[inline]
    async fn subscribe(self, channels: impl Serialize) -> Result<PubSubStream> {
        let channels = CommandArgsMut::default().arg(channels).freeze();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);

        self.subscribe_from_pub_sub_sender(&channels, &pub_sub_sender)
            .await?;

        Ok(PubSubStream::from_channels(
            channels,
            pub_sub_sender,
            pub_sub_receiver,
            self.clone(),
        ))
    }

    #[inline]
    async fn psubscribe(self, patterns: impl Serialize) -> Result<PubSubStream> {
        let patterns = CommandArgsMut::default().arg(patterns).freeze();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);

        self.psubscribe_from_pub_sub_sender(&patterns, &pub_sub_sender)
            .await?;

        Ok(PubSubStream::from_patterns(
            patterns,
            pub_sub_sender,
            pub_sub_receiver,
            self.clone(),
        ))
    }

    #[inline]
    async fn ssubscribe(self, shardchannels: impl Serialize) -> Result<PubSubStream> {
        let shardchannels = CommandArgsMut::default().arg(shardchannels).freeze();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);

        self.ssubscribe_from_pub_sub_sender(&shardchannels, &pub_sub_sender)
            .await?;

        Ok(PubSubStream::from_shardchannels(
            shardchannels,
            pub_sub_sender,
            pub_sub_receiver,
            self.clone(),
        ))
    }
}

impl<'a> BlockingCommands<'a> for &'a Client {
    async fn monitor(self) -> Result<MonitorStream> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();
        let (push_sender, push_receiver): (PushSender, PushReceiver) =
            bounded_channel(self.max_push_bytes);

        let message = Message::monitor(cmd("MONITOR").into(), result_sender, push_sender);

        self.send_message(message)?;

        let _bytes = self.await_result(result_receiver).await?;
        Ok(MonitorStream::new(push_receiver, self.clone()))
    }
}
