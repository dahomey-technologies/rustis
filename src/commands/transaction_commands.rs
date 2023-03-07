use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{cmd, SingleArg, SingleArgCollection},
};

/// A group of Redis commands related to Transactions
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=transactions)
pub trait TransactionCommands<'a> {
    /// Marks the given keys to be watched for conditional execution of a transaction.
    ///
    /// # See Also
    /// [<https://redis.io/commands/watch/>](https://redis.io/commands/watch/)
    #[must_use]
    fn watch<K, KK>(self, keys: KK) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        K: SingleArg,
        KK: SingleArgCollection<K>,
    {
        prepare_command(self, cmd("WATCH").arg(keys))
    }

    /// Flushes all the previously watched keys for a transaction.
    ///
    /// If you call [`execute`](crate::client::Transaction::execute),
    /// there's no need to manually call UNWATCH.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unwatch/>](https://redis.io/commands/unwatch/)
    #[must_use]
    fn unwatch(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("UNWATCH"))
    }
}
