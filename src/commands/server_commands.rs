use crate::{cmd, CommandSend, Future};

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

/// A group of Redis commands related to Server Management
/// # See Also
/// [Redis Server Management Commands](https://redis.io/commands/?group=server)
pub trait ServerCommands: CommandSend {
    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushdb/](https://redis.io/commands/flushdb/)
    fn flushdb(&self, flushing_mode: FlushingMode) -> Future<'_, ()> {
        let mut command = cmd("FLUSHDB");
        match flushing_mode {
            FlushingMode::Default => (),
            FlushingMode::Async => command = command.arg("ASYNC"),
            FlushingMode::Sync => command = command.arg("SYNC"),
        }
        self.send_into(command)
    }

    /// Delete all the keys of all the existing databases, not just the currently selected one.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushall/](https://redis.io/commands/flushall/)
    fn flushall(&self, flushing_mode: FlushingMode) -> Future<'_, ()> {
        let mut command = cmd("FLUSHALL");
        match flushing_mode {
            FlushingMode::Default => (),
            FlushingMode::Async => command = command.arg("ASYNC"),
            FlushingMode::Sync => command = command.arg("SYNC"),
        }
        self.send_into(command)
    }
}
