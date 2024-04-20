#[cfg(test)]
use crate::commands::DebugCommands;
#[cfg(feature = "redis-graph")]
use crate::commands::GraphCommands;
#[cfg(feature = "redis-json")]
use crate::commands::JsonCommands;
#[cfg(feature = "redis-search")]
use crate::commands::SearchCommands;
#[cfg(feature = "redis-time-series")]
use crate::commands::TimeSeriesCommands;
#[cfg(feature = "redis-bloom")]
use crate::commands::{
    BloomCommands, CountMinSketchCommands, CuckooCommands, TDigestCommands, TopKCommands,
};
use crate::{
    client::{
        ClientState, ClientTrackingInvalidationStream, IntoConfig, Message, MonitorStream,
        Pipeline, PreparedCommand, PubSubStream, Transaction,
    },
    commands::{
        BitmapCommands, BlockingCommands, ClusterCommands, ConnectionCommands, GenericCommands,
        GeoCommands, HashCommands, HyperLogLogCommands, InternalPubSubCommands, ListCommands,
        PubSubCommands, ScriptingCommands, SentinelCommands, ServerCommands, SetCommands,
        SortedSetCommands, StreamCommands, StringCommands, TransactionCommands,
    },
    network::{
        timeout, JoinHandle, MsgSender, NetworkHandler, PubSubReceiver, PubSubSender, PushReceiver,
        PushSender, ReconnectReceiver, ReconnectSender, ResultReceiver, ResultSender,
        ResultsReceiver, ResultsSender,
    },
    resp::{cmd, Command, CommandArgs, RespBuf, Response, SingleArg, SingleArgCollection},
    Error, Future, Result,
};
use futures_channel::{mpsc, oneshot};
use futures_util::Stream;
use log::{info, trace};
use serde::de::DeserializeOwned;
use std::{
    future::IntoFuture,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
};

/// Client with a unique connection to a Redis server.
#[derive(Clone)]
pub struct Client {
    msg_sender: Arc<Option<MsgSender>>,
    network_task_join_handle: Arc<Option<JoinHandle<()>>>,
    reconnect_sender: ReconnectSender,
    client_state: Arc<RwLock<ClientState>>,
    command_timeout: Duration,
    retry_on_error: bool,
}

impl Drop for Client {
    /// if this client is the last client on the shared connection, the channel to send messages
    /// to the underlying network handler will be closed explicitely
    fn drop(&mut self) {
        let mut network_task_join_handle: Arc<Option<JoinHandle<()>>> = Arc::new(None);
        std::mem::swap(
            &mut network_task_join_handle,
            &mut self.network_task_join_handle,
        );

        // stop the network loop if we are the last reference to its handle
        if Arc::try_unwrap(network_task_join_handle).is_ok() {
            let mut msg_sender: Arc<Option<MsgSender>> = Arc::new(None);
            std::mem::swap(&mut msg_sender, &mut self.msg_sender);

            if let Ok(Some(msg_sender)) = Arc::try_unwrap(msg_sender) {
                // the network loop will automatically ends when it detects the sender bound has been closed
                msg_sender.close_channel();
            }
        };
    }
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
        let (msg_sender, network_task_join_handle, reconnect_sender) =
            NetworkHandler::connect(config.into_config()?).await?;

