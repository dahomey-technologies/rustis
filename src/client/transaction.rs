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
    client::{InnerClient, BatchPreparedCommand, PreparedCommand},
    commands::{
        BitmapCommands, GenericCommands, GeoCommands, HashCommands, HyperLogLogCommands,
        ListCommands, ScriptingCommands, ServerCommands, SetCommands, SortedSetCommands,
        StreamCommands, StringCommands,
    },
    resp::{cmd, Command, FromValue, ResultValueExt, Value},
    Error, Result,
};
use std::iter::zip;

/// Represents an on-going [`transaction`](https://redis.io/docs/manual/transactions/) on a specific client instance.
pub struct Transaction {
    client: InnerClient,
    commands: Vec<Command>,
    forget_flags: Vec<bool>,
}

impl Transaction {
    pub(crate) fn new(client: InnerClient) -> Transaction {
        let mut transaction = Transaction {
            client,
            commands: Vec::new(),
            forget_flags: Vec::new(),
        };

        transaction.queue(cmd("MULTI"));
        transaction
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
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut client = Client::connect("127.0.0.1:6379").await?;
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
    pub async fn execute<T: FromValue>(mut self) -> Result<T> {
        self.queue(cmd("EXEC"));

        let num_commands = self.commands.len();

        let values: Vec<Value> = self.client.send_batch(self.commands).await?.into()?;
        let mut iter = values.into_iter();

        // MULTI + QUEUED commands
        for _ in 0..num_commands - 1 {
            if let Some(Value::Error(e)) = iter.next() {
                return Err(Error::Redis(e));
            }
        }

        // EXEC
        if let Some(result) = iter.next() {
            match result {
                Value::Array(results) => {
                    let mut filtered_results = zip(results, self.forget_flags.iter().skip(1))
                        .filter_map(
                            |(value, forget_flag)| if *forget_flag { None } else { Some(value) },
                        )
                        .collect::<Vec<_>>();

                    if filtered_results.len() == 1 {
                        let value = filtered_results.pop().unwrap();
                        Ok(value).into_result()?.into()
                    } else {
                        Value::Array(filtered_results).into()
                    }
                }
                Value::Nil => Err(Error::Aborted),
                _ => Err(Error::Client("Unexpected transaction reply".to_owned())),
            }
        } else {
            Err(Error::Client(
                "Unexpected result for transaction".to_owned(),
            ))
        }
    }
}

impl<'a, R> BatchPreparedCommand<'a, R> for PreparedCommand<'a, Transaction, R>
where
    R: FromValue + Send + 'a,
{
    /// Queue a command into the transaction.
    fn queue(self) {
        self.executor.queue(self.command)
    }

    /// Queue a command into the transaction and forget its response.
    fn forget(self) {
        self.executor.forget(self.command)
    }
}

impl BitmapCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl BloomCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl CountMinSketchCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl CuckooCommands for Transaction {}
impl GenericCommands for Transaction {}
impl GeoCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-graph")))]
#[cfg(feature = "redis-graph")]
impl GraphCommands for Transaction {}
impl HashCommands for Transaction {}
impl HyperLogLogCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-json")))]
#[cfg(feature = "redis-json")]
impl JsonCommands for Transaction {}
impl ListCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-search")))]
#[cfg(feature = "redis-search")]
impl SearchCommands for Transaction {}
impl SetCommands for Transaction {}
impl ScriptingCommands for Transaction {}
impl ServerCommands for Transaction {}
impl SortedSetCommands for Transaction {}
impl StreamCommands for Transaction {}
impl StringCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl TDigestCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-time-series")))]
#[cfg(feature = "redis-time-series")]
impl TimeSeriesCommands for Transaction {}
#[cfg_attr(docsrs, doc(cfg(feature = "redis-bloom")))]
#[cfg(feature = "redis-bloom")]
impl TopKCommands for Transaction {}
