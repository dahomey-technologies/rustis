use serde::{
    de::{self, DeserializeOwned, DeserializeSeed, IgnoredAny, SeqAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};

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
    client::{BatchPreparedCommand, Client, PreparedCommand},
    commands::{
        BitmapCommands, GenericCommands, GeoCommands, HashCommands, HyperLogLogCommands,
        ListCommands, ScriptingCommands, ServerCommands, SetCommands, SortedSetCommands,
        StreamCommands, StringCommands,
    },
    resp::{cmd, Command, RespDeserializer, Response},
    Error, Result,
};
use std::{fmt, marker::PhantomData};

/// Represents an on-going [`transaction`](https://redis.io/docs/manual/transactions/) on a specific client instance.
pub struct Transaction {
    client: Client,
    commands: Vec<Command>,
    forget_flags: Vec<bool>,
    retry_on_error: Option<bool>,
}

impl Transaction {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            commands: vec![cmd("MULTI")],
            forget_flags: Vec::new(),
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
    pub fn queue(&mut self, command: Command) {
        self.commands.push(command);
        self.forget_flags.push(false);
    }

    /// Queue a command into the transaction and forget its response.
    pub fn forget(&mut self, command: Command) {
        self.commands.push(command);
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
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     let mut transaction = client.create_transaction();
    ///
    ///     transaction.set("key1", "value1").forget();
    ///     transaction.set("key2", "value2").forget();
    ///     transaction.get::<_, String>("key1").queue();
    ///     let value: String = transaction.execute().await?;
    ///
    ///     assert_eq!("value1", value);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute<T: DeserializeOwned>(mut self) -> Result<T> {
        self.commands.push(cmd("EXEC"));

        let num_commands = self.commands.len();

        let results = self
            .client
            .send_batch(self.commands, self.retry_on_error)
            .await?;

        let mut iter = results.into_iter();

        // MULTI + QUEUED commands
        for _ in 0..num_commands - 1 {
            if let Some(resp_buf) = iter.next() {
                resp_buf.to::<()>()?;
            }
        }

        // EXEC
        if let Some(result) = iter.next() {
            let mut deserializer = RespDeserializer::new(&result);
            match TransactionResultSeed::new(self.forget_flags).deserialize(&mut deserializer) {
                Ok(Some(t)) => Ok(t),
                Ok(None) => Err(Error::Aborted),
                Err(e) => Err(e),
            }
        } else {
            Err(Error::Client(
                "Unexpected result for transaction".to_owned(),
            ))
        }
    }
}

struct TransactionResultSeed<T: DeserializeOwned> {
    phantom: PhantomData<T>,
    forget_flags: Vec<bool>,
}

impl<T: DeserializeOwned> TransactionResultSeed<T> {
    pub fn new(forget_flags: Vec<bool>) -> Self {
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
    forget_flags: std::vec::IntoIter<bool>,
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

impl<'a> BitmapCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> BloomCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> CountMinSketchCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> CuckooCommands<'a> for &'a mut Transaction {}
impl<'a> GenericCommands<'a> for &'a mut Transaction {}
impl<'a> GeoCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
impl<'a> GraphCommands<'a> for &'a mut Transaction {}
impl<'a> HashCommands<'a> for &'a mut Transaction {}
impl<'a> HyperLogLogCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
impl<'a> JsonCommands<'a> for &'a mut Transaction {}
impl<'a> ListCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
impl<'a> SearchCommands<'a> for &'a mut Transaction {}
impl<'a> SetCommands<'a> for &'a mut Transaction {}
impl<'a> ScriptingCommands<'a> for &'a mut Transaction {}
impl<'a> ServerCommands<'a> for &'a mut Transaction {}
impl<'a> SortedSetCommands<'a> for &'a mut Transaction {}
impl<'a> StreamCommands<'a> for &'a mut Transaction {}
impl<'a> StringCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> TDigestCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
impl<'a> TimeSeriesCommands<'a> for &'a mut Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl<'a> TopKCommands<'a> for &'a mut Transaction {}
