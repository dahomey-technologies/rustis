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
        ClientState, IntoConfig, Message, MonitorStream, Pipeline, PreparedCommand, PubSubStream,
        Transaction,
    },
    commands::{
        BitmapCommands, BlockingCommands, ClusterCommands, ConnectionCommands, GenericCommands,
        GeoCommands, HashCommands, HyperLogLogCommands, InternalPubSubCommands, ListCommands,
        PubSubCommands, ScriptingCommands, SentinelCommands, ServerCommands, SetCommands,
        SortedSetCommands, StreamCommands, StringCommands, TransactionCommands,
    },
    network::{
        JoinHandle, MonitorReceiver, MonitorSender, MsgSender, NetworkHandler, PubSubReceiver,
        PubSubSender, ReconnectReceiver, ReconnectSender,
    },
    resp::{
        cmd, Command, CommandArgs, FromValue, ResultValueExt, SingleArg, SingleArgCollection, Value,
    },
    Error, Future, Result, ValueReceiver, ValueSender,
};
use futures::{channel::{mpsc, oneshot}};
use std::{future::IntoFuture, sync::Arc};

/// Client with a unique connection to a Redis server.
pub struct Client {
    msg_sender: Arc<Option<MsgSender>>,
    network_task_join_handle: Arc<Option<JoinHandle<()>>>,
    reconnect_sender: ReconnectSender,
    client_state: ClientState,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Self {
            msg_sender: self.msg_sender.clone(),
            network_task_join_handle: self.network_task_join_handle.clone(),
            reconnect_sender: self.reconnect_sender.clone(),
            client_state: ClientState::new(),
        }
    }
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
        let (msg_sender, network_task_join_handle, reconnect_sender) =
            NetworkHandler::connect(config.into_config()?).await?;

        Ok(Self {
            msg_sender: Arc::new(Some(msg_sender)),
            network_task_join_handle: Arc::new(Some(network_task_join_handle)),
            reconnect_sender,
            client_state: ClientState::new(),
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
    pub fn on_reconnect(&self) ->  ReconnectReceiver  {
        self.reconnect_sender.subscribe()
    }

    /// Give a generic access to attach any state to a client instance
    pub fn get_client_state(&mut self) -> &mut ClientState {
        &mut self.client_state
    }

    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    ///
    /// # Example
    /// ```
    /// use rustis::{client::Client, resp::cmd, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     let values: Vec<String> = client
    ///         .send(cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"))
    ///         .await?
    ///         .into()?;
    ///     println!("{:?}", values);
    ///
    ///     Ok(())
    /// }
    /// ```

    #[inline]
    pub async fn send(&mut self, command: Command) -> Result<Value> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
        let message = Message::single(command, value_sender);
        self.send_message(message)?;
        let value = value_receiver.await?;
        value.into_result()
    }

    /// Send command to the Redis server and forget its response.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    #[inline]
    pub fn send_and_forget(&mut self, command: Command) -> Result<()> {
        let message = Message::single_forget(command);
        self.send_message(message)?;
        Ok(())
    }

    /// Send a batch of commands to the Redis server.
    ///
    /// # Arguments
    /// * `commands` - batch of generic [`Command`](crate::resp::Command)s meant to be sent to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    #[inline]
    pub async fn send_batch(&mut self, commands: Vec<Command>) -> Result<Value> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
        let message = Message::batch(commands, value_sender);
        self.send_message(message)?;
        let value = value_receiver.await?;
        value.into_result()
    }

    #[inline]
    fn send_message(&mut self, message: Message) -> Result<()> {
        if let Some(msg_sender) = &self.msg_sender as &Option<MsgSender> {
            msg_sender.unbounded_send(message)?;
            Ok(())
        } else {
            Err(Error::Client(
                "Invalid channel to send messages to the network handler".to_owned(),
            ))
        }
    }

    /// Create a new transaction
    #[inline]
    pub fn create_transaction(&mut self) -> Transaction {
        Transaction::new(self.clone())
    }

    /// Create a new pipeline
    #[inline]
    pub fn create_pipeline(&mut self) -> Pipeline {
        Pipeline::new(self.clone())
    }

    pub(crate) async fn subscribe_from_pub_sub_sender(
        &mut self,
        channels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();

        let pub_sub_senders = channels
            .iter()
            .map(|c| (c.as_bytes().to_vec(), pub_sub_sender.clone()))
            .collect::<Vec<_>>();

        let message = Message::pub_sub(
            cmd("SUBSCRIBE").arg(channels.clone()),
            value_sender,
            pub_sub_senders,
        );

        self.send_message(message)?;

        let value = value_receiver.await?;
        value.map_into_result(|_| ())
    }

    pub(crate) async fn psubscribe_from_pub_sub_sender(
        &mut self,
        patterns: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();

        let pub_sub_senders = patterns
            .iter()
            .map(|c| (c.as_bytes().to_vec(), pub_sub_sender.clone()))
            .collect::<Vec<_>>();

        let message = Message::pub_sub(
            cmd("PSUBSCRIBE").arg(patterns.clone()),
            value_sender,
            pub_sub_senders,
        );

        self.send_message(message)?;

        let value = value_receiver.await?;
        value.map_into_result(|_| ())
    }

    pub(crate) async fn ssubscribe_from_pub_sub_sender(
        &mut self,
        shardchannels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();

        let pub_sub_senders = shardchannels
            .iter()
            .map(|c| (c.as_bytes().to_vec(), pub_sub_sender.clone()))
            .collect::<Vec<_>>();

        let message = Message::pub_sub(
            cmd("SSUBSCRIBE").arg(shardchannels.clone()),
            value_sender,
            pub_sub_senders,
        );

        self.send_message(message)?;

        let value = value_receiver.await?;
        value.map_into_result(|_| ())
    }
}

