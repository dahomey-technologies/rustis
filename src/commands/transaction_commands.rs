use crate::{
    client::{PreparedCommand, prepare_command},
    resp::cmd,
};
use serde::Serialize;

/// A group of Redis commands related to Transactions
///
/// # ⚠ These commands require a dedicated connection
///
/// The watched state belongs to the connection, not to the handle that asked
/// for it: on a shared connection any other caller can add keys to it, discard
/// it with [`unwatch`](TransactionCommands::unwatch), or consume it by running
/// its own `EXEC`. The trait is therefore implemented for
/// [`ExclusiveClient`](crate::client::ExclusiveClient) alone, and not for the
/// clonable [`Client`](crate::client::Client).
///
/// A transaction itself needs nothing of the sort: `MULTI`/`EXEC` are queued
/// and sent as one block, so
/// [`Client::create_transaction`](crate::client::Client::create_transaction)
/// stays available on a multiplexed client. Only the optimistic-locking half —
/// `WATCH` — is restricted.
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=transactions)
pub trait TransactionCommands<'a>: Sized {
    /// Marks the given keys to be watched for conditional execution of a transaction.
    ///
    /// **Holds state on its connection.** Call it on a client that owns one —
    /// see [the trait documentation](TransactionCommands).
    ///
    /// # See Also
    /// [<https://redis.io/commands/watch/>](https://redis.io/commands/watch/)
    #[must_use]
    fn watch(self, keys: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("WATCH").keys(keys))
    }

    /// Flushes all the previously watched keys for a transaction.
    ///
    /// If you call [`execute`](crate::client::Transaction::execute),
    /// there's no need to manually call UNWATCH.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unwatch/>](https://redis.io/commands/unwatch/)
    #[must_use]
    fn unwatch(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("UNWATCH"))
    }
}
