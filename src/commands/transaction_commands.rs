use crate::{
    resp::{cmd, BulkString, SingleArgOrCollection},
    prepare_command, PreparedCommand,
};

/// A group of Redis commands related to Transactions
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=transactions)
pub trait TransactionCommands {
    /// Marks the given keys to be watched for conditional execution of a transaction.
    ///
    /// # See Also
    /// [<https://redis.io/commands/watch/>](https://redis.io/commands/watch/)
    #[must_use]
    fn watch<K, KK>(&mut self, keys: KK) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
    {
        prepare_command(self, cmd("WATCH").arg(keys))
    }

    /// Flushes all the previously watched keys for a transaction.
    ///
    /// If you call [`execute`](crate::Transaction::execute),
    /// there's no need to manually call UNWATCH.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unwatch/>](https://redis.io/commands/unwatch/)
    #[must_use]
    fn unwatch(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("UNWATCH"))
    }
}