/// Extension trait dedicated to [`PreparedCommand`](crate::client::PreparedCommand)
/// to add specific methods for the [`Client`](crate::client::Client) executor
pub trait ClientPreparedCommand<'a, R>
where
    R: FromValue,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R> ClientPreparedCommand<'a, R> for PreparedCommand<'a, Client, R>
where
    R: FromValue + Send + 'a,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()> {
        self.executor.send_and_forget(self.command)
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, Client, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            if self.keep_command_for_result {
                let command_for_result = self.command.clone();
                self.executor
                    .send(self.command)
                    .await?
                    .into_with_command(&command_for_result)
            } else if let Some(post_process) = self.post_process {
                let command_for_result = self.command.clone();
                let result = self.executor.send(self.command).await?;
                post_process(result, command_for_result, self.executor).await
            } else {
                self.executor.send(self.command).await?.into()
            }
        })
    }
}

impl BitmapCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl BloomCommands for Client {}
impl ClusterCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl CountMinSketchCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl CuckooCommands for Client {}
impl ConnectionCommands for Client {}
impl GenericCommands for Client {}
impl GeoCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
impl GraphCommands for Client {}
impl HashCommands for Client {}
impl HyperLogLogCommands for Client {}
impl InternalPubSubCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
impl JsonCommands for Client {}
impl ListCommands for Client {}
impl ScriptingCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
impl SearchCommands for Client {}
impl SentinelCommands for Client {}
impl ServerCommands for Client {}
impl SetCommands for Client {}
impl SortedSetCommands for Client {}
impl StreamCommands for Client {}
impl StringCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl TDigestCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
impl TimeSeriesCommands for Client {}
impl TransactionCommands for Client {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl TopKCommands for Client {}

impl PubSubCommands for Client {
    #[inline]
    fn subscribe<'a, C, CC>(&'a mut self, channels: CC) -> Future<'a, PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgCollection<C>,
    {
        let channels = channels.into_args(CommandArgs::Empty);

        Box::pin(async move {
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

            self.subscribe_from_pub_sub_sender(&channels, &pub_sub_sender)
                .await?;

            Ok(PubSubStream::from_channels(
                channels,
                pub_sub_sender,
                pub_sub_receiver,
                self.clone(),
            ))
        })
    }

    #[inline]
    fn psubscribe<'a, P, PP>(&'a mut self, patterns: PP) -> Future<'a, PubSubStream>
    where
        P: SingleArg + Send + 'a,
        PP: SingleArgCollection<P>,
    {
        let patterns = patterns.into_args(CommandArgs::Empty);

        Box::pin(async move {
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

            self.psubscribe_from_pub_sub_sender(&patterns, &pub_sub_sender)
                .await?;

            Ok(PubSubStream::from_patterns(
                patterns,
                pub_sub_sender,
                pub_sub_receiver,
                self.clone(),
            ))
        })
    }

    #[inline]
    fn ssubscribe<'a, C, CC>(&'a mut self, shardchannels: CC) -> Future<'a, PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgCollection<C>,
    {
        let shardchannels = shardchannels.into_args(CommandArgs::Empty);

        Box::pin(async move {
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

            self.ssubscribe_from_pub_sub_sender(&shardchannels, &pub_sub_sender)
                .await?;

            Ok(PubSubStream::from_shardchannels(
                shardchannels,
                pub_sub_sender,
                pub_sub_receiver,
                self.clone(),
            ))
        })
    }
}

impl BlockingCommands for Client {
    fn monitor(&mut self) -> Future<MonitorStream> {
        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (monitor_sender, monitor_receiver): (MonitorSender, MonitorReceiver) =
                mpsc::unbounded();

            let message = Message::monitor(cmd("MONITOR"), value_sender, monitor_sender);

            self.send_message(message)?;

            let value = value_receiver.await?;
            value.map_into_result(|_| MonitorStream::new(monitor_receiver, self.clone()))
        })
    }
}
