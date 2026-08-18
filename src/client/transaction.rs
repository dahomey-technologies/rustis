use crate::{
    ClientError, Error, ErrorKind, Result,
    client::{BatchPreparedCommand, Client, PreparedCommand, command_traits::*},
    resp::{Command, RespDeserializer, Response, cmd},
};
use serde::{
    Deserializer,
    de::{self, DeserializeOwned, DeserializeSeed, IgnoredAny, SeqAccess, Visitor},
    forward_to_deserialize_any,
};
use smallvec::SmallVec;
use std::{fmt, marker::PhantomData};

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

    /// Queue a command into the transaction.
    pub fn queue(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(false);
    }

    /// Queue a command into the transaction and forget its response.
    pub fn forget(&mut self, command: impl Into<Command>) {
        self.commands.push(command.into());
        self.forget_flags.push(true);
    }

    /// Execute the transaction by the sending the queued command
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
    ///     transaction.get::<String>("key1").queue();
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
                // `results` is MULTI, then one reply per queued command, then
                // EXEC — hence the offset of one onto the queued commands.
                (Some((i, _)), None) => results.get(i + 1).map(|(_, name)| name.clone()),
                _ => None,
            }
        };

        let mut iter = results.into_iter();

        // MULTI + QUEUED commands. A server error here names the queued command
        // it refused, which is the one the caller has to fix.
        for _ in 0..num_commands - 1 {
            if let Some((response, command)) = iter.next() {
                response.to::<()>().map_err(|e| e.with_command(command))?;
            }
        }

        // EXEC
        if let Some((result, _)) = iter.next() {
            let result = match TransactionResultSeed::new(self.forget_flags)
                .deserialize(RespDeserializer::new(result.view()?))
            {
                Ok(Some(t)) => Ok(t),
                Ok(None) => Err(Error::from(ErrorKind::Aborted)),
                Err(e) => Err(e),
            };
            match (result, awaited_command) {
                (Err(e), Some(command)) => Err(e.with_command(command)),
                (result, _) => result,
            }
        } else {
            Err(Error::from(ClientError::Unexpected))
        }
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

struct TransactionResultSeed<T: DeserializeOwned> {
    phantom: PhantomData<T>,
    forget_flags: SmallVec<[bool; 10]>,
}

impl<T: DeserializeOwned> TransactionResultSeed<T> {
    pub(crate) fn new(forget_flags: SmallVec<[bool; 10]>) -> Self {
        Self {
            phantom: PhantomData,
            forget_flags,
        }
    }
}

impl<'de, T: DeserializeOwned> DeserializeSeed<'de> for TransactionResultSeed<T> {
    type Value = Option<T>;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de, T: DeserializeOwned> Visitor<'de> for TransactionResultSeed<T> {
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Option<T>")
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "one increment per flag in a list held in memory."
    )]
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        if self
            .forget_flags
            .iter()
            .fold(0, |acc, flag| if *flag { acc } else { acc + 1 })
            == 1
        {
            for forget in &self.forget_flags {
                if *forget {
                    seq.next_element::<IgnoredAny>()?;
                } else {
                    return seq.next_element::<T>();
                }
            }
            Ok(None)
        } else {
            let deserializer = SeqAccessDeserializer {
                forget_flags: self.forget_flags.into_iter(),
                seq_access: seq,
            };

            T::deserialize(deserializer)
                .map(Some)
                .map_err(de::Error::custom)
        }
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(None)
    }
}

struct SeqAccessDeserializer<A> {
    forget_flags: smallvec::IntoIter<[bool; 10]>,
    seq_access: A,
}

impl<'de, A> Deserializer<'de> for SeqAccessDeserializer<A>
where
    A: serde::de::SeqAccess<'de>,
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
        bytes byte_buf unit_struct newtype_struct string tuple
        tuple_struct map struct enum identifier ignored_any unit option
    }
}

impl<'de, A> SeqAccess<'de> for SeqAccessDeserializer<A>
where
    A: serde::de::SeqAccess<'de>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        for forget in self.forget_flags.by_ref() {
            if forget {
                self.seq_access
                    .next_element::<IgnoredAny>()
                    .map_err::<Error, _>(de::Error::custom)?;
            } else {
                return self
                    .seq_access
                    .next_element_seed(seed)
                    .map_err(de::Error::custom);
            }
        }
        Ok(None)
    }
}

impl<'a, R: Response> BatchPreparedCommand for PreparedCommand<'a, &'a mut Transaction, R> {
    /// Queue a command into the transaction.
    fn queue(self) {
        self.executor.queue(self.command)
    }

    /// Queue a command into the transaction and forget its response.
    fn forget(self) {
        self.executor.forget(self.command)
    }
}

impl_transaction_command_traits!(Transaction);
