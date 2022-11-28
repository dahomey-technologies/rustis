use crate::{
    prepare_command,
    resp::{
        cmd, CommandArg, CommandArgs, FromSingleValueArray, FromValue, IntoArgs,
        SingleArgOrCollection, Value,
    },
    Error, PreparedCommand, Result,
};

/// A group of generic Redis commands
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=generic)
pub trait GenericCommands {
    /// This command copies the value stored at the source key to the destination key.
    ///
    /// # Return
    /// Success of the operation
    ///
    /// # See Also
    /// [<https://redis.io/commands/copy/>](https://redis.io/commands/copy/)
    #[must_use]
    fn copy<S, D>(
        &mut self,
        source: S,
        destination: D,
        destination_db: Option<usize>,
        replace: bool,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        S: Into<CommandArg>,
        D: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("COPY")
                .arg(source)
                .arg(destination)
                .arg(destination_db.map(|db| ("DB", db)))
                .arg_if(replace, "REPLACE"),
        )
    }

    /// Removes the specified keys. A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/del/>](https://redis.io/commands/del/)
    #[must_use]
    fn del<K, C>(&mut self, keys: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        C: SingleArgOrCollection<K>,
    {
        prepare_command(self, cmd("DEL").arg(keys))
    }

    /// Serialize the value stored at key in a Redis-specific format and return it to the user.
    ///
    /// # Return
    /// The serialized value.
    ///
    /// # See Also
    /// [<https://redis.io/commands/dump/>](https://redis.io/commands/dump/)
    #[must_use]
    fn dump<K>(&mut self, key: K) -> PreparedCommand<Self, DumpResult>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("DUMP").arg(key))
    }

    /// Returns if keys exist.
    ///
    /// # Return
    /// The number of keys that exist from those specified as arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/exists/>](https://redis.io/commands/exists/)
    #[must_use]
    fn exists<K, C>(&mut self, keys: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        C: SingleArgOrCollection<K>,
    {
        prepare_command(self, cmd("EXISTS").arg(keys))
    }

    /// Set a timeout on key in seconds
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/expire/>](https://redis.io/commands/expire/)
    #[must_use]
    fn expire<K>(
        &mut self,
        key: K,
        seconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("EXPIRE").arg(key).arg(seconds).arg(option))
    }

    /// EXPIREAT has the same effect and semantic as EXPIRE,
    /// but instead of specifying the number of seconds representing the TTL (time to live),
    /// it takes an absolute Unix timestamp (seconds since January 1, 1970)
    ///
    /// A timestamp in the past will delete the key
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/expireat/>](https://redis.io/commands/expireat/)
    #[must_use]
    fn expireat<K>(
        &mut self,
        key: K,
        unix_time_seconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("EXPIREAT").arg(key).arg(unix_time_seconds).arg(option),
        )
    }

    /// Returns the absolute Unix timestamp (since January 1, 1970) in seconds at which the given key will expire.
    ///
    /// # Return
    /// Expiration Unix timestamp in seconds, or a negative value in order to signal an error (see the description below).
    /// - The command returns -1 if the key exists but has no associated expiration time.
    /// - The command returns -2 if the key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/expiretime/>](https://redis.io/commands/expiretime/)
    #[must_use]
    fn expiretime<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("EXPIRETIME").arg(key))
    }

    /// Returns all keys matching pattern.
    ///
    /// # Return
    /// list of keys matching pattern.
    ///
    /// # See Also
    /// [<https://redis.io/commands/keys/>](https://redis.io/commands/keys/)
    #[must_use]
    fn keys<P, K, A>(&mut self, pattern: P) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        P: Into<CommandArg>,
        K: FromValue,
        A: FromSingleValueArray<K>,
    {
        prepare_command(self, cmd("KEYS").arg(pattern))
    }

    /// Atomically transfer a key or a collection of keys from a source Redis instance to a destination Redis instance.
    ///
    /// # Return
    /// * `true` - on success
    /// * `false` - if no keys were found in the source instance.
    ///
    /// # See Also
    /// [<https://redis.io/commands/migrate/>](https://redis.io/commands/migrate/)
    #[must_use]
    fn migrate<H, K>(
        &mut self,
        host: H,
        port: u16,
        key: K,
        destination_db: usize,
        timeout: u64,
        options: MigrateOptions,
    ) -> PreparedCommand<Self, MigrateResult>
    where
        Self: Sized,
        H: Into<CommandArg>,
        K: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("MIGRATE")
                .arg(host)
                .arg(port)
                .arg(key)
                .arg(destination_db)
                .arg(timeout)
                .arg(options),
        )
    }

    /// Move key from the currently selected database to the specified destination database.
    ///
    /// # Return
    /// * `true` - if key was moved.
    /// * `false` - f key was not moved.
    ///
    /// # See Also
    /// [<https://redis.io/commands/move/>](https://redis.io/commands/move/)
    #[must_use]
    fn move_<K>(&mut self, key: K, db: usize) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("MOVE").arg(key).arg(db))
    }

    /// Returns the internal encoding for the Redis object stored at `key`
    ///
    /// # Return
    /// The encoding of the object, or nil if the key doesn't exist
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-encoding/>](https://redis.io/commands/object-encoding/)
    #[must_use]
    fn object_encoding<K, E>(&mut self, key: K) -> PreparedCommand<Self, E>
    where
        Self: Sized,
        K: Into<CommandArg>,
        E: FromValue,
    {
        prepare_command(self, cmd("OBJECT").arg("ENCODING").arg(key))
    }

    /// This command returns the logarithmic access frequency counter of a Redis object stored at `key`.
    ///
    /// # Return
    /// The counter's value.
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-freq/>](https://redis.io/commands/object-freq/)
    #[must_use]
    fn object_freq<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("OBJECT").arg("FREQ").arg(key))
    }

    /// This command returns the time in seconds since the last access to the value stored at `key`.
    ///
    /// # Return
    /// The idle time in seconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-idletime/>](https://redis.io/commands/object-idletime/)
    #[must_use]
    fn object_idle_time<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("OBJECT").arg("IDLETIME").arg(key))
    }

    /// This command returns the reference count of the stored at `key`.
    ///
    /// # Return
    /// The number of references.
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-refcount/>](https://redis.io/commands/object-refcount/)
    #[must_use]
    fn object_refcount<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("OBJECT").arg("REFCOUNT").arg(key))
    }

    /// Remove the existing timeout on key,
    /// turning the key from volatile (a key with an expire set)
    /// to persistent (a key that will never expire as no timeout is associated).
    ///
    /// # Return
    /// * `true` - if the timeout was removed.
    /// * `false` - if key does not exist or does not have an associated timeout.
    ///
    /// # See Also
    /// [<https://redis.io/commands/persist/>](https://redis.io/commands/persist/)
    #[must_use]
    fn persist<K>(&mut self, key: K) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("PERSIST").arg(key))
    }

    /// This command works exactly like EXPIRE but the time to live of the key is specified in milliseconds instead of seconds.
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pexpire/>](https://redis.io/commands/pexpire/)
    #[must_use]
    fn pexpire<K>(
        &mut self,
        key: K,
        milliseconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("PEXPIRE").arg(key).arg(milliseconds).arg(option))
    }

    /// PEXPIREAT has the same effect and semantic as EXPIREAT,
    /// but the Unix time at which the key will expire is specified in milliseconds instead of seconds.
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pexpireat/>](https://redis.io/commands/pexpireat/)
    #[must_use]
    fn pexpireat<K>(
        &mut self,
        key: K,
        unix_time_milliseconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("PEXPIREAT")
                .arg(key)
                .arg(unix_time_milliseconds)
                .arg(option),
        )
    }

    /// PEXPIRETIME has the same semantic as EXPIRETIME,
    /// but returns the absolute Unix expiration timestamp in milliseconds instead of seconds.
    ///
    /// # Return
    ///  Expiration Unix timestamp in milliseconds, or a negative value in order to signal an error (see the description below).
    /// - The command returns -1 if the key exists but has no associated expiration time.
    /// - The command returns -2 if the key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pexpiretime/>](https://redis.io/commands/pexpiretime/)
    #[must_use]
    fn pexpiretime<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("PEXPIRETIME").arg(key))
    }

    /// Returns the remaining time to live of a key that has a timeout.
    ///
    /// # Return
    /// TTL in milliseconds, or a negative value in order to signal an error:
    /// -2 if the key does not exist.
    /// -1 if the key exists but has no associated expire.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pttl/>](https://redis.io/commands/pttl/)
    #[must_use]
    fn pttl<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("PTTL").arg(key))
    }

    /// Return a random key from the currently selected database.
    ///
    /// # Return
    /// The number of references.
    ///
    /// # See Also
    /// [<https://redis.io/commands/randomkey/>](https://redis.io/commands/randomkey/)
    #[must_use]
    fn randomkey<R>(&mut self) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        R: FromValue,
    {
        prepare_command(self, cmd("RANDOMKEY"))
    }

    /// Renames key to newkey.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rename/>](https://redis.io/commands/rename/)
    #[must_use]
    fn rename<K1, K2>(&mut self, key: K1, new_key: K2) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        K1: Into<CommandArg>,
        K2: Into<CommandArg>,
    {
        prepare_command(self, cmd("RENAME").arg(key).arg(new_key))
    }

    /// Renames key to newkey if newkey does not yet exist.
    /// It returns an error when key does not exist.
    ///
    /// # Return
    /// * `true` if key was renamed to newkey.
    /// * `false` if newkey already exists.
    /// # See Also
    /// [<https://redis.io/commands/renamenx/>](https://redis.io/commands/renamenx/)
    #[must_use]
    fn renamenx<K1, K2>(&mut self, key: K1, new_key: K2) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K1: Into<CommandArg>,
        K2: Into<CommandArg>,
    {
        prepare_command(self, cmd("RENAMENX").arg(key).arg(new_key))
    }

    /// Create a key associated with a value that is obtained by deserializing
    /// the provided serialized value (obtained via DUMP).
    ///
    /// # Return
    /// Restore command builder
    ///
    /// # See Also
    /// [<https://redis.io/commands/restore/>](https://redis.io/commands/restore/)
    #[must_use]
    fn restore<K>(
        &mut self,
        key: K,
        ttl: u64,
        serialized_value: Vec<u8>,
        options: RestoreOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("RESTORE")
                .arg(key)
                .arg(ttl)
                .arg(CommandArg::Binary(serialized_value))
                .arg(options),
        )
    }

    /// Iterates the set of keys in the currently selected Redis database.
    ///
    /// # Return
    /// A list of keys
    ///
    /// # See Also
    /// [<https://redis.io/commands/scan/>](https://redis.io/commands/scan/)
    #[must_use]
    fn scan<K, A>(&mut self, cursor: u64, options: ScanOptions) -> PreparedCommand<Self, (u64, A)>
    where
        Self: Sized,
        K: FromValue,
        A: FromSingleValueArray<K>,
    {
        prepare_command(self, cmd("SCAN").arg(cursor).arg(options))
    }

    /// Returns the elements contained in the list, set or sorted set at key.
    ///
    /// # Return
    /// A collection of sorted elements.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sort/>](https://redis.io/commands/sort/)
    #[must_use]
    fn sort<K, M, A>(&mut self, key: K, options: SortOptions) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<CommandArg>,
        M: FromValue,
        A: FromSingleValueArray<M>,
    {
        prepare_command(self, cmd("SORT").arg(key).arg(options))
    }

    /// Stores the elements contained in the list, set or sorted set at key.
    ///
    /// # Return
    /// The number of sorted elements in the destination list.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sort/>](https://redis.io/commands/sort/)
    #[must_use]
    fn sort_and_store<K, D>(
        &mut self,
        key: K,
        destination: D,
        options: SortOptions,
    ) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        D: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("SORT")
                .arg(key)
                .arg(options)
                .arg("STORE")
                .arg(destination),
        )
    }

    /// Read-only variant of the SORT command.
    ///
    /// It is exactly like the original SORT but refuses the STORE option
    /// and can safely be used in read-only replicas.
    ///
    /// # Return
    /// A collection of sorted elements.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sort_ro/>](https://redis.io/commands/sort_ro/)
    #[must_use]
    fn sort_readonly<K, M, A>(&mut self, key: K, options: SortOptions) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<CommandArg>,
        M: FromValue,
        A: FromSingleValueArray<M>,
    {
        prepare_command(self, cmd("SORT_RO").arg(key).arg(options))
    }

    /// Alters the last access time of a key(s). A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were touched.
    ///
    /// # See Also
    /// [<https://redis.io/commands/touch/>](https://redis.io/commands/touch/)
    #[must_use]
    fn touch<K, KK>(&mut self, keys: KK) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        KK: SingleArgOrCollection<K>,
    {
        prepare_command(self, cmd("TOUCH").arg(keys))
    }

    /// Returns the remaining time to live of a key that has a timeout.
    ///
    /// # Return
    /// TTL in seconds, or a negative value in order to signal an error:
    /// -2 if the key does not exist.
    /// -1 if the key exists but has no associated expire.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ttl/>](https://redis.io/commands/ttl/)
    #[must_use]
    fn ttl<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("TTL").arg(key))
    }

    /// Returns the string representation of the type of the value stored at key.
    ///
    /// The different types that can be returned are: string, list, set, zset, hash and stream.
    ///
    /// # Return
    /// type of key, or empty string when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/type/>](https://redis.io/commands/type/)
    #[must_use]
    fn type_<K>(&mut self, key: K) -> PreparedCommand<Self, String>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("TYPE").arg(key))
    }

    /// This command is very similar to DEL: it removes the specified keys.
    ///
    /// # Return
    /// The number of keys that were unlinked.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unlink/>](https://redis.io/commands/unlink/)
    #[must_use]
    fn unlink<K, C>(&mut self, keys: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        C: SingleArgOrCollection<K>,
    {
        prepare_command(self, cmd("UNLINK").arg(keys))
    }

    /// This command blocks the current client until all the previous write commands are
    /// successfully transferred and acknowledged by at least the specified number of replicas.
    ///
    /// # Return
    /// The number of replicas reached by all the writes performed in the context of the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/wait/>](https://redis.io/commands/wait/)
    #[must_use]
    fn wait(&mut self, num_replicas: usize, timeout: u64) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("WAIT").arg(num_replicas).arg(timeout))
    }
}

