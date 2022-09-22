use crate::{
    cmd,
    resp::{Array, FromValue, ResultValueExt, Value},
    BitmapCommands, Command, CommandResult, Database, Error, Future, GenericCommands, GeoCommands,
    HashCommands, HyperLogLogCommands, ListCommands, PrepareCommand, Result, ScriptingCommands,
    ServerCommands, SetCommands, SortedSetCommands, StreamCommands, StringCommands,
};
use std::{
    iter::zip,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

pub struct Transaction<T> {
    phantom: PhantomData<T>,
    database: Database,
    forget_flags: Arc<Mutex<Vec<bool>>>,
}

impl<T: Send + Sync> Transaction<T> {
    pub(crate) async fn initialize(database: Database) -> Result<Self> {
        database.send(cmd("MULTI")).await?.into::<()>()?;
        Ok(Self {
            phantom: PhantomData,
            database,
            forget_flags: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub(crate) fn from_transaction<U: Send + Sync>(transaction: &Transaction<U>) -> Self {
        Self {
            phantom: PhantomData,
            database: transaction.database.clone(),
            forget_flags: transaction.forget_flags.clone(),
        }
    }

    pub(crate) async fn internal_queue(&self, command: Command) -> Result<()> {
        self.forget_flags.lock().unwrap().push(false);
        self.database.send(command).await?.into()
    }

    pub(crate) async fn internal_queue_and_forget(&self, command: Command) -> Result<()> {
        self.forget_flags.lock().unwrap().push(true);
        self.database.send(command).await?.into()
    }

    pub(crate) fn internal_exec<R: FromValue>(&self) -> Future<'_, R> {
        Box::pin(async move {
            let result = self.database.send(cmd("EXEC")).await?;

            match result {
                Value::Array(Array::Vec(results)) => {
                    let forget_flags = self.forget_flags.lock().unwrap();
                    let forget_flags = &*forget_flags;
                    let mut filtered_results = zip(results, forget_flags.iter())
                        .filter_map(
                            |(value, forget_flag)| if *forget_flag { None } else { Some(value) },
                        )
                        .collect::<Vec<_>>();

                    if filtered_results.len() == 1 {
                        let value = filtered_results.pop().unwrap();
                        Ok(value).into_result()?.into()
                    } else {
                        Value::Array(Array::Vec(filtered_results)).into()
                    }
                },
                Value::Array(Array::Nil) => Err(Error::Aborted),
                _ => Err(Error::Internal("Unexpected transaction reply".to_owned())),
            }
        })
    }

    /// Flushes all previously queued commands in a transaction and restores the connection state to normal.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error)
    pub async fn discard(self) -> Result<()> {
        self.database.send(cmd("DISCARD")).await?.into()
    }
}

impl<T: Send + Sync> PrepareCommand<T> for Transaction<T> {
    fn prepare_command<R: FromValue>(&self, command: Command) -> CommandResult<T, R> {
        CommandResult::from_transaction(command, self)
    }
}

impl<T: Send + Sync> BitmapCommands<T> for Transaction<T> {}
impl<T: Send + Sync> GenericCommands<T> for Transaction<T> {}
impl<T: Send + Sync> GeoCommands<T> for Transaction<T> {}
impl<T: Send + Sync> HashCommands<T> for Transaction<T> {}
impl<T: Send + Sync> HyperLogLogCommands<T> for Transaction<T> {}
impl<T: Send + Sync> ListCommands<T> for Transaction<T> {}
impl<T: Send + Sync> SetCommands<T> for Transaction<T> {}
impl<T: Send + Sync> ScriptingCommands<T> for Transaction<T> {}
impl<T: Send + Sync> SortedSetCommands<T> for Transaction<T> {}
impl<T: Send + Sync> ServerCommands<T> for Transaction<T> {}
impl<T: Send + Sync> StreamCommands<T> for Transaction<T> {}
impl<T: Send + Sync> StringCommands<T> for Transaction<T> {}
