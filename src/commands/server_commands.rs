use crate::{cmd, CommandSend, Future, IntoArgs};

/// Database flushing mode
pub enum FlushingMode {
    Default,
    /// Flushes the database(s) asynchronously
    Async,
    /// Flushed the database(s) synchronously
    Sync,
}

impl Default for FlushingMode {
    fn default() -> Self {
        FlushingMode::Default
    }
}

impl IntoArgs for FlushingMode {
    fn into_args(self, args: crate::CommandArgs) -> crate::CommandArgs {
        match self {
            FlushingMode::Default => args,
            FlushingMode::Async => args.arg("ASYNC"),
            FlushingMode::Sync => args.arg("SYNC"),
        }
    }
}

/// A group of Redis commands related to Server Management
/// # See Also
/// [Redis Server Management Commands](https://redis.io/commands/?group=server)
pub trait ServerCommands: CommandSend {
    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushdb/](https://redis.io/commands/flushdb/)
    fn flushdb(&self, flushing_mode: FlushingMode) -> Future<'_, ()> {
        self.send_into(cmd("FLUSHDB").arg(flushing_mode))
    }

    /// Delete all the keys of all the existing databases, not just the currently selected one.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushall/](https://redis.io/commands/flushall/)
    fn flushall(&self, flushing_mode: FlushingMode) -> Future<'_, ()> {
        self.send_into(cmd("FLUSHALL").arg(flushing_mode))
    }
}
