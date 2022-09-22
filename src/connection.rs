use crate::{
    cmd,
    message::Message,
    resp::{BulkString, FromValue, ResultValueExt, Value},
    BitmapCommands, Command, CommandResult, ConnectionCommands,
    ConnectionResult, Future, GenericCommands, GeoCommands, HashCommands, HyperLogLogCommands,
    ListCommands, MsgSender, NetworkHandler, PrepareCommand, PubSubCommands,
    PubSubReceiver, PubSubSender, PubSubStream, Result, ScriptingCommands, ServerCommands,
    SetCommands, SortedSetCommands, StreamCommands, StringCommands, TransactionCommands,
    ValueReceiver, ValueSender, Transaction, TransactionResult0,
};
use futures::channel::{mpsc, oneshot};
use std::sync::Arc;


#[derive(Clone)]
pub struct Connection {
    msg_sender: Arc<MsgSender>,
}

impl Connection {
    /// Establish connection to the Redis server
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    pub async fn connect(addr: impl Into<String>) -> Result<Self> {
        let msg_sender = NetworkHandler::connect(addr).await?;

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
    /// use redis_driver::{cmd, Connection, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let connection = Connection::connect("127.0.0.1:6379").await?;
    ///
    ///    let values: Vec<String> = connection
    ///         .send(cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"))
    ///         .await?
    ///         .into()?;
    ///     println!("{:?}", values);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn send(&self, command: Command) -> Result<Value> {
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
    pub fn send_and_forget(&self, command: Command) -> Result<()> {
        let message = Message::new(command);
        self.send_message(message)?;
        Ok(())
    }

    /// Create a new transaction
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error)
    pub async fn create_transaction(&self) -> Result<Transaction<TransactionResult0>> {
        Transaction::initialize(self.clone()).await
    }

    fn send_message(&self, message: Message) -> Result<()> {
        self.msg_sender.unbounded_send(message)?;
        Ok(())
    }
}

impl PrepareCommand<ConnectionResult> for Connection {
    fn prepare_command<R: FromValue>(
        &self,
        command: Command,
    ) -> CommandResult<ConnectionResult, R> {
        CommandResult::from_connection(command, self)
    }
}

impl BitmapCommands<ConnectionResult> for Connection {}
impl ConnectionCommands<ConnectionResult> for Connection {}
impl GenericCommands<ConnectionResult> for Connection {}
impl GeoCommands<ConnectionResult> for Connection {}
impl HashCommands<ConnectionResult> for Connection {}
impl HyperLogLogCommands<ConnectionResult> for Connection {}
impl ListCommands<ConnectionResult> for Connection {}
impl ScriptingCommands<ConnectionResult> for Connection {}
impl ServerCommands<ConnectionResult> for Connection {}
impl SetCommands<ConnectionResult> for Connection {}
impl SortedSetCommands<ConnectionResult> for Connection {}
impl StreamCommands<ConnectionResult> for Connection {}
impl StringCommands<ConnectionResult> for Connection {}
impl TransactionCommands<ConnectionResult> for Connection {}

impl PubSubCommands<ConnectionResult> for Connection {
    fn subscribe<'a, C>(&'a self, channel: C) -> Future<'a, PubSubStream>
    where
        C: Into<BulkString> + Send + 'a,
    {
        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

            let channel: BulkString = channel.into();
            let channel_name = channel.to_string();
            let message = Message::new(cmd("SUBSCRIBE").arg(channel))
                .value_sender(value_sender)
                .pub_sub_sender(pub_sub_sender);

            self.send_message(message)?;

            let value = value_receiver.await?;
            value.map_into_result(|_| {
                PubSubStream::new(channel_name, pub_sub_receiver, self.clone())
            })
        })
    }
}
