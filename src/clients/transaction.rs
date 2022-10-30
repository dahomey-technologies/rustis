use crate::{
    resp::{cmd, Array, Command, FromValue, ResultValueExt, Value},
    BitmapCommands, Client, Error, Future, GenericCommands, GeoCommands, HashCommands,
    HyperLogLogCommands, ListCommands, PreparedCommand, Result, ScriptingCommands, ServerCommands,
    SetCommands, SortedSetCommands, StreamCommands, StringCommands,
};
use std::{
    future::IntoFuture,
    iter::zip,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

/// Represents an on-going [`transaction`](https://redis.io/docs/manual/transactions/) on a specific client instance.
pub struct Transaction<T> {
    phantom: PhantomData<T>,
    client: Client,
    forget_flags: Arc<Mutex<Vec<bool>>>,
}

impl<T: Send + Sync> Transaction<T> {
    pub(crate) async fn initialize(mut client: Client) -> Result<Self> {
        client.send(cmd("MULTI")).await?.into::<()>()?;
        Ok(Self {
            phantom: PhantomData,
            client,
            forget_flags: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub(crate) fn from_transaction<U: Send + Sync>(transaction: &Transaction<U>) -> Self {
        Self {
            phantom: PhantomData,
            client: transaction.client.clone(),
            forget_flags: transaction.forget_flags.clone(),
        }
    }

    pub(crate) async fn queue(&mut self, command: Command) -> Result<()> {
        self.forget_flags.lock().unwrap().push(false);
        self.client.send(command).await?.into()
    }

    pub(crate) async fn queue_and_forget(&mut self, command: Command) -> Result<()> {
        self.forget_flags.lock().unwrap().push(true);
        self.client.send(command).await?.into()
    }

    pub(crate) fn execute<R: FromValue>(mut self) -> Future<'static, R> {
        Box::pin(async move {
            let result = self.client.send(cmd("EXEC")).await?;

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
                }
                Value::Array(Array::Nil) => Err(Error::Aborted),
                _ => Err(Error::Client("Unexpected transaction reply".to_owned())),
            }
        })
    }

    /// Flushes all previously queued commands in a transaction and restores the connection state to normal.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error)
    pub async fn discard(mut self) -> Result<()> {
        self.client.send(cmd("DISCARD")).await?.into()
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

impl<'a, T1> IntoFuture for PreparedCommand<'a, Transaction<TransactionResult0>, T1>
where
    T1: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult1<T1>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult1<T1>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2> IntoFuture for PreparedCommand<'a, Transaction<TransactionResult1<T1>>, T2>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult2<T1, T2>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult2<T1, T2>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3> IntoFuture for PreparedCommand<'a, Transaction<TransactionResult2<T1, T2>>, T3>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult3<T1, T2, T3>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult3<T1, T2, T3>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4> IntoFuture
    for PreparedCommand<'a, Transaction<TransactionResult3<T1, T2, T3>>, T4>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
{
    type Output = Result<Transaction<TransactionResult4<T1, T2, T3, T4>>>;
    type IntoFuture = Future<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5> IntoFuture
    for PreparedCommand<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>, T5>
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
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6> IntoFuture
    for PreparedCommand<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>, T6>
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
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7> IntoFuture
    for PreparedCommand<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>, T7>
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
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8> IntoFuture
    for PreparedCommand<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>, T8>
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
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9> IntoFuture
    for PreparedCommand<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>, T9>
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
    type IntoFuture =
        Future<'a, Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> IntoFuture
    for PreparedCommand<
        'a,
        Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>,
        T10,
    >
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
    type IntoFuture =
        Future<'a, Transaction<TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.queue(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

#[allow(clippy::module_name_repetitions)]
pub trait TransactionPreparedCommand<'a, T, U> {
    fn forget(self) -> Future<'a, Transaction<U>>;
}

impl<'a, T1> TransactionPreparedCommand<'a, TransactionResult1<T1>, TransactionResult0>
    for PreparedCommand<'a, Transaction<TransactionResult0>, T1>
where
    T1: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult0>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2> TransactionPreparedCommand<'a, TransactionResult2<T1, T2>, TransactionResult1<T1>>
    for PreparedCommand<'a, Transaction<TransactionResult1<T1>>, T2>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult1<T1>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3>
    TransactionPreparedCommand<'a, TransactionResult3<T1, T2, T3>, TransactionResult2<T1, T2>>
    for PreparedCommand<'a, Transaction<TransactionResult2<T1, T2>>, T3>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult2<T1, T2>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4>
    TransactionPreparedCommand<
        'a,
        TransactionResult4<T1, T2, T3, T4>,
        TransactionResult3<T1, T2, T3>,
    > for PreparedCommand<'a, Transaction<TransactionResult3<T1, T2, T3>>, T4>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult3<T1, T2, T3>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5>
    TransactionPreparedCommand<
        'a,
        TransactionResult5<T1, T2, T3, T4, T5>,
        TransactionResult4<T1, T2, T3, T4>,
    > for PreparedCommand<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>, T5>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult4<T1, T2, T3, T4>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6>
    TransactionPreparedCommand<
        'a,
        TransactionResult6<T1, T2, T3, T4, T5, T6>,
        TransactionResult5<T1, T2, T3, T4, T5>,
    > for PreparedCommand<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>, T6>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult5<T1, T2, T3, T4, T5>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7>
    TransactionPreparedCommand<
        'a,
        TransactionResult7<T1, T2, T3, T4, T5, T6, T7>,
        TransactionResult6<T1, T2, T3, T4, T5, T6>,
    > for PreparedCommand<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>, T7>
where
    T1: FromValue + Send + Sync + 'a,
    T2: FromValue + Send + Sync + 'a,
    T3: FromValue + Send + Sync + 'a,
    T4: FromValue + Send + Sync + 'a,
    T5: FromValue + Send + Sync + 'a,
    T6: FromValue + Send + Sync + 'a,
    T7: FromValue + Send + Sync + 'a,
{
    fn forget(self) -> Future<'a, Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8>
    TransactionPreparedCommand<
        'a,
        TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>,
        TransactionResult7<T1, T2, T3, T4, T5, T6, T7>,
    > for PreparedCommand<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>, T8>
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
    fn forget(self) -> Future<'a, Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9>
    TransactionPreparedCommand<
        'a,
        TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
        TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>,
    > for PreparedCommand<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>, T9>
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
    fn forget(self) -> Future<'a, Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>> {
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
    }
}

impl<'a, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
    TransactionPreparedCommand<
        'a,
        TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>,
        TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
    >
    for PreparedCommand<
        'a,
        Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>,
        T10,
    >
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
        Box::pin(async move {
            self.executor.queue_and_forget(self.command).await?;
            Ok(Transaction::from_transaction(self.executor))
        })
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
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2)> {
        self.execute()
    }
}