/// Options for the [`expire`](crate::GenericCommands::expire) command
pub enum ExpireOption {
    /// No option
    None,
    /// Set expiry only when the key has no expiry
    Nx,
    /// Set expiry only when the key has no expiry    
    Xx,
    /// Set expiry only when the new expiry is greater than current one
    Gt,
    /// Set expiry only when the new expiry is less than current one
    Lt,
}

impl Default for ExpireOption {
    fn default() -> Self {
        ExpireOption::None
    }
}

impl IntoArgs for ExpireOption {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ExpireOption::None => args,
            ExpireOption::Nx => args.arg("NX"),
            ExpireOption::Xx => args.arg("XX"),
            ExpireOption::Gt => args.arg("GT"),
            ExpireOption::Lt => args.arg("LT"),
        }
    }
}

#[derive(Default)]
pub struct MigrateOptions {
    command_args: CommandArgs,
}

impl MigrateOptions {
    #[must_use]
    pub fn copy(self) -> Self {
        Self {
            command_args: self.command_args.arg("COPY"),
        }
    }

    #[must_use]
    pub fn replace(self) -> Self {
        Self {
            command_args: self.command_args.arg("REPLACE"),
        }
    }

    #[must_use]
    pub fn auth<P: Into<CommandArg>>(self, password: P) -> Self {
        Self {
            command_args: self.command_args.arg("AUTH").arg(password),
        }
    }

