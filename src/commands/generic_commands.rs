use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, CommandArgs, Response, cmd, deserialize_byte_buf},
};
use serde::Deserialize;

/// A group of generic Redis commands
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=generic)
pub trait GenericCommands<'a>: Sized {
    /// This command copies the value stored at the source key to the destination key.
    ///
    /// # Return
    /// Success of the operation
    ///
    /// # See Also
    /// [<https://redis.io/commands/copy/>](https://redis.io/commands/copy/)
    #[must_use]
    fn copy(
        self,
        source: impl Args,
        destination: impl Args,
        destination_db: Option<usize>,
        replace: bool,
    ) -> PreparedCommand<'a, Self, bool> {
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
    fn del(self, keys: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn dump(self, key: impl Args) -> PreparedCommand<'a, Self, DumpResult> {
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
    fn exists(self, keys: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn expire(
        self,
        key: impl Args,
        seconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<'a, Self, bool> {
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
    fn expireat(
        self,
        key: impl Args,
        unix_time_seconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<'a, Self, bool> {
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
    fn expiretime(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
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
    fn keys<R: Response>(self, pattern: impl Args) -> PreparedCommand<'a, Self, R> {
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
    fn migrate(
        self,
        host: impl Args,
        port: u16,
        key: impl Args,
        destination_db: usize,
        timeout: u64,
        options: MigrateOptions,
    ) -> PreparedCommand<'a, Self, MigrateResult> {
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
    fn move_(self, key: impl Args, db: usize) -> PreparedCommand<'a, Self, i64> {
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
    fn object_encoding<R: Response>(self, key: impl Args) -> PreparedCommand<'a, Self, R> {
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
    fn object_freq(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("OBJECT").arg("FREQ").arg(key))
    }

    /// The command returns a helpful text describing the different OBJECT subcommands.
    ///
    /// # Return
    /// The array strings.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::GenericCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// # #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// # async fn main() -> Result<()> {
    /// #     let client = Client::connect("127.0.0.1:6379").await?;
    /// let result: Vec<String> = client.object_help().await?;
    /// assert!(result.iter().any(|e| e == "HELP"));
    /// #     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/docs/latest/commands/object-help/>](https://redis.io/docs/latest/commands/object-help/)
    #[must_use]
    fn object_help<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("OBJECT").arg("HELP"))
    }

    /// This command returns the time in seconds since the last access to the value stored at `key`.
    ///
    /// # Return
    /// The idle time in seconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-idletime/>](https://redis.io/commands/object-idletime/)
    #[must_use]
    fn object_idle_time(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
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
    fn object_refcount(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
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
    fn persist(self, key: impl Args) -> PreparedCommand<'a, Self, bool> {
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
    fn pexpire(
        self,
        key: impl Args,
        milliseconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<'a, Self, bool> {
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
    fn pexpireat(
        self,
        key: impl Args,
        unix_time_milliseconds: u64,
        option: ExpireOption,
    ) -> PreparedCommand<'a, Self, bool> {
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
    fn pexpiretime(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
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
    fn pttl(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
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
    fn randomkey<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("RANDOMKEY"))
    }

    /// Renames key to newkey.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rename/>](https://redis.io/commands/rename/)
    #[must_use]
    fn rename(self, key: impl Args, new_key: impl Args) -> PreparedCommand<'a, Self, ()> {
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
    fn renamenx(self, key: impl Args, new_key: impl Args) -> PreparedCommand<'a, Self, bool> {
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
    fn restore(
        self,
        key: impl Args,
        ttl: u64,
        serialized_value: Vec<u8>,
        options: RestoreOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("RESTORE")
                .arg(key)
                .arg(ttl)
                .arg(serialized_value)
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
    fn scan<R: Response>(self, cursor: u64, options: ScanOptions) -> PreparedCommand<'a, Self, R> {
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
    fn sort<R: Response>(
        self,
        key: impl Args,
        options: SortOptions,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn sort_and_store(
        self,
        key: impl Args,
        destination: impl Args,
        options: SortOptions,
    ) -> PreparedCommand<'a, Self, usize> {
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
    fn sort_readonly<R: Response>(
        self,
        key: impl Args,
        options: SortOptions,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn touch(self, keys: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn ttl(self, key: impl Args) -> PreparedCommand<'a, Self, i64> {
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
    fn type_(self, key: impl Args) -> PreparedCommand<'a, Self, String> {
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
    fn unlink(self, keys: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn wait(self, num_replicas: usize, timeout: u64) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("WAIT").arg(num_replicas).arg(timeout))
    }
}

/// Options for the [`expire`](GenericCommands::expire) and [`hexpire`](crate::commands::HashCommands::hexpire) commands
#[derive(Default)]
pub enum ExpireOption {
    /// No option
    #[default]
    None,
    /// Set expiry only when the key has no expiry
    Nx,
    /// Set expiry only when the key has an existing expiry  
    Xx,
    /// Set expiry only when the new expiry is greater than current one
    Gt,
    /// Set expiry only when the new expiry is less than current one
    Lt,
}

impl Args for ExpireOption {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ExpireOption::None => {}
            ExpireOption::Nx => {
                args.arg("NX");
            }
            ExpireOption::Xx => {
                args.arg("XX");
            }
            ExpireOption::Gt => {
                args.arg("GT");
            }
            ExpireOption::Lt => {
                args.arg("LT");
            }
        }
    }
}

/// Options for the [`migrate`](GenericCommands::migrate) command.
#[derive(Default)]
pub struct MigrateOptions {
    command_args: CommandArgs,
}

impl MigrateOptions {
    #[must_use]
    pub fn copy(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("COPY").build(),
        }
    }

    #[must_use]
    pub fn replace(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("REPLACE").build(),
        }
    }

    #[must_use]
    pub fn auth(mut self, password: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg("AUTH").arg(password).build(),
        }
    }

    #[must_use]
    pub fn auth2(mut self, username: impl Args, password: impl Args) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("AUTH2")
                .arg(username)
                .arg(password)
                .build(),
        }
    }

    #[must_use]
    pub fn keys(mut self, keys: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg("KEYS").arg(keys).build(),
        }
    }
}

