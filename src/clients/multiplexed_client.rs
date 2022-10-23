use crate::{
    resp::{cmd, BulkString, Command, FromValue, ResultValueExt, SingleArgOrCollection, Value},
    BitmapCommands, ConnectionCommands, Future, GenericCommands, GeoCommands, HashCommands,
    HyperLogLogCommands, InternalPubSubCommands, IntoConfig, ListCommands, Message, MsgSender,
    MultiplexedPubSubStream, NetworkHandler, PreparedCommand, PubSubCommands, PubSubReceiver,
    PubSubSender, Result, ScriptingCommands, SentinelCommands, ServerCommands, SetCommands,
    SortedSetCommands, StreamCommands, StringCommands, ValueReceiver, ValueSender,
};
use futures::channel::{mpsc, oneshot};
use std::{future::IntoFuture, sync::Arc};

/// A multiplexed client that can be cloned, allowing requests
/// to be be sent concurrently on the same underlying connection.
///
/// Compared to a [single client](crate::Client), a multiplexed client cannot offers access
/// to all existing Redis commands.
/// Transactions and [blocking commands](crate::BlockingCommands) are not compatible with a multiplexed client
/// because they monopolize the whole connection which cannot be shared anymore. It means other consumers of the same
/// multiplexed client will be blocked each time a transaction or a blocking command is in progress, losing the advantage
/// of a shared connection.
///
/// #See also [Multiplexing Explained](https://redis.com/blog/multiplexing-explained/)
#[derive(Clone)]
pub struct MultiplexedClient {
    msg_sender: Arc<MsgSender>,
}

impl MultiplexedClient {
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

    fn send_message(&mut self, message: Message) -> Result<()> {
        self.msg_sender.unbounded_send(message)?;
        Ok(())
    }

    /// Subscribes the client to the specified channels.
    ///
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     resp::cmd, MultiplexedClient, MultiplexedClientCommandResult, FlushingMode,
    ///     PubSubCommands, ServerCommands, Result
    /// };
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut pub_sub_client = MultiplexedClient::connect("127.0.0.1:6379").await?;
    ///     let mut regular_client = MultiplexedClient::connect("127.0.0.1:6379").await?;
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
    pub fn subscribe<'a, C, CC>(&'a mut self, channels: CC) -> Future<'a, MultiplexedPubSubStream>
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
                MultiplexedPubSubStream::from_channels(channels, pub_sub_receiver, self.clone())
            })
        })
    }

    /// Subscribes the client to the given patterns.
    ///
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     resp::cmd, MultiplexedClient, MultiplexedClientCommandResult, FlushingMode,
    ///     PubSubCommands, ServerCommands, Result
    /// };
    /// use futures::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut pub_sub_client = MultiplexedClient::connect("127.0.0.1:6379").await?;
    ///     let mut regular_client = MultiplexedClient::connect("127.0.0.1:6379").await?;
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
    pub fn psubscribe<'a, P, PP>(&'a mut self, patterns: PP) -> Future<'a, MultiplexedPubSubStream>
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
                MultiplexedPubSubStream::from_patterns(patterns, pub_sub_receiver, self.clone())
            })
        })
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait MultiplexedClientCommandResult<'a, R>
where
    R: FromValue,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R> MultiplexedClientCommandResult<'a, R> for PreparedCommand<'a, MultiplexedClient, R>
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

impl<'a, R> IntoFuture for PreparedCommand<'a, MultiplexedClient, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.executor.send(self.command).await?.into() })
    }
}

impl BitmapCommands for MultiplexedClient {}
impl ConnectionCommands for MultiplexedClient {}
impl GenericCommands for MultiplexedClient {}
impl GeoCommands for MultiplexedClient {}
impl HashCommands for MultiplexedClient {}
impl HyperLogLogCommands for MultiplexedClient {}
impl InternalPubSubCommands for MultiplexedClient {}
impl ListCommands for MultiplexedClient {}
impl ScriptingCommands for MultiplexedClient {}
impl SentinelCommands for MultiplexedClient {}
impl ServerCommands for MultiplexedClient {}
impl SetCommands for MultiplexedClient {}
impl SortedSetCommands for MultiplexedClient {}
impl StreamCommands for MultiplexedClient {}
impl StringCommands for MultiplexedClient {}
impl PubSubCommands for MultiplexedClient {}
