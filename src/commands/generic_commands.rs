use crate::{cmd, resp::BulkString, Command, CommandSend, Result, SingleArgOrCollection};
use futures::Future;
use std::pin::Pin;

/// A group of generic Redis commands
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=generic)
pub trait GenericCommands: CommandSend {
    /// This command copies the value stored at the source key to the destination key.
    ///
    /// # See Also
    /// [https://redis.io/commands/copy/](https://redis.io/commands/copy/)
    fn copy<S, D>(&self, source: S, destination: D) -> Copy<Self>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
    {
        Copy {
            generic_commands: &self,
            cmd: cmd("COPY").arg(source).arg(destination),
        }
    }

    /// Removes the specified keys. A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were removed.
    ///
    /// # See Also
    /// [https://redis.io/commands/del/](https://redis.io/commands/del/)
    fn del<K, C>(&self, keys: C) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(cmd("DEL").arg(keys))
    }

    /// Returns if keys exist.
    ///
    /// # Return
    /// The number of keys that exist from those specified as arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/exists/](https://redis.io/commands/exists/)
    fn exists<K, C>(&self, keys: C) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(cmd("EXISTS").arg(keys))
    }

    /// Set a timeout on key in seconds
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/expire/](https://redis.io/commands/expire/)
    fn expire<K>(&self, key: K, seconds: u64) -> Expire<Self>
    where
        K: Into<BulkString>,
    {
        Expire {
            generic_commands: &self,
            cmd: cmd("EXPIRE").arg(key).arg(seconds),
        }
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
    fn expireat<K>(&self, key: K, unix_time_seconds: u64) -> Expire<Self>
    where
        K: Into<BulkString>,
    {
        Expire {
            generic_commands: &self,
            cmd: cmd("EXPIREAT").arg(key).arg(unix_time_seconds),
        }
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
    fn expiretime<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("EXPIRETIME").arg(key))
    }

    /// Move key from the currently selected database to the specified destination database.
    ///
    /// # Return
    /// * `true` - if key was moved.
    /// * `false` - f key was not moved.
    ///
    /// # See Also
    /// [https://redis.io/commands/move/](https://redis.io/commands/move/)
    fn move_<K>(&self, key: K, db: usize) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("MOVE").arg(key).arg(db))
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
    fn persist<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("PERSIST").arg(key))
    }

    /// This command works exactly like EXPIRE but the time to live of the key is specified in milliseconds instead of seconds.
    ///
    /// # Return
    /// * `true` - if the timeout was set.
    /// * `false` - if the timeout was not set. e.g. key doesn't exist, or operation skipped due to the provided arguments.
    ///
    /// # See Also
    /// [https://redis.io/commands/pexpire/](https://redis.io/commands/pexpire/)
    fn pexpire<K>(&self, key: K, milliseconds: u64) -> Expire<Self>
    where
        K: Into<BulkString>,
    {
        Expire {
            generic_commands: &self,
            cmd: cmd("PEXPIRE").arg(key).arg(milliseconds),
        }
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
    fn pexpireat<K>(&self, key: K, unix_time_milliseconds: u64) -> Expire<Self>
    where
        K: Into<BulkString>,
    {
        Expire {
            generic_commands: &self,
            cmd: cmd("PEXPIREAT").arg(key).arg(unix_time_milliseconds),
        }
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
    fn pexpiretime<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("PEXPIRETIME").arg(key))
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
    fn pttl<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("PTTL").arg(key))
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
    fn ttl<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("TTL").arg(key))
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
    fn type_<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<String>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("TYPE").arg(key))
    }
}

/// Builder for the [copy](crate::GenericCommands::copy) command
pub struct Copy<'a, T: GenericCommands + ?Sized> {
    generic_commands: &'a T,
    cmd: Command,
}

impl<'a, T: GenericCommands> Copy<'a, T> {
    /// Allows specifying an alternative logical database index for the destination key.
    pub fn db(self, destination_db: usize) -> Self {
        Self {
            generic_commands: self.generic_commands,
            cmd: self.cmd.arg("DB").arg(destination_db),
        }
    }

    /// Removes the destination key before copying the value to it
    pub fn replace(self) -> Self {
        Self {
            generic_commands: self.generic_commands,
            cmd: self.cmd.arg("REPLACE"),
        }
    }

    /// Execute the command
    ///
    /// # Return
    ///  Success of the operation
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a>> {
        self.generic_commands.send_into(self.cmd)
    }
}

/// Builder for the [expire](crate::GenericCommands::expire) command
pub struct Expire<'a, T: GenericCommands + ?Sized> {
    generic_commands: &'a T,
    cmd: Command,
}

impl<'a, T: GenericCommands> Expire<'a, T> {
    /// Set expiry only when the key has no expiry
    pub fn nx(self) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a>> {
        self.generic_commands.send_into(self.cmd.arg("NX"))
    }

    /// Set expiry only when the key has an existing expiry
    pub fn xx(self) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a>> {
        self.generic_commands.send_into(self.cmd.arg("XX"))
    }

    /// Set expiry only when the new expiry is greater than current one
    pub fn gt(self) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a>> {
        self.generic_commands.send_into(self.cmd.arg("GT"))
    }

    /// Set expiry only when the new expiry is less than current one
    pub fn lt(self) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a>> {
        self.generic_commands.send_into(self.cmd.arg("LT"))
    }

    /// execute with no option
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<bool>> + 'a>> {
        self.generic_commands.send_into(self.cmd)
    }
}