    #[must_use]
    pub fn auth2<U: Into<CommandArg>, P: Into<CommandArg>>(self, username: U, password: P) -> Self {
        Self {
            command_args: self.command_args.arg("AUTH2").arg(username).arg(password),
        }
    }

    #[must_use]
    pub fn keys<K: Into<CommandArg>, KK: SingleArgOrCollection<K>>(self, keys: KK) -> Self {
        Self {
            command_args: self.command_args.arg("KEYS").arg(keys),
        }
    }
}

impl IntoArgs for MigrateOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`restore`](crate::GenericCommands::restore) command
#[derive(Default)]
pub struct RestoreOptions {
    command_args: CommandArgs,
}

impl RestoreOptions {
    #[must_use]
    pub fn replace(self) -> Self {
        Self {
            command_args: self.command_args.arg("REPLACE"),
        }
    }

    #[must_use]
    pub fn abs_ttl(self) -> Self {
        Self {
            command_args: self.command_args.arg("ABSTTL"),
        }
    }

    #[must_use]
    pub fn idle_time(self, idle_time: i64) -> Self {
        Self {
            command_args: self.command_args.arg("IDLETIME").arg(idle_time),
        }
    }

    #[must_use]
    pub fn frequency(self, frequency: f64) -> Self {
        Self {
            command_args: self.command_args.arg("FREQ").arg(frequency),
        }
    }
}

