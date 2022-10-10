use crate::{
    resp::{Command, FromValue},
    Client, Error, Future, Result, Transaction,
};
use futures::future::ready;
use std::{future::IntoFuture, marker::PhantomData};

pub enum CommandResult<'a, T, R>
where
    R: FromValue,
{
    Client(PhantomData<(R, T)>, Command, &'a Client),
    Transaction(PhantomData<R>, Command, &'a Transaction<T>),
}

impl<'a, T, R> CommandResult<'a, T, R>
where
    R: FromValue,
    T: Send + Sync,
{
    #[must_use]
    pub fn from_client(command: Command, client: &'a Client) -> Self {
        CommandResult::Client(PhantomData, command, client)
    }

    #[must_use]
    pub fn from_transaction(command: Command, transaction: &'a Transaction<T>) -> Self {
        CommandResult::Transaction(PhantomData, command, transaction)
    }

    pub(crate) async fn queue<U: Send + Sync>(self) -> Result<Transaction<U>> {
        if let CommandResult::Transaction(_, command, transaction) = self {
            transaction.queue(command).await?;
            Ok(Transaction::from_transaction(transaction))
        } else {
            Err(Error::Client(
                "queue method must be called with a valid transaction".to_owned(),
            ))
        }
    }

    pub(crate) async fn queue_and_forget(self) -> Result<Transaction<T>> {
        if let CommandResult::Transaction(_, command, transaction) = self {
            transaction.queue_and_forget(command).await?;
            Ok(Transaction::from_transaction(transaction))
        } else {
            Err(Error::Client(
                "queue method must be called with a valid transaction".to_owned(),
            ))
        }
    }
}

pub trait PrepareCommand<T> {
    fn prepare_command<R: FromValue>(&self, command: Command) -> CommandResult<T, R>;
}

pub struct ClientResult;

#[allow(clippy::module_name_repetitions)]
pub trait ClientCommandResult<'a, R>
where
    R: FromValue,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R> ClientCommandResult<'a, R> for CommandResult<'a, ClientResult, R>
where
    R: FromValue + Send + 'a,
{
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()> {
        if let CommandResult::Client(_, command, client) = self {
            client.send_and_forget(command)
        } else {
            Err(Error::Client(
                "send_and_forget method must be called with a valid client".to_owned(),
            ))
        }
    }
}

impl<'a, R> IntoFuture for CommandResult<'a, ClientResult, R>
where
    R: FromValue + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        if let CommandResult::Client(_, command, client) = self {
            let fut = client.send(command);
            Box::pin(async move { fut.await?.into() })
        } else {
            Box::pin(ready(Err(Error::Client(
                "send method must be called with a valid client".to_owned(),
            ))))
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
#[allow(clippy::complexity)]
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
#[allow(clippy::complexity)]
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
#[allow(clippy::complexity)]
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

impl<'a, T1> IntoFuture for CommandResult<'a, TransactionResult0, T1>
where
    T1: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult1<T1>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult1<T1>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2> IntoFuture for CommandResult<'a, TransactionResult1<T1>, T2>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult2<T1, T2>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult2<T1, T2>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3> IntoFuture for CommandResult<'a, TransactionResult2<T1, T2>, T3>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult3<T1, T2, T3>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult3<T1, T2, T3>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4> IntoFuture for CommandResult<'a, TransactionResult3<T1, T2, T3>, T4>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult4<T1, T2, T3, T4>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4, T5> IntoFuture for CommandResult<'a, TransactionResult4<T1, T2, T3, T4>, T5>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult5<T1, T2, T3, T4, T5>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6> IntoFuture for CommandResult<'a, TransactionResult5<T1, T2, T3, T4, T5>, T6>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7> IntoFuture for CommandResult<'a, TransactionResult6<T1, T2, T3, T4, T5, T6>, T7>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
    T7: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8> IntoFuture for CommandResult<'a, TransactionResult7<T1, T2, T3, T4, T5, T6, T7>, T8>
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
    type Output = Result<Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9> IntoFuture for CommandResult<'a, TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>, T9>
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
    type Output = Result<Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> IntoFuture for CommandResult<'a, TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>, T10>
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
    type Output = Result<Transaction<TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.queue())
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait TransactionCommandResult<'a, T, U> {
    fn forget(self) -> Future<'a, Transaction<U>>;
}

impl<'a, T1> TransactionCommandResult<'a, TransactionResult1<T1>, TransactionResult0>
    for CommandResult<'a, TransactionResult0, T1>
where
    T1: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult0>> {
        Box::pin(self.queue_and_forget())
    }
}

impl<'a, T1, T2> TransactionCommandResult<'a, TransactionResult2<T1, T2>, TransactionResult1<T1>>
    for CommandResult<'a, TransactionResult1<T1>, T2>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult1<T1>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(self) -> Future<'a, Transaction<TransactionResult2<T1, T2>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(self) -> Future<'a, Transaction<TransactionResult3<T1, T2, T3>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(self) -> Future<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(self) -> Future<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>> {
        Box::pin(self.queue_and_forget())
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
    fn forget(
        self,
    ) -> Future<'a, Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>> {
        Box::pin(self.queue_and_forget())
    }
}

pub trait TransactionExt<T> {
    fn exec(self) -> Future<'static, T>;
}

impl<T: FromValue + Send + Sync> TransactionExt<T> for Transaction<TransactionResult1<T>> {
    fn exec(self) -> Future<'static, T> {
        self.execute()
    }
}

impl<T1, T2> TransactionExt<(T1, T2)> for Transaction<TransactionResult2<T1, T2>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2)> {
        self.execute()
    }
}

impl<T1, T2, T3> TransactionExt<(T1, T2, T3)> for Transaction<TransactionResult3<T1, T2, T3>>
where
    T1: FromValue + Default + Send + Sync,
    T2: FromValue + Default + Send + Sync,
    T3: FromValue + Default + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3)> {
        self.execute()
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4)> {
        self.execute()
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5)> {
        self.execute()
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6)> {
        self.execute()
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7)> {
        self.execute()
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7, T8)> {
        self.execute()
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7, T8, T9)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
    TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
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
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)> {
        self.execute()
    }
}