        Ok(Self {
            msg_sender: Arc::new(Some(msg_sender)),
            network_task_join_handle: Arc::new(Some(network_task_join_handle)),
            reconnect_sender,
            client_state: Arc::new(RwLock::new(ClientState::new())),
            command_timeout,
            retry_on_error,
        })
    }

    /// if this client is the last client on the shared connection, the channel to send messages
    /// to the underlying network handler will be closed explicitely.
    ///
    /// Then, this function will await for the network handler to be ended
    pub async fn close(mut self) -> Result<()> {
        let mut network_task_join_handle: Arc<Option<JoinHandle<()>>> = Arc::new(None);
        std::mem::swap(
            &mut network_task_join_handle,
            &mut self.network_task_join_handle,
        );

        // stop the network loop if we are the last reference to its handle
        if let Ok(Some(network_task_join_handle)) = Arc::try_unwrap(network_task_join_handle) {
            let mut msg_sender: Arc<Option<MsgSender>> = Arc::new(None);
            std::mem::swap(&mut msg_sender, &mut self.msg_sender);

            if let Ok(Some(msg_sender)) = Arc::try_unwrap(msg_sender) {
                // the network loop will automatically ends when it detects the sender bound has been closed
                msg_sender.close_channel();
                network_task_join_handle.await?;
            }
        };

        Ok(())
    }

    /// Used to receive notifications when the client reconnects to the Redis server.
    ///
    /// To turn this receiver into a Stream, you can use the
    /// [`BroadcastStream`](https://docs.rs/tokio-stream/latest/tokio_stream/wrappers/struct.BroadcastStream.html) wrapper.
    pub fn on_reconnect(&self) -> ReconnectReceiver {
        self.reconnect_sender.subscribe()
    }

    /// Give an immutable generic access to attach any state to a client instance
    pub fn get_client_state(&self) -> RwLockReadGuard<ClientState> {
        self.client_state.read().unwrap()
    }

    /// Give a mutable generic access to attach any state to a client instance
    pub fn get_client_state_mut(&self) -> RwLockWriteGuard<ClientState> {
        self.client_state.write().unwrap()
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
    /// # Example
    /// ```
    /// use rustis::{client::Client, resp::cmd, Result};
    ///
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     client
    ///         .send(
    ///             cmd("MSET")
    ///                 .arg("key1")
    ///                 .arg("value1")
    ///                 .arg("key2")
    ///                 .arg("value2")
    ///                  .arg("key3")
    ///                 .arg("value3")
    ///                 .arg("key4")
    ///                 .arg("value4"),
    ///             None,
    ///         )
    ///         .await?
    ///         .to::<()>()?;
    ///
    ///     let values: Vec<String> = client
    ///         .send(
    ///             cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"),
    ///             None,
    ///         )
    ///         .await?
    ///         .to()?;
    ///
    ///     assert_eq!(vec!["value1".to_owned(), "value2".to_owned(), "value3".to_owned(), "value4".to_owned()], values);
    ///
    ///     Ok(())
    /// }
    /// ```

    #[inline]
    pub async fn send(&self, command: Command, retry_on_error: Option<bool>) -> Result<RespBuf> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) = oneshot::channel();
        let message = Message::single(
            command,
            result_sender,
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        self.send_message(message)?;

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
    pub fn send_and_forget(&self, command: Command, retry_on_error: Option<bool>) -> Result<()> {
        let message =
            Message::single_forget(command, retry_on_error.unwrap_or(self.retry_on_error));
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
    pub async fn send_batch(
        &self,
        commands: Vec<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<Vec<RespBuf>> {
        let (results_sender, results_receiver): (ResultsSender, ResultsReceiver) =
            oneshot::channel();
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
    fn send_message(&self, message: Message) -> Result<()> {
        if let Some(msg_sender) = &self.msg_sender as &Option<MsgSender> {
            trace!("Will enqueue message: {message:?}");
            Ok(msg_sender.unbounded_send(message).map_err(|e| {
                info!("{}", e.to_string());
                Error::Client("Disconnected from server".to_string())
            })?)
        } else {
            Err(Error::Client(
                "Invalid channel to send messages to the network handler".to_owned(),
            ))
        }
    }

    /// Create a new transaction
    #[inline]
    pub fn create_transaction(&self) -> Transaction {
        Transaction::new(self.clone())
    }

    /// Create a new pipeline
    #[inline]
    pub fn create_pipeline(&self) -> Pipeline {
        Pipeline::new(self)
    }

    /// Create a new pub sub stream with no upfront subscription
    #[inline]
    pub fn create_pub_sub(&self) -> PubSubStream {
        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) = mpsc::unbounded();
        PubSubStream::new(pub_sub_sender, pub_sub_receiver, self.clone())
    }

    pub fn create_client_tracking_invalidation_stream(
        &self,
    ) -> Result<impl Stream<Item = Vec<String>>> {
        let (push_sender, push_receiver): (PushSender, PushReceiver) = mpsc::unbounded();
        let message = Message::client_tracking_invalidation(push_sender);
        self.send_message(message)?;
        Ok(ClientTrackingInvalidationStream::new(push_receiver))
    }

    pub(crate) async fn subscribe_from_pub_sub_sender(
        &self,
        channels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) = oneshot::channel();

        let pub_sub_senders = channels
            .into_iter()
            .map(|c| (c.to_vec(), pub_sub_sender.clone()))
            .collect::<Vec<_>>();

        let message = Message::pub_sub(
            cmd("SUBSCRIBE").arg(channels.clone()),
            result_sender,
            pub_sub_senders,
        );

        self.send_message(message)?;

        result_receiver.await??.to::<()>()
    }

    pub(crate) async fn psubscribe_from_pub_sub_sender(
        &self,
        patterns: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) = oneshot::channel();

        let pub_sub_senders = patterns
            .into_iter()
            .map(|c| (c.to_vec(), pub_sub_sender.clone()))
            .collect::<Vec<_>>();

        let message = Message::pub_sub(
            cmd("PSUBSCRIBE").arg(patterns.clone()),
            result_sender,
            pub_sub_senders,
        );

        self.send_message(message)?;

        result_receiver.await??.to::<()>()
    }

    pub(crate) async fn ssubscribe_from_pub_sub_sender(
        &self,
        shardchannels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) = oneshot::channel();

        let pub_sub_senders = shardchannels
            .into_iter()
            .map(|c| (c.to_vec(), pub_sub_sender.clone()))
            .collect::<Vec<_>>();

        let message = Message::pub_sub(
            cmd("SSUBSCRIBE").arg(shardchannels.clone()),
            result_sender,
            pub_sub_senders,
        );

        self.send_message(message)?;

        result_receiver.await??.to::<()>()
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

impl<'a, R> IntoFuture for PreparedCommand<'a, &'a Client, R>
where
    R: DeserializeOwned + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            if let Some(custom_converter) = self.custom_converter {
                let command_for_result = self.command.clone();
                let result = self
                    .executor
                    .send(self.command, self.retry_on_error)
                    .await?;
                custom_converter(result, command_for_result, self.executor).await
            } else {
                let result = self
                    .executor
                    .send(self.command, self.retry_on_error)
                    .await?;
                result.to()
            }
        })
    }
}

