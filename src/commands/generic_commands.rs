use std::marker::PhantomData;

use crate::{
    cmd,
    resp::{BulkString, FromSingleValueArray, FromValue, Value},
    CommandArgs, CommandResult, Error, IntoArgs, IntoCommandResult, SingleArgOrCollection,
};

/// A group of generic Redis commands
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=generic)
pub trait GenericCommands<T>: IntoCommandResult<T> {
    /// This command copies the value stored at the source key to the destination key.
    ///
    /// # Return
    /// Success of the operation
    ///
    /// # See Also
    /// [https://redis.io/commands/copy/](https://redis.io/commands/copy/)
    fn copy<S, D>(
        &self,
        source: S,
        destination: D,
        destination_db: Option<usize>,
        replace: bool,
    ) -> CommandResult<T, bool>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
    {
        self.into_command_result(
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
    /// [https://redis.io/commands/del/](https://redis.io/commands/del/)
    fn del<K, C>(&self, keys: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.into_command_result(cmd("DEL").arg(keys))
    }

    /// Serialize the value stored at key in a Redis-specific format and return it to the user.
    ///
    /// # Return
    /// The serialized value.
    ///
    /// # See Also
    /// [https://redis.io/commands/dump/](https://redis.io/commands/dump/)
    fn dump<K>(&self, key: K) -> CommandResult<T, DumpResult>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("DUMP").arg(key))
    }

    /// Returns if keys exist.
    ///
    /// # Return
    /// The number of keys that exist from those specified as arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/exists/](https://redis.io/commands/exists/)
    fn exists<K, C>(&self, keys: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.into_command_result(cmd("EXISTS").arg(keys))
    }

