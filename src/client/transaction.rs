use crate::{
    ClientError, Error, ErrorKind, Result,
    client::{BatchPreparedCommand, Client, PreparedCommand, command_traits::*},
    resp::{Command, RespBatchDeserializer, RespResponse, RespView, cmd},
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;

/// Represents an on-going [`transaction`](https://redis.io/docs/manual/transactions/) on a specific client instance.
pub struct Transaction {
    client: Client,
    commands: Vec<Command>,
    forget_flags: SmallVec<[bool; 10]>,
    retry_on_error: Option<bool>,
}

impl Transaction {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            commands: vec![cmd("MULTI").into()],
            forget_flags: SmallVec::new(),
            retry_on_error: None,
        }
    }

    /// Set a flag to override default `retry_on_error` behavior.
    ///
    /// See [Config::retry_on_error](crate::client::Config::retry_on_error)
    pub fn retry_on_error(&mut self, retry_on_error: bool) {
        self.retry_on_error = Some(retry_on_error);
    }

    /// Queue a command built with the generic API into the transaction.
    ///
    /// Built-in commands use
    /// [`BatchPreparedCommand::queue`](crate::client::BatchPreparedCommand::queue)
    /// instead: `transaction.get::<()>("k").queue()`. The names differ because the
    /// calls do: this one takes a command, that one consumes a prepared command.
    pub fn queue_command(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(false);
    }

    /// Queue a command built with the generic API into the transaction and
    /// forget its response.
    ///
    /// See [`Self::queue_command`] for why the name differs from
    /// [`BatchPreparedCommand::forget`](crate::client::BatchPreparedCommand::forget).
    pub fn forget_command(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(true);
    }

    /// Execute the transaction by the sending the queued command
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
    ///     client::{Client, Transaction, BatchPreparedCommand},
    ///     commands::StringCommands,
    ///     resp::{cmd, Value}, Result,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     let mut transaction = client.create_transaction();
    ///
    ///     transaction.set("key1", "value1").forget();
    ///     transaction.set("key2", "value2").forget();
    ///     transaction.get::<()>("key1").queue();
    ///     let value: String = transaction.execute().await?;
    ///
    ///     assert_eq!("value1", value);
    ///
    ///     Ok(())
    /// }
    /// ```
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`EXEC` was pushed just above, so the command count is at least 1."
    )]
    pub async fn execute<T: DeserializeOwned>(mut self) -> Result<T> {
        if self.client.is_cluster() {
            // Slots are no longer computed at command-build time; populate them
            // here (caller thread, cluster only) before the cross-slot check
            // reads them.
            for command in &mut self.commands {
                command.compute_slots();
            }
            Self::check_single_slot(&self.commands)?;
        }

        self.commands.push(cmd("EXEC").into());

        let num_commands = self.commands.len();

        // Unlike a pipeline, a transaction wants one name per command: the server
        // refuses a command by name at queue time, and the queued phase below
        // names each refusal. Taken here, from the commands, because a batch hands
        // its replies back unnamed. `forget_flags` is offset by one against this
        // list, `MULTI` occupying `commands[0]` and carrying no flag.
        let command_names: Vec<Bytes> = self.commands.iter().map(Command::name_bytes).collect();

        let results = self
            .client
            .internal_send_batch(self.commands, self.retry_on_error)
            .await?;

        // The reply the caller reads is EXEC's, whose elements are the queued
        // commands' own replies. Which command an error inside it belongs to is
        // only recoverable when exactly one command is awaited: with several,
        // the batch deserializer reports on the tuple as a whole and does not
        // say which element it stumbled on.
        let awaited_command = {
            let mut awaited = self
                .forget_flags
                .iter()
                .enumerate()
                .filter(|(_, forget)| !**forget);
            match (awaited.next(), awaited.next()) {
                // `commands` is MULTI, then the queued commands, then EXEC —
                // hence the offset of one onto the queued commands.
                (Some((i, _)), None) => command_names.get(i + 1).cloned(),
                _ => None,
            }
        };

        let mut iter = results.into_iter();

        // MULTI + QUEUED commands. A server error here names the queued command
        // it refused, which is the one the caller has to fix.
        for name in command_names.iter().take(num_commands - 1) {
            if let Some(response) = iter.next() {
                response
                    .to::<()>()
                    .map_err(|e| e.with_command(name.clone()))?;
            }
        }

        // EXEC. Its reply holds one element per queued command -- the same batch
        // shape a pipeline hands back, read by the same deserializer.
        let Some(result) = iter.next() else {
            return Err(Error::from(ClientError::MissingTransactionReply));
        };

        match (
            Self::deserialize_exec_reply(result, self.forget_flags),
            awaited_command,
        ) {
            (Err(e), Some(command)) => Err(e.with_command(command)),
            (result, _) => result,
        }
    }

    /// Reads `EXEC`'s reply as the batch of the replies the caller kept.
    ///
    /// The elements are handed out as responses of their own -- a refcount bump
    /// each, no byte copied and no value decoded -- so that the batch the caller
    /// reads is the very one [`RespBatchDeserializer`] reads for a pipeline, and
    /// a transaction retaining one reply answers `Vec<T>` and `T` the same way a
    /// pipeline does.
    fn deserialize_exec_reply<T: DeserializeOwned>(
        result: RespResponse,
        forget_flags: SmallVec<[bool; 10]>,
    ) -> Result<T> {
        // A nil `EXEC` is the server saying it dropped the transaction: a key a
        // `WATCH` was holding changed under it.
        if matches!(result.view()?, RespView::Null) {
            return Err(Error::from(ErrorKind::Aborted));
        }

        let mut forget_flags = forget_flags.into_iter();
        let replies = result
            .into_collection_iter()?
            // A forgotten reply is dropped unread, as in a pipeline: the caller
            // said it does not want it, and that covers the errors it may carry.
            // An element the flags do not cover is kept, so a reply longer than
            // the transaction reads as the mismatch it is instead of vanishing.
            .filter(|_| !forget_flags.next().unwrap_or(false))
            .collect::<Result<Vec<RespResponse>>>()?;

        let deserializer = RespBatchDeserializer::new(&replies);
        T::deserialize(&deserializer)
    }

    /// Enforce Redis Cluster's own transaction constraint: every key must hash to
    /// the same slot.
    ///
    /// In cluster mode each queued command is routed independently by its own key,
    /// while MULTI is pinned to the node of the first key-bearing command and EXEC
    /// follows that pin. A command whose slot belongs to another node is therefore
    /// sent there *outside* any MULTI and executes immediately, and the queued-phase
    /// check cannot notice: it accepts any non-error reply, so a direct command
    /// result passes for `+QUEUED`. The outcome is a partially applied transaction
    /// reported as a success. Refuse it before anything is sent.
    fn check_single_slot(commands: &[Command]) -> Result<()> {
        let mut slot: Option<u16> = None;

        for command in commands {
            for command_slot in command.slots() {
                match slot {
                    None => slot = Some(command_slot),
                    Some(slot) if slot != command_slot => {
                        return Err(Error::from(ClientError::CrossSlot));
                    }
                    Some(_) => (),
                }
            }
        }

        Ok(())
    }
}

impl<'a, R: DeserializeOwned> BatchPreparedCommand for PreparedCommand<'a, &'a mut Transaction, R> {
    /// Queue a command into the transaction.
    fn queue(self) {
        self.executor.queue_command(self.command)
    }

    /// Queue a command into the transaction and forget its response.
    fn forget(self) {
        self.executor.forget_command(self.command)
    }
}

impl_transaction_command_traits!(Transaction);