impl Args for MigrateOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`restore`](GenericCommands::restore) command
#[derive(Default)]
pub struct RestoreOptions {
    command_args: CommandArgs,
}

impl RestoreOptions {
    #[must_use]
    pub fn replace(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("REPLACE").build(),
        }
    }

    #[must_use]
    pub fn abs_ttl(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("ABSTTL").build(),
        }
    }

    #[must_use]
    pub fn idle_time(mut self, idle_time: i64) -> Self {
        Self {
            command_args: self.command_args.arg("IDLETIME").arg(idle_time).build(),
        }
    }

    #[must_use]
    pub fn frequency(mut self, frequency: f64) -> Self {
        Self {
            command_args: self.command_args.arg("FREQ").arg(frequency).build(),
        }
    }
}

impl Args for RestoreOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Order option of the [`sort`](GenericCommands::sort) command
pub enum SortOrder {
    Asc,
    Desc,
}

impl Args for SortOrder {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            SortOrder::Asc => args.arg("ASC"),
            SortOrder::Desc => args.arg("DESC"),
        };
    }
}

/// Options for the [`sort`](GenericCommands::sort) command
#[derive(Default)]
pub struct SortOptions {
    command_args: CommandArgs,
}

impl SortOptions {
    #[must_use]
    pub fn by(mut self, pattern: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg("BY").arg(pattern).build(),
        }
    }

    #[must_use]
    pub fn limit(mut self, offset: usize, count: isize) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LIMIT")
                .arg(offset)
                .arg(count)
                .build(),
        }
    }

    #[must_use]
    pub fn get(mut self, pattern: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg("GET").arg(pattern).build(),
        }
    }

    #[must_use]
    pub fn order(mut self, order: SortOrder) -> Self {
        Self {
            command_args: self.command_args.arg(order).build(),
        }
    }

    #[must_use]
    pub fn alpha(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("ALPHA").build(),
        }
    }
}

impl Args for SortOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`dump`](GenericCommands::dump) command.
#[derive(Deserialize)]
pub struct DumpResult(#[serde(deserialize_with = "deserialize_byte_buf")] pub Vec<u8>);

/// Options for the [`scan`](GenericCommands::scan) command
#[derive(Default)]
pub struct ScanOptions {
    command_args: CommandArgs,
}

impl ScanOptions {
    #[must_use]
    pub fn match_pattern(mut self, match_pattern: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg("MATCH").arg(match_pattern).build(),
        }
    }

    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }

    #[must_use]
    pub fn type_(mut self, type_: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg("TYPE").arg(type_).build(),
        }
    }
}

impl Args for ScanOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`migrate`](GenericCommands::migrate) command
#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MigrateResult {
    /// key(s) successfully migrated
    Ok,
    /// no keys were found in the source instance.
    NoKey,
}