    /// Set a timeout on key in seconds
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/expire/](https://redis.io/commands/expire/)
    fn expire<K>(
        &self,
        key: K,
        seconds: u64,
        option: ExpireOption,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("EXPIRE").arg(key).arg(seconds).arg(option))
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
    /// [https://redis.io/commands/expireat/](https://redis.io/commands/expireat/)
    fn expireat<K>(
        &self,
        key: K,
        unix_time_seconds: u64,
        option: Option<ExpireOption>,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("EXPIREAT").arg(key).arg(unix_time_seconds).arg(option))
    }

    /// Returns the absolute Unix timestamp (since January 1, 1970) in seconds at which the given key will expire.
    ///
    /// # Return
    /// Expiration Unix timestamp in seconds, or a negative value in order to signal an error (see the description below).
    /// - The command returns -1 if the key exists but has no associated expiration time.
    /// - The command returns -2 if the key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/expiretime/](https://redis.io/commands/expiretime/)
    fn expiretime<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("EXPIRETIME").arg(key))
    }

    /// Returns all keys matching pattern.
    ///
    /// # Return
    /// list of keys matching pattern.
    ///
    /// # See Also
    /// [https://redis.io/commands/keys/](https://redis.io/commands/keys/)
    fn keys<P, K, A>(&self, pattern: P) -> CommandResult<T, A>
    where
        P: Into<BulkString>,
        K: FromValue,
        A: FromSingleValueArray<K>,
    {
        self.into_command_result(cmd("KEYS").arg(pattern))
    }

    /// Atomically transfer a key or a collection of keys from a source Redis instance to a destination Redis instance.
    ///
    /// # Return
    /// * `true` - on success
    /// * `false` - if no keys were found in the source instance.
    ///
    /// # See Also
    /// [https://redis.io/commands/migrate/](https://redis.io/commands/migrate/)
    fn migrate<H, K, P1, U, P2, KK>(
        &self,
        host: H,
        port: u16,
        key: K,
        destination_db: usize,
        timeout: u64,
        options: MigrateOptions<P1, U, P2, K, KK>,
    ) -> CommandResult<T, bool>
    where
        H: Into<BulkString>,
        K: Into<BulkString>,
        P1: Into<BulkString>,
        U: Into<BulkString>,
        P2: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
    {
        self.into_command_result(
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
    /// [https://redis.io/commands/move/](https://redis.io/commands/move/)
    fn move_<K>(&self, key: K, db: usize) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("MOVE").arg(key).arg(db))
    }

    /// Returns the internal encoding for the Redis object stored at `key`
    ///
    /// # Return
    /// The encoding of the object, or nil if the key doesn't exist
    ///
    /// # See Also
    /// [https://redis.io/commands/object-encoding/](https://redis.io/commands/object-encoding/)
    fn object_encoding<K, E>(&self, key: K) -> CommandResult<T, E>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.into_command_result(cmd("OBJECT").arg("ENCODING").arg(key))
    }

    /// This command returns the logarithmic access frequency counter of a Redis object stored at `key`.
    ///
    /// # Return
    /// The counter's value.
    ///
    /// # See Also
    /// [https://redis.io/commands/object-freq/](https://redis.io/commands/object-freq/)
    fn object_freq<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("OBJECT").arg("FREQ").arg(key))
    }

    /// This command returns the time in seconds since the last access to the value stored at `key`.
    ///
    /// # Return
    /// The idle time in seconds.
    ///
    /// # See Also
    /// [https://redis.io/commands/object-idletime/](https://redis.io/commands/object-idletime/)
    fn object_idle_time<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("OBJECT").arg("IDLETIME").arg(key))
    }

    /// This command returns the reference count of the stored at `key`.
    ///
    /// # Return
    /// The number of references.
    ///
    /// # See Also
    /// [https://redis.io/commands/object-refcount/](https://redis.io/commands/object-refcount/)
    fn object_refcount<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("OBJECT").arg("REFCOUNT").arg(key))
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
    /// [https://redis.io/commands/persist/](https://redis.io/commands/persist/)
    fn persist<K>(&self, key: K) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("PERSIST").arg(key))
    }

    /// This command works exactly like EXPIRE but the time to live of the key is specified in milliseconds instead of seconds.
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/pexpire/](https://redis.io/commands/pexpire/)
    fn pexpire<K>(
        &self,
        key: K,
        milliseconds: u64,
        option: Option<ExpireOption>,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("PEXPIRE").arg(key).arg(milliseconds).arg(option))
    }

    /// PEXPIREAT has the same effect and semantic as EXPIREAT,
    /// but the Unix time at which the key will expire is specified in milliseconds instead of seconds.
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/pexpireat/](https://redis.io/commands/pexpireat/)
    fn pexpireat<K>(
        &self,
        key: K,
        unix_time_milliseconds: u64,
        option: Option<ExpireOption>,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(
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
    /// [https://redis.io/commands/pexpiretime/](https://redis.io/commands/pexpiretime/)
    fn pexpiretime<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("PEXPIRETIME").arg(key))
    }

    /// Returns the remaining time to live of a key that has a timeout.
    ///
    /// # Return
    /// TTL in milliseconds, or a negative value in order to signal an error:
    /// -2 if the key does not exist.
    /// -1 if the key exists but has no associated expire.
    ///
    /// # See Also
    /// [https://redis.io/commands/pttl/](https://redis.io/commands/pttl/)
    fn pttl<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("PTTL").arg(key))
    }

    /// Return a random key from the currently selected database.
    ///
    /// # Return
    /// The number of references.
    ///
    /// # See Also
    /// [https://redis.io/commands/randomkey/](https://redis.io/commands/randomkey/)
    fn randomkey<R>(&self) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.into_command_result(cmd("RANDOMKEY"))
    }

    /// Renames key to newkey.
    ///
    /// # See Also
    /// [https://redis.io/commands/rename/](https://redis.io/commands/rename/)
    fn rename<K1, K2>(&self, key: K1, new_key: K2) -> CommandResult<T, ()>
    where
        K1: Into<BulkString>,
        K2: Into<BulkString>,
    {
        self.into_command_result(cmd("RENAME").arg(key).arg(new_key))
    }

    /// Renames key to newkey if newkey does not yet exist.
    /// It returns an error when key does not exist.
    ///
    /// # Return
    /// * `true` if key was renamed to newkey.
    /// * `false` if newkey already exists.
    /// # See Also
    /// [https://redis.io/commands/renamenx/](https://redis.io/commands/renamenx/)
    fn renamenx<K1, K2>(&self, key: K1, new_key: K2) -> CommandResult<T, bool>
    where
        K1: Into<BulkString>,
        K2: Into<BulkString>,
    {
        self.into_command_result(cmd("RENAMENX").arg(key).arg(new_key))
    }

    /// Create a key associated with a value that is obtained by deserializing
    /// the provided serialized value (obtained via DUMP).
    ///
    /// # Return
    /// Restore command builder
    ///
    /// # See Also
    /// [https://redis.io/commands/restore/](https://redis.io/commands/restore/)
    fn restore<K>(
        &self,
        key: K,
        ttl: u64,
        serialized_value: Vec<u8>,
        replace: bool,
        abs_ttl: bool,
        idle_time: Option<i64>,
        frequency: Option<f64>,
    ) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(
            cmd("RESTORE")
                .arg(key)
                .arg(ttl)
                .arg(BulkString::Binary(serialized_value))
                .arg_if(replace, "REPLACE")
                .arg_if(abs_ttl, "ABSTTL")
                .arg(idle_time.map(|idle_time| ("IDLETIME", idle_time)))
                .arg(frequency.map(|frequency| ("FREQ", frequency))),
        )
    }

    /// Iterates the set of keys in the currently selected Redis database.
    ///
    /// # Return
    /// A list of keys
    ///
    /// # See Also
    /// [https://redis.io/commands/scan/](https://redis.io/commands/scan/)
    fn scan<P, TY, K, A>(
        &self,
        cursor: u64,
        match_pattern: Option<P>,
        count: Option<usize>,
        type_: Option<TY>,
    ) -> CommandResult<T, (u64, A)>
    where
        P: Into<BulkString>,
        TY: Into<BulkString>,
        K: FromValue,
        A: FromSingleValueArray<K> + Default,
    {
        self.into_command_result(
            cmd("SCAN")
                .arg(cursor)
                .arg(match_pattern.map(|p| ("MATCH", p)))
                .arg(count.map(|c| ("COUNT", c)))
                .arg(type_.map(|t| ("TYPE", t))),
        )
    }

    /// Returns the elements contained in the list, set or sorted set at key.
    ///
    /// # Return
    /// A collection of sorted elements.
    ///
    /// # See Also
    /// [https://redis.io/commands/sort/](https://redis.io/commands/sort/)
    fn sort<K, BP, GP, M, A>(&self, key: K, options: SortOptions<BP, GP>) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        BP: Into<BulkString>,
        GP: Into<BulkString>,
        M: FromValue,
        A: FromSingleValueArray<M>,
    {
        self.into_command_result(cmd("SORT").arg(key).arg(options))
    }

    /// Stores the elements contained in the list, set or sorted set at key.
    ///
    /// # Return
    /// The number of sorted elements in the destination list.
    ///
    /// # See Also
    /// [https://redis.io/commands/sort/](https://redis.io/commands/sort/)
    fn sort_and_store<K, BP, GP, D>(
        &self,
        key: K,
        destination: D,
        options: SortOptions<BP, GP>,
    ) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        BP: Into<BulkString>,
        GP: Into<BulkString>,
        D: Into<BulkString>,
    {
        self.into_command_result(
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
    /// [https://redis.io/commands/sort_ro/](https://redis.io/commands/sort_ro/)
    fn sort_readonly<K, BP, GP, M, A>(
        &self,
        key: K,
        options: SortOptions<BP, GP>,
    ) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        BP: Into<BulkString>,
        GP: Into<BulkString>,
        M: FromValue,
        A: FromSingleValueArray<M>,
    {
        self.into_command_result(cmd("SORT_RO").arg(key).arg(options))
    }

    /// Alters the last access time of a key(s). A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were touched.
    ///
    /// # See Also
    /// [https://redis.io/commands/touch/](https://redis.io/commands/touch/)
    fn touch<K, KK>(&self, keys: KK) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
    {
        self.into_command_result(cmd("TOUCH").arg(keys))
    }

    /// Returns the remaining time to live of a key that has a timeout.
    ///
    /// # Return
    /// TTL in seconds, or a negative value in order to signal an error:
    /// -2 if the key does not exist.
    /// -1 if the key exists but has no associated expire.
    ///
    /// # See Also
    /// [https://redis.io/commands/ttl/](https://redis.io/commands/ttl/)
    fn ttl<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("TTL").arg(key))
    }

    /// Returns the string representation of the type of the value stored at key.
    ///
    /// The different types that can be returned are: string, list, set, zset, hash and stream.
    ///
    /// # Return
    /// type of key, or empty string when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/type/](https://redis.io/commands/type/)
    fn type_<K>(&self, key: K) -> CommandResult<T, String>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("TYPE").arg(key))
    }

    /// This command is very similar to DEL: it removes the specified keys.
    ///
    /// # Return
    /// The number of keys that were unlinked.
    ///
    /// # See Also
    /// [https://redis.io/commands/unlink/](https://redis.io/commands/unlink/)
    fn unlink<K, C>(&self, keys: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.into_command_result(cmd("UNLINK").arg(keys))
    }

    /// This command blocks the current client until all the previous write commands are
    /// successfully transferred and acknowledged by at least the specified number of replicas.
    ///
    /// # Return
    /// The number of replicas reached by all the writes performed in the context of the current connection.
    ///
    /// # See Also
    /// [https://redis.io/commands/wait/](https://redis.io/commands/wait/)
    fn wait(&self, num_replicas: usize, timeout: u64) -> CommandResult<T, usize> {
        self.into_command_result(cmd("WAIT").arg(num_replicas).arg(timeout))
    }
}

