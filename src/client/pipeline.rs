use crate::{
    Result,
    client::{Client, PreparedCommand, command_traits::*},
    resp::{Command, RespBatchDeserializer},
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;

/// Represents a Redis command pipeline.
pub struct Pipeline<'a> {
    client: &'a Client,
    commands: Vec<Command>,
    forget_flags: SmallVec<[bool; 10]>,
    retry_on_error: Option<bool>,
}

impl Pipeline<'_> {
    pub(crate) fn new<'a>(client: &'a Client) -> Pipeline<'a> {
        Pipeline {
            client,
            commands: Vec::new(),
            forget_flags: SmallVec::new(),
            retry_on_error: None,
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.commands.reserve(additional);
        self.forget_flags.reserve(additional);
    }

    /// Set a flag to override default `retry_on_error` behavior.
    ///
    /// See [Config::retry_on_error](crate::client::Config::retry_on_error)
    pub fn retry_on_error(&mut self, retry_on_error: bool) {
        self.retry_on_error = Some(retry_on_error);
    }

    /// Queue a command built with the generic API.
    ///
    /// Built-in commands use [`BatchPreparedCommand::queue`] instead:
    /// `pipeline.get::<()>("k").queue()`. The names differ because the calls do:
    /// this one takes a command, that one consumes a prepared command.
    pub fn queue_command(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(false);
    }

    /// Queue a command built with the generic API and forget its response.
    ///
    /// See [`Self::queue_command`] for why the name differs from
    /// [`BatchPreparedCommand::forget`].
    pub fn forget_command(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(true);
    }

    /// The name of the one command whose response the caller awaits, when there
    /// is exactly one.
    ///
    /// That is the only case where a name reaches the caller: with several
    /// responses [`Self::execute`] deserializes the reply as a whole, which
    /// belongs to no single command. So one name is taken here, from the command
    /// the flags say is awaited, instead of one per reply — the difference being
    /// what a pipeline of a thousand commands pays to read none of them.
    fn single_awaited_command(commands: &[Command], forget_flags: &[bool]) -> Option<Bytes> {
        let mut awaited = forget_flags
            .iter()
            .enumerate()
            .filter(|(_, forget)| !**forget);

        match (awaited.next(), awaited.next()) {
            (Some((index, _)), None) => commands.get(index).map(Command::name_bytes),
            _ => None,
        }
    }

    /// Execute the pipeline by the sending the queued command
    /// as a whole batch to the Redis server.
    ///
    /// # Return
    /// It is the caller's responsibility to use the right type to cast the server response
    /// to the right tuple or collection depending on which command has been
    /// [queued](BatchPreparedCommand::queue) or [forgotten](BatchPreparedCommand::forget).
    ///
    /// The most generic type that can be requested as a result is `Vec<resp::Value>`
    ///
    /// # Example
    /// ```
    /// use rustis::{
    ///     client::{Client, Pipeline, BatchPreparedCommand},
    ///     commands::StringCommands,
    ///     resp::{cmd, Value}, Result,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     let mut pipeline = client.create_pipeline();
    ///     pipeline.set("key1", "value1").forget();
    ///     pipeline.set("key2", "value2").forget();
    ///     pipeline.get::<()>("key1").queue();
    ///     pipeline.get::<()>("key2").queue();
    ///
    ///     let (value1, value2): (String, String) = pipeline.execute().await?;
    ///     assert_eq!("value1", value1);
    ///     assert_eq!("value2", value2);
    ///
    ///     Ok(())
    /// }
    /// ```    
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the index advances once per retained result, so it is bounded by the \
                  flag list it indexes."
    )]
    pub async fn execute<T: DeserializeOwned>(self) -> Result<T> {
        // An empty pipeline never reaches the network layer (no `MessageToReceive`
        // is created), so awaiting a result would surface an opaque channel-canceled
        // error. Resolve it as an empty batch instead.
        if self.commands.is_empty() {
            let deserializer = RespBatchDeserializer::new(&[]);
            return T::deserialize(&deserializer);
        }

        let awaited_command = Self::single_awaited_command(&self.commands, &self.forget_flags);

        let mut results = self
            .client
            .internal_send_batch(self.commands, self.retry_on_error)
            .await?;

        // Forget-flag filtering runs whenever at least one command is forgotten,
        // regardless of the batch size: a single forgotten command must have its
        // response dropped just like it would in a multi-command batch. When
        // nothing is forgotten the whole `retain` pass is skipped.
        if self.forget_flags.iter().any(|&forget| forget) {
            let mut idx = 0;
            results.retain(|_| {
                let keep = !self.forget_flags[idx];
                idx += 1;
                keep
            });
        }

        // A one-reply batch goes through the batch deserializer like any other:
        // the requested type is what decides whether that reply stands for
        // itself or for a sequence of one. See [`RespBatchDeserializer`].
        let deserializer = RespBatchDeserializer::new(&results);
        match (T::deserialize(&deserializer), awaited_command) {
            (Err(e), Some(command)) => Err(e.with_command(command)),
            (named, _) => named,
        }
    }
}

/// Extension trait dedicated to [`PreparedCommand`](crate::client::PreparedCommand)
/// to add specific methods for the [`Pipeline`](crate::client::Pipeline) &
/// the [`Transaction`](crate::client::Transaction) executors
///
/// # The response type is ignored here
///
/// Queuing discards a [`PreparedCommand`](crate::client::PreparedCommand)'s
/// response type. The type on
/// [`Pipeline::execute`](crate::client::Pipeline::execute) decides the decoding.
/// Write `::<()>` on a queued command. Any other type compiles and means nothing.
pub trait BatchPreparedCommand<R = ()> {
    /// Queue a command. Its response type is ignored — see the trait docs.
    fn queue(self);

    /// Queue a command and forget its response.
    fn forget(self);
}

impl<'a, R: DeserializeOwned> BatchPreparedCommand
    for PreparedCommand<'a, &'a mut Pipeline<'_>, R>
{
    /// Queue a command.
    #[inline]
    fn queue(self) {
        self.executor.queue_command(self.command)
    }

    /// Queue a command and forget its response.
    #[inline]
    fn forget(self) {
        self.executor.forget_command(self.command)
    }
}

impl_pipeline_command_traits!(Pipeline<'_>);
