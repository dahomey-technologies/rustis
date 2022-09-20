use crate::{resp::FromValue, Command, Database, Error, Future, Result, Transaction};
use futures::future::ready;
use std::marker::PhantomData;

pub enum CommandResult<'a, T, R>
where
    R: FromValue,
{
    Database(PhantomData<(R, T)>, Command, &'a Database),
    Transaction(PhantomData<R>, Command, &'a Transaction<T>),
}

impl<'a, T, R> CommandResult<'a, T, R>
where
    R: FromValue,
    T: Send + Sync,
{
    pub fn from_database(command: Command, database: &'a Database) -> Self {
        CommandResult::Database(PhantomData, command, database)
    }

    pub fn from_transaction(command: Command, transaction: &'a Transaction<T>) -> Self {
        CommandResult::Transaction(PhantomData, command, transaction)
    }

    pub(crate) async fn internal_queue<U: Send + Sync>(self) -> Result<Transaction<U>> {
        match self {
            CommandResult::Transaction(_, command, transaction) => {
                transaction.internal_queue(command).await?;
                Ok(Transaction::from_transaction(transaction))
            }
            _ => Err(Error::Internal(
                "queue method must be called with a valid transaction".to_owned(),
            )),
        }
    }

    pub(crate) async fn internal_queue_and_forget(self) -> Result<Transaction<T>> {
        match self {
            CommandResult::Transaction(_, command, transaction) => {
                transaction.internal_queue_and_forget(command).await?;
                Ok(Transaction::from_transaction(transaction))
            }
            _ => Err(Error::Internal(
                "queue method must be called with a valid transaction".to_owned(),
            )),
        }
    }
}

pub trait IntoCommandResult<T> {
    fn into_command_result<R: FromValue>(&self, command: Command) -> CommandResult<T, R>;
}

pub struct DatabaseResult;

pub trait DatabaseCommandResult<'a, R>
where
    R: FromValue,
{
    fn send(self) -> Future<'a, R>;
    fn send_and_forget(self) -> Future<'a, ()>;
}

impl<'a, R> DatabaseCommandResult<'a, R> for CommandResult<'a, DatabaseResult, R>
where
    R: FromValue + Send + 'a,
{
    fn send(self) -> Future<'a, R> {
        match self {
            CommandResult::Database(_, command, database) => {
                let fut = database.send(command);
                Box::pin(async move { fut.await?.into() })
            }
            _ => Box::pin(ready(Err(Error::Internal(
                "send method must be called with a valid database".to_owned(),
            )))),
        }
    }

    fn send_and_forget(self) -> Future<'a, ()> {
        match self {
            CommandResult::Database(_, command, database) => {
                Box::pin(database.send_and_forget(command))
            }
            _ => Box::pin(ready(Err(Error::Internal(
                "send_and_forget method must be called with a valid database".to_owned(),
            )))),
        }
    }
}

#[derive(Debug)]
pub struct TransactionResult0 {}

#[derive(Debug)]
pub struct TransactionResult1<T1>
where
    T1: FromValue,
{
    phantom: PhantomData<T1>,
}

#[derive(Debug)]
pub struct TransactionResult2<T1, T2>
where
    T1: FromValue,
    T2: FromValue,
{
    phantom: PhantomData<(T1, T2)>,
}

#[derive(Debug)]
pub struct TransactionResult3<T1, T2, T3>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
{
    phantom: PhantomData<(T1, T2, T3)>,
}

#[derive(Debug)]
pub struct TransactionResult4<T1, T2, T3, T4>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4)>,
}

#[derive(Debug)]
pub struct TransactionResult5<T1, T2, T3, T4, T5>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
    T5: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4, T5)>,
}

#[derive(Debug)]
pub struct TransactionResult6<T1, T2, T3, T4, T5, T6>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
    T5: FromValue,
    T6: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4, T5, T6)>,
}

#[derive(Debug)]
pub struct TransactionResult7<T1, T2, T3, T4, T5, T6, T7>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
    T5: FromValue,
    T6: FromValue,
    T7: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4, T5, T6, T7)>,
}

#[derive(Debug)]
pub struct TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
    T5: FromValue,
    T6: FromValue,
    T7: FromValue,
    T8: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4, T5, T6, T7, T8)>,
}

#[derive(Debug)]
pub struct TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
    T5: FromValue,
    T6: FromValue,
    T7: FromValue,
    T8: FromValue,
    T9: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>,
}

#[derive(Debug)]
pub struct TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
where
    T1: FromValue,
    T2: FromValue,
    T3: FromValue,
    T4: FromValue,
    T5: FromValue,
    T6: FromValue,
    T7: FromValue,
    T8: FromValue,
    T9: FromValue,
    T10: FromValue,
{
    phantom: PhantomData<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>,
}

pub trait TransactionCommandResult<'a, T, U> {
    fn queue(self) -> Future<'a, Transaction<T>>;
    fn queue_and_forget(self) -> Future<'a, Transaction<U>>;
}

impl<'a, T> TransactionCommandResult<'a, TransactionResult1<T>, TransactionResult0>
    for CommandResult<'a, TransactionResult0, T>