/// Options for the [expire](crate::GenericCommands::expire) command
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
pub struct MigrateOptions<
    P1 = &'static str,
    U = &'static str,
    P2 = &'static str,
    K = &'static str,
    KK = Vec<&'static str>,
> where
    P1: Into<BulkString>,
    U: Into<BulkString>,
    P2: Into<BulkString>,
    K: Into<BulkString>,
    KK: SingleArgOrCollection<K>,
{
    phantom: PhantomData<K>,
    copy: bool,
    replace: bool,
    auth: Option<P1>,
    auth2: Option<(U, P2)>,
    keys: Option<KK>,
}

impl<P1, U, P2, K, KK> MigrateOptions<P1, U, P2, K, KK>
where
    P1: Into<BulkString>,
    U: Into<BulkString>,
    P2: Into<BulkString>,
    K: Into<BulkString>,
    KK: SingleArgOrCollection<K>,
{
    pub fn copy(self) -> Self {
        Self {
            phantom: PhantomData,
            copy: true,
            replace: self.replace,
            auth: self.auth,
            auth2: self.auth2,
            keys: self.keys,
        }
    }

    pub fn auth(self, password: P1) -> Self {
        Self {
            phantom: PhantomData,
            copy: self.copy,
            replace: self.replace,
            auth: Some(password),
            auth2: self.auth2,
            keys: self.keys,
        }
    }

    pub fn auth2(self, username: U, password: P2) -> Self {
        Self {
            phantom: PhantomData,
            copy: self.copy,
            replace: self.replace,
            auth: self.auth,
            auth2: Some((username, password)),
            keys: self.keys,
        }
    }

    pub fn keys(self, keys: KK) -> Self {
        Self {
            phantom: PhantomData,
            copy: self.copy,
            replace: self.replace,
            auth: self.auth,
            auth2: self.auth2,
            keys: Some(keys),
        }
    }
}

