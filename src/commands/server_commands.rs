use crate::{
    resp::{
        cmd, BulkString, CommandArgs, FromValue, IntoArgs, KeyValueArgOrCollection,
        SingleArgOrCollection, FromKeyValueValueArray,
    },
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to Server Management
/// # See Also
/// [Redis Server Management Commands](https://redis.io/commands/?group=server)
pub trait ServerCommands<T>: PrepareCommand<T> {
    /// Used to read the configuration parameters of a running Redis server.
    ///
    /// For every key that does not hold a string value or does not exist,
    /// the special value nil is returned. Because of this, the operation never fails.
    ///
    /// # Return
    /// Array reply: collection of the requested params with their matching values.
    ///
    /// # See Also
    /// [https://redis.io/commands/mget/](https://redis.io/commands/mget/)
    #[must_use]
    fn config_get<P, PP, V, VV>(&self, params: PP) -> CommandResult<T, VV>
    where
        P: Into<BulkString>,
        PP: SingleArgOrCollection<P>,
        V: FromValue,
        VV: FromKeyValueValueArray<String, V>
    {
        self.prepare_command(cmd("CONFIG").arg("GET").arg(params))
    }

    /// Used in order to reconfigure the server at run time without the need to restart Redis.
    ///
    /// # See Also
    /// [https://redis.io/commands/config-set/](https://redis.io/commands/config-set/)
    #[must_use]
    fn config_set<P, V, C>(&self, configs: C) -> CommandResult<T, ()>
    where
        P: Into<BulkString>,
        V: Into<BulkString>,
        C: KeyValueArgOrCollection<P, V>,
    {
        self.prepare_command(cmd("CONFIG").arg("SET").arg(configs))
    }

    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushdb/](https://redis.io/commands/flushdb/)
    #[must_use]
    fn flushdb(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FLUSHDB").arg(flushing_mode))
    }

    /// Delete all the keys of all the existing databases, not just the currently selected one.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushall/](https://redis.io/commands/flushall/)
    #[must_use]
    fn flushall(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FLUSHALL").arg(flushing_mode))
    }

    /// The TIME command returns the current server time as a two items lists:
    /// a Unix timestamp and the amount of microseconds already elapsed in the current second.
    ///
    /// # See Also
    /// [https://redis.io/commands/time/](https://redis.io/commands/time/)
    #[must_use]
    fn time(&self) -> CommandResult<T, (u32, u32)> {
        self.prepare_command(cmd("TIME"))
    }
}

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
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            FlushingMode::Default => args,
            FlushingMode::Async => args.arg("ASYNC"),
            FlushingMode::Sync => args.arg("SYNC"),
        }
    }
}