impl IntoArgs for RestoreOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Order option of the [`sort`](crate::GenericCommands::sort) command
pub enum SortOrder {
    Asc,
    Desc,
}

impl IntoArgs for SortOrder {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            SortOrder::Asc => args.arg("ASC"),
            SortOrder::Desc => args.arg("DESC"),
        }
    }
}

/// Options for the [`sort`](crate::GenericCommands::sort) command
#[derive(Default)]
pub struct SortOptions {
    command_args: CommandArgs,
}

impl SortOptions {
    #[must_use]
    pub fn by<P: Into<CommandArg>>(self, pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("BY").arg(pattern),
        }
    }

    #[must_use]
    pub fn limit(self, offset: usize, count: isize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(offset).arg(count),
        }
    }

    #[must_use]
    pub fn get<P: Into<CommandArg>>(self, pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("GET").arg(pattern),
        }
    }

    #[must_use]
    pub fn order(self, order: SortOrder) -> Self {
        Self {
            command_args: self.command_args.arg(order),
        }
    }

    #[must_use]
    pub fn alpha(self) -> Self {
        Self {
            command_args: self.command_args.arg("ALPHA"),
        }
    }
}

impl IntoArgs for SortOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`dump`](crate::GenericCommands::dump) command.
pub struct DumpResult {
    pub serialized_value: Vec<u8>,
}