impl<'a> BitmapCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> BloomCommands<'a> for &'a Client {}
impl<'a> ClusterCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> CountMinSketchCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> CuckooCommands<'a> for &'a Client {}
impl<'a> ConnectionCommands<'a> for &'a Client {}
#[cfg(test)]
impl<'a> DebugCommands<'a> for &'a Client {}
impl<'a> GenericCommands<'a> for &'a Client {}
impl<'a> GeoCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
impl<'a> GraphCommands<'a> for &'a Client {}
impl<'a> HashCommands<'a> for &'a Client {}
impl<'a> HyperLogLogCommands<'a> for &'a Client {}
impl<'a> InternalPubSubCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
impl<'a> JsonCommands<'a> for &'a Client {}
impl<'a> ListCommands<'a> for &'a Client {}
impl<'a> ScriptingCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
impl<'a> SearchCommands<'a> for &'a Client {}
impl<'a> SentinelCommands<'a> for &'a Client {}
impl<'a> ServerCommands<'a> for &'a Client {}
impl<'a> SetCommands<'a> for &'a Client {}
impl<'a> SortedSetCommands<'a> for &'a Client {}
impl<'a> StreamCommands<'a> for &'a Client {}
impl<'a> StringCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> TDigestCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
impl<'a> TimeSeriesCommands<'a> for &'a Client {}
impl<'a> TransactionCommands<'a> for &'a Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> TopKCommands<'a> for &'a Client {}

impl<'a> PubSubCommands<'a> for &'a Client {
    #[inline]
    async fn subscribe<C, CC>(self, channels: CC) -> Result<PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgCollection<C>,
    {
        let channels = CommandArgs::default().arg(channels).build();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) = mpsc::unbounded();

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
    async fn psubscribe<P, PP>(self, patterns: PP) -> Result<PubSubStream>
    where
        P: SingleArg + Send + 'a,
        PP: SingleArgCollection<P>,
    {
        let patterns = CommandArgs::default().arg(patterns).build();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) = mpsc::unbounded();

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
    async fn ssubscribe<C, CC>(self, shardchannels: CC) -> Result<PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgCollection<C>,
    {
        let shardchannels = CommandArgs::default().arg(shardchannels).build();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) = mpsc::unbounded();

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
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) = oneshot::channel();
        let (push_sender, push_receiver): (PushSender, PushReceiver) = mpsc::unbounded();

        let message = Message::monitor(cmd("MONITOR"), result_sender, push_sender);

        self.send_message(message)?;

        let _bytes = result_receiver.await??;
        Ok(MonitorStream::new(push_receiver, self.clone()))
    }
}
