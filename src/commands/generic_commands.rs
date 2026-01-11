use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{RequestPolicy, ResponsePolicy},
    resp::{BulkString, CommandArgsMut, Response, cmd, serialize_flag},
};
use serde::{Deserialize, Serialize};

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
        source: impl Serialize,
        destination: impl Serialize,
        destination_db: Option<usize>,
        replace: bool,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("COPY")
                .key(source)
                .key(destination)
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
    fn del(self, keys: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("DEL")
                .key(keys)
                .cluster_info(RequestPolicy::MultiShard, ResponsePolicy::AggSum, 1),
        )
    }

    /// Serialize the value stored at key in a Redis-specific format and return it to the user.
    ///
    /// # Return
    /// The serialized value.
    ///
    /// # See Also
    /// [<https://redis.io/commands/dump/>](https://redis.io/commands/dump/)
    #[must_use]
    fn dump(self, key: impl Serialize) -> PreparedCommand<'a, Self, BulkString> {
        prepare_command(self, cmd("DUMP").key(key))
    }

    /// Returns if keys exist.
    ///
    /// # Return
    /// The number of keys that exist from those specified as arguments.
    ///
    /// # See Also
    /// [<https://redis.io/commands/exists/>](https://redis.io/commands/exists/)
    #[must_use]
    fn exists(self, keys: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("EXISTS").key(keys).cluster_info(
                RequestPolicy::MultiShard,
                ResponsePolicy::AggSum,
                1,
            ),
        )
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
        key: impl Serialize,
        seconds: u64,
        option: impl Into<Option<ExpireOption>>,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("EXPIRE").key(key).arg(seconds).arg(option.into()))
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
        key: impl Serialize,
        unix_time_seconds: u64,
        option: impl Into<Option<ExpireOption>>,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("EXPIREAT")
                .key(key)
                .arg(unix_time_seconds)
                .arg(option.into()),
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
    fn expiretime(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("EXPIRETIME").key(key))
    }

    /// Returns all keys matching pattern.
    ///
    /// # Return
    /// list of keys matching pattern.
    ///
    /// # See Also
    /// [<https://redis.io/commands/keys/>](https://redis.io/commands/keys/)
    #[must_use]
    fn keys<R: Response>(self, pattern: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("KEYS")
                .arg(pattern)
                .cluster_info(RequestPolicy::AllShards, None, 1),
        )
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
        host: impl Serialize,
        port: u16,
        key: impl Serialize,
        destination_db: usize,
        timeout: u64,
        options: MigrateOptions,
    ) -> PreparedCommand<'a, Self, MigrateResult> {
        prepare_command(
            self,
            cmd("MIGRATE")
                .arg(host)
                .arg(port)
                .key(key)
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
    fn move_(self, key: impl Serialize, db: usize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("MOVE").key(key).arg(db))
    }

    /// Returns the internal encoding for the Redis object stored at `key`
    ///
    /// # Return
    /// The encoding of the object, or nil if the key doesn't exist
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-encoding/>](https://redis.io/commands/object-encoding/)
    #[must_use]
    fn object_encoding<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("OBJECT").arg("ENCODING").key(key))
    }

    /// This command returns the logarithmic access frequency counter of a Redis object stored at `key`.
    ///
    /// # Return
    /// The counter's value.
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-freq/>](https://redis.io/commands/object-freq/)
    #[must_use]
    fn object_freq(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("OBJECT").arg("FREQ").key(key))
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
    fn object_idle_time(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("OBJECT").arg("IDLETIME").key(key))
    }

    /// This command returns the reference count of the stored at `key`.
    ///
    /// # Return
    /// The number of references.
    ///
    /// # See Also
    /// [<https://redis.io/commands/object-refcount/>](https://redis.io/commands/object-refcount/)
    #[must_use]
    fn object_refcount(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("OBJECT").arg("REFCOUNT").key(key))
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
    fn persist(self, key: impl Serialize) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("PERSIST").key(key))
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
        key: impl Serialize,
        milliseconds: u64,
        option: impl Into<Option<ExpireOption>>,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("PEXPIRE").key(key).arg(milliseconds).arg(option.into()),
        )
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
        key: impl Serialize,
        unix_time_milliseconds: u64,
        option: impl Into<Option<ExpireOption>>,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("PEXPIREAT")
                .key(key)
                .arg(unix_time_milliseconds)
                .arg(option.into()),
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
    fn pexpiretime(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("PEXPIRETIME").key(key))
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
    fn pttl(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("PTTL").key(key))
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
        prepare_command(
            self,
            cmd("RANDOMKEY").cluster_info(RequestPolicy::AllShards, ResponsePolicy::Special, 1),
        )
    }

    /// Renames key to newkey.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rename/>](https://redis.io/commands/rename/)
    #[must_use]
    fn rename(self, key: impl Serialize, new_key: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("RENAME").key(key).key(new_key))
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
    fn renamenx(
        self,
        key: impl Serialize,
        new_key: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("RENAMENX").key(key).key(new_key))
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
        key: impl Serialize,
        ttl: u64,
        serialized_value: &BulkString,
        options: RestoreOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("RESTORE")
                .key(key)
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
        prepare_command(
            self,
            cmd("SCAN").arg(cursor).arg(options).cluster_info(
                RequestPolicy::Special,
                ResponsePolicy::Special,
                1,
            ),
        )
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
        key: impl Serialize,
        options: SortOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SORT").key(key).arg(options))
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
        key: impl Serialize,
        destination: impl Serialize,
        options: SortOptions,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("SORT")
                .key(key)
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
        key: impl Serialize,
        options: SortOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SORT_RO").key(key).arg(options))
    }

    /// Alters the last access time of a key(s). A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were touched.
    ///
    /// # See Also
    /// [<https://redis.io/commands/touch/>](https://redis.io/commands/touch/)
    #[must_use]
    fn touch(self, keys: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("TOUCH").key(keys).cluster_info(
                RequestPolicy::MultiShard,
                ResponsePolicy::AggSum,
                1,
            ),
        )
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
    fn ttl(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("TTL").key(key))
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
    fn type_<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TYPE").key(key))
    }

    /// This command is very similar to DEL: it removes the specified keys.
    ///
    /// # Return
    /// The number of keys that were unlinked.
    ///
    /// # See Also
    /// [<https://redis.io/commands/unlink/>](https://redis.io/commands/unlink/)
    #[must_use]
    fn unlink(self, keys: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("UNLINK").key(keys).cluster_info(
                RequestPolicy::MultiShard,
                ResponsePolicy::AggSum,
                1,
            ),
        )
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
        prepare_command(
            self,
            cmd("WAIT").arg(num_replicas).arg(timeout).cluster_info(
                RequestPolicy::AllShards,
                ResponsePolicy::AggMin,
                1,
            ),
        )
    }

    /// This command blocks the current client until all previous write commands by that client are acknowledged
    /// as having been fsynced to the AOF of the local Redis and/or at least the specified number of replicas.
    ///
    /// # Return
    /// A pair of two integers:
    /// 1. The first is the number of local Redises (0 or 1) that have fsynced to AOF
    ///    all writes performed in the context of the current connection
    /// 2. The second is the number of replicas that have acknowledged doing the same.
    ///
    /// # See Also
    /// [<https://redis.io/commands/waitaof/>](https://redis.io/commands/waitaof/)
    #[must_use]
    fn waitaof(
        self,
        num_local: usize,
        num_replicas: usize,
        timeout: u64,
    ) -> PreparedCommand<'a, Self, (usize, usize)> {
        prepare_command(
            self,
            cmd("WAITAOF")
                .arg(num_local)
                .arg(num_replicas)
                .arg(timeout)
                .cluster_info(RequestPolicy::AllShards, ResponsePolicy::AggMin, 1),
        )
    }
}