impl FromValue for DumpResult {
    fn from_value(value: Value) -> crate::Result<Self> {
        match value {
            Value::BulkString(Some(b)) => Ok(DumpResult {
                serialized_value: b,
            }),
            _ => Err(Error::Client("Unexpected dump format".to_owned())),
        }
    }
}

/// Options for the [`scan`](crate::GenericCommands::scan) command
#[derive(Default)]
pub struct ScanOptions {
    command_args: CommandArgs,
}

impl ScanOptions {
    #[must_use]
    pub fn match_pattern<P: Into<CommandArg>>(self, match_pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("MATCH").arg(match_pattern),
        }
    }

    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }

    #[must_use]
    pub fn type_<TY: Into<CommandArg>>(self, type_: TY) -> Self {
        Self {
            command_args: self.command_args.arg("TYPE").arg(type_),
        }
    }
}

impl IntoArgs for ScanOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`migrate`](crate::GenericCommands::migrate) command
pub enum MigrateResult {
    /// key(s) successfully migrated
    Ok,
    /// no keys were found in the source instance.
    NoKey,
}

impl FromValue for MigrateResult {
    fn from_value(value: Value) -> Result<Self> {
        let result: String = value.into()?;
        match result.as_str() {
            "OK" => Ok(Self::Ok),
            "NOKEY" => Ok(Self::NoKey),
            _ => Err(Error::Client(
                "Unexpected result for command 'MIGRATE'".to_owned(),
            )),
        }
    }
}