impl<T1, T2, T3> TransactionExt<(T1, T2, T3)> for Transaction<TransactionResult3<T1, T2, T3>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4> TransactionExt<(T1, T2, T3, T4)>
    for Transaction<TransactionResult4<T1, T2, T3, T4>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5> TransactionExt<(T1, T2, T3, T4, T5)>
    for Transaction<TransactionResult5<T1, T2, T3, T4, T5>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
    T5: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5, T6> TransactionExt<(T1, T2, T3, T4, T5, T6)>
    for Transaction<TransactionResult6<T1, T2, T3, T4, T5, T6>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
    T5: FromValue + Send + Sync,
    T6: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7> TransactionExt<(T1, T2, T3, T4, T5, T6, T7)>
    for Transaction<TransactionResult7<T1, T2, T3, T4, T5, T6, T7>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
    T5: FromValue + Send + Sync,
    T6: FromValue + Send + Sync,
    T7: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8> TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8)>
    for Transaction<TransactionResult8<T1, T2, T3, T4, T5, T6, T7, T8>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
    T5: FromValue + Send + Sync,
    T6: FromValue + Send + Sync,
    T7: FromValue + Send + Sync,
    T8: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7, T8)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8, T9)>
    for Transaction<TransactionResult9<T1, T2, T3, T4, T5, T6, T7, T8, T9>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
    T5: FromValue + Send + Sync,
    T6: FromValue + Send + Sync,
    T7: FromValue + Send + Sync,
    T8: FromValue + Send + Sync,
    T9: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7, T8, T9)> {
        self.execute()
    }
}

impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
    TransactionExt<(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
    for Transaction<TransactionResult10<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>>
where
    T1: FromValue + Send + Sync,
    T2: FromValue + Send + Sync,
    T3: FromValue + Send + Sync,
    T4: FromValue + Send + Sync,
    T5: FromValue + Send + Sync,
    T6: FromValue + Send + Sync,
    T7: FromValue + Send + Sync,
    T8: FromValue + Send + Sync,
    T9: FromValue + Send + Sync,
    T10: FromValue + Send + Sync,
{
    fn exec(self) -> Future<'static, (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)> {
        self.execute()
    }
}

impl<T> BitmapCommands for Transaction<T> {}
impl<T> GenericCommands for Transaction<T> {}
impl<T> GeoCommands for Transaction<T> {}
impl<T> HashCommands for Transaction<T> {}
impl<T> HyperLogLogCommands for Transaction<T> {}
impl<T> ListCommands for Transaction<T> {}
impl<T> SetCommands for Transaction<T> {}
impl<T> ScriptingCommands for Transaction<T> {}
impl<T> SortedSetCommands for Transaction<T> {}
impl<T> ServerCommands for Transaction<T> {}
impl<T> StreamCommands for Transaction<T> {}
impl<T> StringCommands for Transaction<T> {}
