use crate::{
    client::{
        Cache, ClientPreparedCommand, IntoConfig, Message, Pipeline, PreparedCommand, PubSubStream,
        Transaction,
    },
    commands::InternalPubSubCommands,
    network::{PubSubReceiver, PubSubSender},
    resp::{cmd, Command, FromValue, ResultValueExt, SingleArg, SingleArgOrCollection, Value},
    Future, MsgSender, NetworkHandler, Result, ValueReceiver, ValueSender,
};
use futures::channel::{mpsc, oneshot};
use std::{future::IntoFuture, sync::Arc};

pub(crate) struct InnerClient {
    msg_sender: Arc<MsgSender>,
    cache: Cache,
}

impl Clone for InnerClient {
    fn clone(&self) -> Self {
        Self {
            msg_sender: self.msg_sender.clone(),
            cache: Cache::new(),
        }
    }
}

impl InnerClient {
    /// Connects asynchronously to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    pub async fn connect(config: impl IntoConfig) -> Result<Self> {
        let msg_sender = NetworkHandler::connect(config.into_config()?).await?;

        Ok(Self {
            msg_sender: Arc::new(msg_sender),
            cache: Cache::new(),
        })
    }

    pub fn get_cache(&mut self) -> &mut Cache {
        &mut self.cache
    }

    pub async fn send(&mut self, command: Command) -> Result<Value> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
        let message = Message::single(command, value_sender);
        self.send_message(message)?;
        let value = value_receiver.await?;
        value.into_result()
    }

    pub fn send_and_forget(&mut self, command: Command) -> Result<()> {
        let message = Message::single_forget(command);
        self.send_message(message)?;
        Ok(())
    }

    pub async fn send_batch(&mut self, commands: Vec<Command>) -> Result<Value> {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
        let message = Message::batch(commands, value_sender);
        self.send_message(message)?;
        let value = value_receiver.await?;
        value.into_result()
    }

    pub fn send_message(&mut self, message: Message) -> Result<()> {
        self.msg_sender.unbounded_send(message)?;
        Ok(())
    }

    pub fn create_transaction(&mut self) -> Transaction {
        Transaction::new(self.clone())
    }

    pub fn create_pipeline(&mut self) -> Pipeline {
        Pipeline::new(self.clone())
    }

    pub fn subscribe<'a, C, CC>(&'a mut self, channels: CC) -> Future<'a, PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgOrCollection<C>,
    {
        let channels: Vec<String> = channels
            .into_iter()
            .map(|c| c.into_command_arg().to_string())
            .collect();

        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

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
            value.map_into_result(|_| {
                PubSubStream::from_channels(channels, pub_sub_receiver, self.clone())
            })
        })
    }

    pub fn psubscribe<'a, P, PP>(&'a mut self, patterns: PP) -> Future<'a, PubSubStream>
    where
        P: SingleArg + Send + 'a,
        PP: SingleArgOrCollection<P>,
    {
        let patterns: Vec<String> = patterns
            .into_iter()
            .map(|p| p.into_command_arg().to_string())
            .collect();

        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

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
            value.map_into_result(|_| {
                PubSubStream::from_patterns(patterns, pub_sub_receiver, self.clone())
            })
        })
    }

    pub fn ssubscribe<'a, C, CC>(&'a mut self, shardchannels: CC) -> Future<'a, PubSubStream>
    where
        C: SingleArg + Send + 'a,
        CC: SingleArgOrCollection<C>,
    {
        let shardchannels: Vec<String> = shardchannels
            .into_iter()
            .map(|c| c.into_command_arg().to_string())
            .collect();

        Box::pin(async move {
            let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();
            let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
                mpsc::unbounded();

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
            value.map_into_result(|_| {
                PubSubStream::from_shardchannels(shardchannels, pub_sub_receiver, self.clone())
            })
        })
    }
}

impl<'a, R> ClientPreparedCommand<'a, R> for PreparedCommand<'a, InnerClient, R>
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

impl<'a, R> IntoFuture for PreparedCommand<'a, InnerClient, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.executor.send(self.command).await?.into() })
    }
}

impl InternalPubSubCommands for InnerClient {}