where
    T: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult1<T>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(self) -> Future<'a, Transaction<TransactionResult0>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2> TransactionCommandResult<'a, TransactionResult2<T1, T2>, TransactionResult1<T1>>
    for CommandResult<'a, TransactionResult1<T1>, T2>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult2<T1, T2>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(self) -> Future<'a, Transaction<TransactionResult1<T1>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3>
    TransactionCommandResult<'a, TransactionResult3<T1, T2, T3>, TransactionResult2<T1, T2>>
    for CommandResult<'a, TransactionResult2<T1, T2>, T3>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult3<T1, T2, T3>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(self) -> Future<'a, Transaction<TransactionResult2<T1, T2>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4>
    TransactionCommandResult<'a, TransactionResult4<T1, T2, T3, T4>, TransactionResult3<T1, T2, T3>>
    for CommandResult<'a, TransactionResult3<T1, T2, T3>, T4>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(self) -> Future<'a, Transaction<TransactionResult3<T1, T2, T3>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4, T5>
    TransactionCommandResult<
        'a,
        TransactionResult5<T1, T2, T3, T4, T5>,
        TransactionResult4<T1, T2, T3, T4>,
    > for CommandResult<'a, TransactionResult4<T1, T2, T3, T4>, T5>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(self) -> Future<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6>
    TransactionCommandResult<
        'a,
        TransactionResult6<T1, T2, T3, T4, T5, T6>,
        TransactionResult5<T1, T2, T3, T4, T5>,
    > for CommandResult<'a, TransactionResult5<T1, T2, T3, T4, T5>, T6>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(self) -> Future<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7>
    TransactionCommandResult<
        'a,
        TransactionResult7<T1, T2, T3, T4, T5, T6, T7>,
        TransactionResult6<T1, T2, T3, T4, T5, T6>,
    > for CommandResult<'a, TransactionResult6<T1, T2, T3, T4, T5, T6>, T7>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
    T7: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8>
    TransactionCommandResult<
        'a,
        TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>,
        TransactionResult7<T1, T2, T3, T4, T5, T6, T7>,
    > for CommandResult<'a, TransactionResult7<T1, T2, T3, T4, T5, T6, T7>, T8>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
    T7: FromValue + Send + Sync + 'a,
    T8: FromValue + Send + Sync + 'a,
{
    fn queue(self) -> Future<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9>
    TransactionCommandResult<
        'a,
        TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
        TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>,
    > for CommandResult<'a, TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>, T9>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
    T7: FromValue + Send + Sync + 'a,
    T8: FromValue + Send + Sync + 'a,
    T9: FromValue + Send + Sync + 'a,
{
    fn queue(
        self,
    ) -> Future<'a, Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
    TransactionCommandResult<
        'a,
        TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>,
        TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
    > for CommandResult<'a, TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>, T10>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
    T7: FromValue + Send + Sync + 'a,
    T8: FromValue + Send + Sync + 'a,
    T9: FromValue + Send + Sync + 'a,
    T10: FromValue + Send + Sync + 'a,
{
    fn queue(
        self,
    ) -> Future<'a, Transaction<TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>>> {
        Box::pin(self.internal_queue())
    }

    fn queue_and_forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>> {
        Box::pin(self.internal_queue_and_forget())
    }
}

pub trait TransactionExt<T> {
    fn exec(&self) -> Future<'_, T>;
}

impl<T: FromValue + Send + Sync> TransactionExt<T> for Transaction<TransactionResult1<T>> {
    fn exec(&self) -> Future<'_, T> {
        self.internal_exec()
    }
}

impl<T1, T2> TransactionExt<(T1, T2)> for Transaction<TransactionResult2<T1, T2>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3> TransactionExt<(T1, T2, T3)> for Transaction<TransactionResult3<T1, T2, T3>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4> TransactionExt<(T1, T2, T3, T4)>
    for Transaction<TransactionResult4<T1, T2, T3, T4>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4, T5> TransactionExt<(T1, T2, T3, T4, T5)>
    for Transaction<TransactionResult5<T1, T2, T3, T4, T5>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
    T5: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4, T5)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4, T5, T6> TransactionExt<(T1, T2, T3, T4, T5, T6)>
    for Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
    T5: FromValue + Default + Send + Sync,
    T6: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4, T5, T6)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7> TransactionExt<(T1, T2, T3, T4, T5, T6, T7)>
    for Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
    T5: FromValue + Default + Send + Sync,
    T6: FromValue + Default + Send + Sync,
    T7: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4, T5, T6, T7)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8> TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
    T5: FromValue + Default + Send + Sync,
    T6: FromValue + Default + Send + Sync,
    T7: FromValue + Default + Send + Sync,
    T8: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4, T5, T6, T7, T8)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
    T5: FromValue + Default + Send + Sync,
    T6: FromValue + Default + Send + Sync,
    T7: FromValue + Default + Send + Sync,
    T8: FromValue + Default + Send + Sync,
    T9: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4, T5, T6, T7, T8, T9)> {
        self.internal_exec()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for Transaction<TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
    T4: FromValue + Default + Send + Sync,
    T5: FromValue + Default + Send + Sync,
    T6: FromValue + Default + Send + Sync,
    T7: FromValue + Default + Send + Sync,
    T8: FromValue + Default + Send + Sync,
    T9: FromValue + Default + Send + Sync,
    T10: FromValue + Default + Send + Sync,
{
    fn exec(&self) -> Future<'_, (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)> {
        self.internal_exec()
    }
}

