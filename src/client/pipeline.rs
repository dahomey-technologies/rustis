use crate::{
    Result,
    client::{Client, PreparedCommand},
    commands::{
        ArrayCommands, BitmapCommands, BloomCommands, ClusterCommands, ConnectionCommands,
        CountMinSketchCommands, CuckooCommands, GenericCommands, GeoCommands, HashCommands,
        HyperLogLogCommands, JsonCommands, ListCommands, ScriptingCommands, SearchCommands,
        ServerCommands, SetCommands, SortedSetCommands, StreamCommands, StringCommands,
        TDigestCommands, TimeSeriesCommands, TopKCommands, VectorSetCommands,
    },
    resp::{Command, RespBatchDeserializer, Response},
};
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

    /// Queue a command
    pub fn queue(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(false);
    }

    /// Queue a command and forget its response
    pub fn forget(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(true);
    }

    /// Execute the pipeline by the sending the queued command
    /// as a whole batch to the Redis server.
    ///
    /// # Return
    /// It is the caller responsability to use the right type to cast the server response
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
    ///     pipeline.get::<String>("key1").queue();
    ///     pipeline.get::<String>("key2").queue();
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

        // Past this point the command names have served their purpose for the
        // single-response path below; the batch deserializer reports on the
        // whole reply, which belongs to no single command.
        let (results, command_names): (Vec<_>, Vec<_>) = results.into_iter().unzip();

        // A single response deserializes directly as `T` rather than as a
        // one-element batch. Peeling it off with `pop` inside the condition
        // rather than after it keeps the emptiness of `results` the only thing
        // this branch depends on, with no length invariant left to assert.
        let mut results = results;
        if results.len() == 1
            && let Some(result) = results.pop()
        {
            let named = result.to();
            return match (named, command_names.into_iter().next()) {
                (Err(e), Some(command)) => Err(e.with_command(command)),
                (named, _) => named,
            };
        }

        let deserializer = RespBatchDeserializer::new(&results);
        T::deserialize(&deserializer)
    }
}

/// Extension trait dedicated to [`PreparedCommand`](crate::client::PreparedCommand)
/// to add specific methods for the [`Pipeline`](crate::client::Pipeline) &
/// the [`Transaction`](crate::client::Transaction) executors
pub trait BatchPreparedCommand<R = ()> {
    /// Queue a command.
    fn queue(self);

    /// Queue a command and forget its response.
    fn forget(self);
}

impl<'a, R: Response> BatchPreparedCommand for PreparedCommand<'a, &'a mut Pipeline<'_>, R> {
    /// Queue a command.
    #[inline]
    fn queue(self) {
        self.executor.queue(self.command)
    }

    /// Queue a command and forget its response.
    #[inline]
    fn forget(self) {
        self.executor.forget(self.command)
    }
}

impl<'a> ArrayCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> BitmapCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> BloomCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> ClusterCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> ConnectionCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> CountMinSketchCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> CuckooCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> GenericCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> GeoCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> HashCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> HyperLogLogCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> JsonCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> ListCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> SearchCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> SetCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> ScriptingCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> ServerCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> SortedSetCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> StreamCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> StringCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> TDigestCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> TimeSeriesCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> TopKCommands<'a> for &'a mut Pipeline<'_> {}
impl<'a> VectorSetCommands<'a> for &'a mut Pipeline<'_> {}