/// Result for the [`type`](GenericCommands::type_) command
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyType {
    String,
    List,
    Set,
    ZSet,
    Hash,
    Stream,
    VectorSet,
}

/// Options for the [`expire`](GenericCommands::expire) and [`hexpire`](crate::commands::HashCommands::hexpire) commands
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExpireOption {
    /// Set expiry only when the key has no expiry
    Nx,
    /// Set expiry only when the key has an existing expiry  
    Xx,
    /// Set expiry only when the new expiry is greater than current one
    Gt,
    /// Set expiry only when the new expiry is less than current one
    Lt,
}

/// Options for the [`migrate`](GenericCommands::migrate) command.
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct MigrateOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    copy: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    replace: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth2: Option<(&'a str, &'a str)>,
    #[serde(skip_serializing_if = "CommandArgsMut::is_empty")]
    keys: CommandArgsMut,
}

impl<'a> MigrateOptions<'a> {
    #[must_use]
    pub fn copy(mut self) -> Self {
        self.copy = true;
        self
    }

    #[must_use]
    pub fn replace(mut self) -> Self {
        self.replace = true;
        self
    }

    #[must_use]
    pub fn auth(mut self, password: &'a str) -> Self {
        self.auth = Some(password);
        self
    }

    #[must_use]
    pub fn auth2(mut self, username: &'a str, password: &'a str) -> Self {
        self.auth2 = Some((username, password));
        self
    }

