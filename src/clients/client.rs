use crate::{
    network::{MonitorReceiver, MonitorSender},
    resp::{cmd, BulkString, Command, FromValue, ResultValueExt, SingleArgOrCollection, Value},
    BitmapCommands, BlockingCommands, CommandResult, ConnectionCommands, Error, Future,
    GenericCommands, GeoCommands, HashCommands, HyperLogLogCommands, InternalPubSubCommands,
    IntoConfig, ListCommands, Message, MonitorStream, MsgSender, NetworkHandler, PrepareCommand,
    PubSubCommands, PubSubReceiver, PubSubSender, PubSubStream, Result, ScriptingCommands,
    SentinelCommands, ServerCommands, SetCommands, SortedSetCommands, StreamCommands,
    StringCommands, Transaction, TransactionCommands, TransactionResult0, ValueReceiver,
    ValueSender,
};
use futures::channel::{mpsc, oneshot};
use std::{
    future::{ready, IntoFuture},
    sync::Arc,
};

/// Client with a unique connection to a Redis server.
pub struct Client {
    msg_sender: Arc<MsgSender>,
}

impl Client {
    /// Connects asynchronously to the Redis server.
    /// 
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    pub async fn connect(config: impl IntoConfig) -> Result<Self> {
        let msg_sender = NetworkHandler::connect(config.into_config()?).await?;

        Ok(Self {
            msg_sender: Arc::new(msg_sender),
        })
    }

    /// We don't want the Client struct to be publicly cloneable
    /// If one wants to consume a multiplexed client, 
    /// the [MultiplexedClient](crate::MultiplexedClient) must be used instead
    pub(crate) fn clone(&self) -> Client {
        Client {
            msg_sender: self.msg_sender.clone()
        }
    }

    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `name` - Command name in uppercase.
    /// * `args` - Command arguments which can be provided as arrays (up to 4 elements) or vectors of [`BulkString`](crate::resp::BulkString).
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    ///
    /// # Example
    /// ```
    /// use redis_driver::{resp::cmd, Client, Result};
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
    pub async fn send(&mut self, command: Command) -> Result<Value> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
        let message = Message::new(command).value_sender(value_sender);
        self.send_message(message)?;
        let value = value_receiver.await?;
        value.into_result()
    }

    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    pub fn send_and_forget(&mut self, command: Command) -> Result<()> {
        let message = Message::new(command);
        self.send_message(message)?;
        Ok(())
    }

    /// Create a new transaction
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error)
    pub async fn create_transaction(&mut self) -> Result<Transaction<TransactionResult0>> {
        Transaction::initialize(self.clone()).await
    }

    fn send_message(&mut self, message: Message) -> Result<()> {
        self.msg_sender.unbounded_send(message)?;
        Ok(())
    }

    /// Subscribes the client to the specified channels.
    ///
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     resp::cmd, Client, ClientCommandResult, FlushingMode,
    ///     PubSubCommands, ServerCommands, Result
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
    pub fn subscribe<'a, C, CC>(&'a mut self, channels: CC) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + Send + 'a,
        CC: SingleArgOrCollection<C>,
    {
        let channels: Vec<String> = channels.into_iter().map(|c| c.into().to_string()).collect();

        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

            let pub_sub_senders = channels
                .iter()
                .map(|c| (c.as_bytes().to_vec(), pub_sub_sender.clone()))
                .collect::<Vec<_>>();

            let message = Message::new(cmd("SUBSCRIBE").arg(channels.clone()))
                .value_sender(value_sender)
                .pub_sub_senders(pub_sub_senders);

            self.send_message(message)?;

            let value = value_receiver.await?;
            value.map_into_result(|_| {
                PubSubStream::from_channels(channels, pub_sub_receiver, self.clone())
            })
        })
    }

    /// Subscribes the client to the given patterns.
    ///
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     resp::cmd, Client, ClientCommandResult, FlushingMode,
    ///     PubSubCommands, ServerCommands, Result
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
    pub fn psubscribe<'a, P, PP>(&'a mut self, patterns: PP) -> Future<'a, PubSubStream>
    where
        P: Into<BulkString> + Send + 'a,
        PP: SingleArgOrCollection<P>,
    {
        let patterns: Vec<String> = patterns.into_iter().map(|p| p.into().to_string()).collect();

        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

            let pub_sub_senders = patterns
                .iter()
                .map(|c| (c.as_bytes().to_vec(), pub_sub_sender.clone()))
                .collect::<Vec<_>>();

            let message = Message::new(cmd("PSUBSCRIBE").arg(patterns.clone()))
                .value_sender(value_sender)
                .pub_sub_senders(pub_sub_senders);

            self.send_message(message)?;

            let value = value_receiver.await?;
            value.map_into_result(|_| {
                PubSubStream::from_patterns(patterns, pub_sub_receiver, self.clone())
            })
        })
    }
}

impl PrepareCommand<ClientResult> for Client {
    fn prepare_command<R: FromValue>(
        &mut self,
        command: Command,
    ) -> CommandResult<ClientResult, R> {
        CommandResult::from_client(command, self)
    }
}

pub struct ClientResult;

#[allow(clippy::module_name_repetitions)]
pub trait ClientCommandResult<'a, R>
where
    R: FromValue,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R> ClientCommandResult<'a, R> for CommandResult<'a, ClientResult, R>
where
    R: FromValue + Send + 'a,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()> {
        if let CommandResult::Client(_, command, client) = self {
            client.send_and_forget(command)
        } else {
            Err(Error::Client(
                "send_and_forget method must be called with a valid client".to_owned(),
            ))
        }
    }
}

impl<'a, R> IntoFuture for CommandResult<'a, ClientResult, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        if let CommandResult::Client(_, command, client) = self {
            Box::pin(async move { client.send(command).await?.into() })
        } else {
            Box::pin(ready(Err(Error::Client(
                "send method must be called with a valid client".to_owned(),
            ))))
        }
    }
}

impl BitmapCommands<ClientResult> for Client {}
impl ConnectionCommands<ClientResult> for Client {}
impl GenericCommands<ClientResult> for Client {}
impl GeoCommands<ClientResult> for Client {}
impl HashCommands<ClientResult> for Client {}
impl HyperLogLogCommands<ClientResult> for Client {}
impl InternalPubSubCommands<ClientResult> for Client {}
impl ListCommands<ClientResult> for Client {}
impl PubSubCommands<ClientResult> for Client {}
impl ScriptingCommands<ClientResult> for Client {}
impl SentinelCommands<ClientResult> for Client {}
impl ServerCommands<ClientResult> for Client {}
impl SetCommands<ClientResult> for Client {}
impl SortedSetCommands<ClientResult> for Client {}
impl StreamCommands<ClientResult> for Client {}
impl StringCommands<ClientResult> for Client {}
impl TransactionCommands<ClientResult> for Client {}

impl BlockingCommands<ClientResult> for Client {
    fn monitor(&mut self) -> Future<crate::MonitorStream> {
        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (monitor_sender, monitor_receiver): (MonitorSender, MonitorReceiver) =
                mpsc::unbounded();

            let message = Message::new(cmd("MONITOR"))
                .value_sender(value_sender)
                .monitor_sender(monitor_sender);

            self.send_message(message)?;

            let value = value_receiver.await?;
            value.map_into_result(|_| MonitorStream::new(monitor_receiver, self.clone()))
        })
    }
}
