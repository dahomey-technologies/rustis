use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to Transactions
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=transactions)
pub trait TransactionCommands<T>: PrepareCommand<T> {
    /// Marks the given keys to be watched for conditional execution of a transaction.
    ///
    /// # See Also
    /// [https://redis.io/commands/watch/](https://redis.io/commands/watch/)
    #[must_use]
    fn watch<K, KK>(&self, keys: KK) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
    {
        self.prepare_command(cmd("WATCH").arg(keys))
    }

    /// Flushes all the previously watched keys for a transaction.
    ///
    /// If you call [`exec`](crate::TransactionExt::exec) or [`discard`](crate::Transaction::discard),
    /// there's no need to manually call UNWATCH.
    ///
    /// # See Also
    /// [https://redis.io/commands/unwatch/](https://redis.io/commands/unwatch/)
    #[must_use]
    fn unwatch(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("UNWATCH"))
    }
}