    #[must_use]
    pub fn key(mut self, key: impl Serialize) -> Self {
        self.keys = self.keys.arg(key);
        self
    }
}

/// Options for the [`restore`](GenericCommands::restore) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct RestoreOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    replace: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    absttl: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    idletime: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency: Option<f64>,
}

impl RestoreOptions {
    #[must_use]
    pub fn replace(mut self) -> Self {
        self.replace = true;
        self
    }

    #[must_use]
    pub fn abs_ttl(mut self) -> Self {
        self.absttl = true;
        self
    }

    #[must_use]
    pub fn idle_time(mut self, idle_time: i64) -> Self {
        self.idletime = Some(idle_time);
        self
    }

    #[must_use]
    pub fn frequency(mut self, frequency: f64) -> Self {
        self.frequency = Some(frequency);
        self
    }
}

/// Order option of the [`sort`](GenericCommands::sort) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Options for the [`sort`](GenericCommands::sort) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct SortOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<(u32, i32)>,
    #[serde(rename = "", skip_serializing_if = "CommandArgsMut::is_empty")]
    get: CommandArgsMut,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    order: Option<SortOrder>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    alpha: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    store: Option<&'a str>,
}

impl<'a> SortOptions<'a> {
    #[must_use]
    pub fn by(mut self, pattern: &'a str) -> Self {
        self.by = Some(pattern);
        self
    }

    #[must_use]
    pub fn limit(mut self, offset: u32, count: i32) -> Self {
        self.limit = Some((offset, count));
        self
    }

    #[must_use]
    pub fn get(mut self, pattern: impl Serialize) -> Self {
        self.get = self.get.arg("GET").arg(pattern);
        self
    }

    #[must_use]
    pub fn order(mut self, order: SortOrder) -> Self {
        self.order = Some(order);
        self
    }

    #[must_use]
    pub fn alpha(mut self) -> Self {
        self.alpha = true;
        self
    }

    #[must_use]
    pub fn store(mut self, destination: &'a str) -> Self {
        self.store = Some(destination);
        self
    }
}

/// Options for the [`scan`](GenericCommands::scan) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct ScanOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#match: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<KeyType>,
}

impl<'a> ScanOptions<'a> {
    #[must_use]
    pub fn match_pattern(mut self, match_pattern: &'a str) -> Self {
        self.r#match = Some(match_pattern);
        self
    }

    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    #[must_use]
    pub fn type_(mut self, r#type: KeyType) -> Self {
        self.r#type = Some(r#type);
        self
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