impl<P1, U, P2, K, KK> IntoArgs for MigrateOptions<P1, U, P2, K, KK>
where
    P1: Into<BulkString>,
    U: Into<BulkString>,
    P2: Into<BulkString>,
    K: Into<BulkString>,
    KK: SingleArgOrCollection<K>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg_if(self.copy, "COPY")
            .arg_if(self.replace, "REPLACE")
            .arg(self.auth.map(|pwd| ("AUTH", pwd)))
            .arg(self.auth2.map(|(user, pwd)| ("AUTH2", user, pwd)))
            .arg(self.keys)
    }
}

/// Order option of the [sort](crate::GenericCommands::sort) command
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

/// Options of the [sort](crate::GenericCommands::sort) command
#[derive(Default)]
pub struct SortOptions<BP = &'static str, GP = &'static str>
where
    BP: Into<BulkString>,
    GP: Into<BulkString>,
{
    by_pattern: Option<BP>,
    limit: Option<(usize, isize)>,
    get_patterns: Option<Vec<GP>>,
    order: Option<SortOrder>,
    alpha: bool,
}

impl<BP, GP> SortOptions<BP, GP>
where
    BP: Into<BulkString>,
    GP: Into<BulkString>,
{
    pub fn by(self, pattern: BP) -> Self {
        Self {
            by_pattern: Some(pattern),
            limit: self.limit,
            get_patterns: self.get_patterns,
            order: self.order,
            alpha: self.alpha,
        }
    }

    pub fn limit(self, offset: usize, count: isize) -> Self {
        Self {
            by_pattern: self.by_pattern,
            limit: Some((offset, count)),
            get_patterns: self.get_patterns,
            order: self.order,
            alpha: self.alpha,
        }
    }

    pub fn get(self, patterns: Vec<GP>) -> Self {
        Self {
            by_pattern: self.by_pattern,
            limit: self.limit,
            get_patterns: Some(patterns),
            order: self.order,
            alpha: self.alpha,
        }
    }

    pub fn order(self, order: SortOrder) -> Self {
        Self {
            by_pattern: self.by_pattern,
            limit: self.limit,
            get_patterns: self.get_patterns,
            order: Some(order),
            alpha: self.alpha,
        }
    }

    pub fn alpha(self) -> Self {
        Self {
            by_pattern: self.by_pattern,
            limit: self.limit,
            get_patterns: self.get_patterns,
            order: self.order,
            alpha: true,
        }
    }
}

impl<BP, GP> IntoArgs for SortOptions<BP, GP>
where
    BP: Into<BulkString>,
    GP: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.by_pattern.map(|bp| ("BY", bp)))
            .arg(self.limit.map(|(offset, count)| ("LIMIT", offset, count)))
            .arg(
                self.get_patterns
                    .map(|patterns| patterns.into_iter().map(|p| ("GET", p)).collect::<Vec<_>>()),
            )
            .arg(self.order)
            .arg_if(self.alpha, "ALPHA")
    }
}

/// Result for the [dump](crate::GenericCommands::dump) command.
pub struct DumpResult {
    pub serialized_value: Vec<u8>,
}

impl FromValue for DumpResult {
    fn from_value(value: Value) -> crate::Result<Self> {
        match value {
            Value::BulkString(BulkString::Binary(b)) => Ok(DumpResult {
                serialized_value: b,
            }),
            _ => Err(Error::Internal("Unexpected dump format".to_owned())),
        }
    }
}
