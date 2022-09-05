use crate::{
    cmd,
    resp::{Array, BulkString, FromValue, Value},
    Command, CommandSend, Error, Result, SingleArgOrCollection, KeyValueArgOrCollection,
};
use futures::Future;
use std::pin::Pin;

/// A group of Redis commands related to Strings
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=string)
pub trait StringCommands: CommandSend {
    /// If key already exists and is a string,
    /// this command appends the value at the end of the string.
    /// If key does not exist it is created and set as an empty string,
    /// so APPEND will be similar to SET in this special case.
    ///
    /// # Return
    /// the length of the string after the append operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/append/](https://redis.io/commands/append/)
    fn append<K, V>(&self, key: K, value: V) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("APPEND").arg(key).arg(value))
    }

    /// Decrements the number stored at key by one.
    ///
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type or contains
    /// a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// # Return
    /// the value of key after the decrement
    ///
    /// # See Also
    /// [https://redis.io/commands/decr/](https://redis.io/commands/decr/)
    fn decr<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("DECR").arg(key))
    }

    /// Decrements the number stored at key by one.
    ///
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type or contains
    /// a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// # Return
    /// the value of key after the decrement
    ///
    /// # See Also
    /// [https://redis.io/commands/decrby/](https://redis.io/commands/decrby/)
    fn decrby<K>(&self, key: K, decrement: i64) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("DECRBY").arg(key).arg(decrement))
    }

    /// Get the value of key.
    ///
    /// Decrements the number stored at key by decrement.
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type
    /// or contains a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// # Return
    /// the value of key, or nil when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/get/](https://redis.io/commands/get/)
    fn get<K, V>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<V>> + '_>>
    where
        K: Into<BulkString>,
        V: FromValue,
        Self: Sized,
    {
        self.send_into(cmd("GET").arg(key))
    }

    /// Get the value of key and delete the key.
    ///
    /// This command is similar to GET, except for the fact that it also deletes the key on success
    /// (if and only if the key's value type is a string).
    ///
    /// # Return
    /// the value of key, nil when key does not exist, or an error if the key's value type isn't a string.
    ///
    /// # See Also
    /// [https://redis.io/commands/getdel/](https://redis.io/commands/getdel/)
    fn getdel<K, V>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<V>> + '_>>
    where
        K: Into<BulkString>,
        V: FromValue,
    {
        self.send_into(cmd("GETDEL").arg(key))
    }

    ///   Get the value of key and optionally set its expiration. GETEX is similar to GET, but is a write command with additional options.
    ///
    /// Decrements the number stored at key by decrement.
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type
    /// or contains a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// # Return
    /// the value of key, or nil when key does not exist.
    ///
    /// # Example
    /// ```ignore
    /// let connection = redis::ConnectionMultiplexer::connect("127.0.0.1:6379").await?;
    /// let database = connection.get_default_database();
    /// let value: String = database.getex("key").ex(60).await?;
    /// ```
    ///
    /// # See Also
    /// [https://redis.io/commands/getex/](https://redis.io/commands/getex/)
    fn getex<K>(&self, key: K) -> GetEx<Self>
    where
        K: Into<BulkString>,
    {
        GetEx {
            string_commands: &self,
            cmd: cmd("GETEX").arg(key),
        }
    }

    /// Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).
    ///
    /// Negative offsets can be used in order to provide an offset starting from the end of the string.
    /// So -1 means the last character, -2 the penultimate and so forth.
    ///
    /// The function handles out of range requests by limiting the resulting range to the actual length of the string.

    /// # See Also
    /// [https://redis.io/commands/getrange/](https://redis.io/commands/getrange/)
    fn getrange<K, V>(
        &self,
        key: K,
        start: usize,
        end: isize,
    ) -> Pin<Box<dyn Future<Output = Result<V>> + '_>>
    where
        K: Into<BulkString>,
        V: FromValue,
    {
        self.send_into(cmd("GETRANGE").arg(key).arg(start).arg(end))
    }

    /// Atomically sets key to value and returns the old value stored at key.
    /// Returns an error when key exists but does not hold a string value.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # Return
    /// the old value stored at key, or nil when key did not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/getset/](https://redis.io/commands/getset/)
    fn getset<K, V, R>(&self, key: K, value: V) -> Pin<Box<dyn Future<Output = Result<R>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
        R: FromValue,
    {
        self.send_into(cmd("GETSET").arg(key).arg(value))
    }

    /// Increments the number stored at key by one.
    ///
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type
    /// or contains a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// Note: this is a string operation because Redis does not have a dedicated integer type.
    /// The string stored at the key is interpreted as a base-10 64 bit signed integer to execute the operation.
    ///
    /// Redis stores integers in their integer representation, so for string values that actually hold an integer,
    /// there is no overhead for storing the string representation of the integer.
    ///
    /// # Return
    /// the value of key after the increment
    ///
    /// # See Also
    /// [https://redis.io/commands/incr/](https://redis.io/commands/incr/)
    fn incr<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("INCR").arg(key))
    }

    /// Increments the number stored at key by increment.
    ///
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type
    /// or contains a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// See [incr](crate::StringCommands::incr) for extra information on increment/decrement operations.
    ///
    /// # Return
    /// the value of key after the increment
    ///
    /// # See Also
    /// [https://redis.io/commands/incrby/](https://redis.io/commands/incrby/)
    fn incrby<K>(&self, key: K, increment: i64) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("INCRBY").arg(key).arg(increment))
    }

    ///Increment the string representing a floating point number stored at key by the specified increment.
    /// By using a negative increment value, the result is that the value stored at the key is decremented (by the obvious properties of addition).
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if one of the following conditions occur:
    ///
    /// - The key contains a value of the wrong type (not a string).
    ///
    /// - The current key content or the specified increment are not parsable as a double precision floating point number.
    ///
    /// If the command is successful the new incremented value is stored as the new value of the key (replacing the old one),
    /// and returned to the caller as a string.
    ///   
    /// Both the value already contained in the string key and the increment argument can be optionally provided in exponential notation,
    /// however the value computed after the increment is stored consistently in the same format, that is,
    /// an integer number followed (if needed) by a dot, and a variable number of digits representing the decimal part of the number.
    /// Trailing zeroes are always removed.
    ///    
    /// The precision of the output is fixed at 17 digits after the decimal point
    /// regardless of the actual internal precision of the computation.
    ///
    /// # Return
    /// the value of key after the increment
    ///
    /// # See Also
    /// [https://redis.io/commands/incrbyfloat/](https://redis.io/commands/incrbyfloat/)
    fn incrbyfloat<K>(
        &self,
        key: K,
        increment: f64,
    ) -> Pin<Box<dyn Future<Output = Result<f64>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("INCRBYFLOAT").arg(key).arg(increment))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # See Also
    /// [https://redis.io/commands/lcs/](https://redis.io/commands/lcs/)
    fn lcs<K>(&self, key1: K, key2: K) -> Lcs<Self>
    where
        K: Into<BulkString>,
    {
        Lcs {
            string_commands: &self,
            cmd: cmd("LCS").arg(key1).arg(key2),
        }
    }

    /// Returns the values of all specified keys.
    ///
    /// For every key that does not hold a string value or does not exist,
    /// the special value nil is returned. Because of this, the operation never fails.
    ///
    /// # Return
    /// Array reply: list of values at the specified keys.
    ///
    /// # See Also
    /// [https://redis.io/commands/mget/](https://redis.io/commands/mget/)
    fn mget<'a, K, V, C>(
        &'a self,
        keys: C,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Option<V>>>> + '_>>
    where
        K: Into<BulkString>,
        V: FromValue,
        C: SingleArgOrCollection<K>
    {
        self.send_into(cmd("MGET").arg(keys))
    }

    /// Sets the given keys to their respective values.
    ///
    /// # Return
    /// always OK since MSET can't fail.
    ///
    /// # See Also
    /// [https://redis.io/commands/mset/](https://redis.io/commands/mset/)
    fn mset<'a, K, V, C>(&'a self, items: C) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>
    where
        C: KeyValueArgOrCollection<K, V>,
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("MSET").arg(items))
    }

    /// Sets the given keys to their respective values.
    /// MSETNX will not perform any operation at all even if just a single key already exists.
    ///
    /// Because of this semantic MSETNX can be used in order to set different keys representing
    /// different fields of a unique logic object in a way that ensures that either
    /// all the fields or none at all are set.
    ///
    /// MSETNX is atomic, so all given keys are set at once. It is not possible for
    /// clients to see that some of the keys were updated while others are unchanged.
    ///
    /// # Return
    /// specifically:
    /// - 1 if the all the keys were set.
    /// - 0 if no key was set (at least one key already existed).
    ///
    /// # See Also
    /// [https://redis.io/commands/msetnx/](https://redis.io/commands/msetnx/)
    fn msetnx<'a, K, V, C>(&'a self, items: C) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        C: KeyValueArgOrCollection<K, V>,
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("MSETNX").arg(items))
    }

    /// Works exactly like [setex](crate::StringCommands::setex) with the sole
    /// difference that the expire time is specified in milliseconds instead of seconds.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/psetex/](https://redis.io/commands/psetex/)
    fn psetex<K, V>(
        &self,
        key: K,
        milliseconds: u64,
        value: V,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("PSETEX").arg(key).arg(milliseconds).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    fn set<K, V>(&self, key: K, value: V) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
        Self: Sized,
    {
        self.send_into(cmd("SET").arg(key).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    fn set_with_options<K, V>(&self, key: K, value: V) -> SetWithOptions<Self>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        SetWithOptions {
            string_commands: &self,
            cmd: cmd("SET").arg(key).arg(value),
        }
    }

    /// Set key to hold the string value and set key to timeout after a given number of seconds.
    ///
    /// # See Also
    /// [https://redis.io/commands/setex/](https://redis.io/commands/setex/)
    fn setex<K, V>(
        &self,
        key: K,
        seconds: u64,
        value: V,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("SETEX").arg(key).arg(seconds).arg(value))
    }

    /// Set key to hold string value if key does not exist.
    ///
    /// In that case, it is equal to SET.
    /// When key already holds a value, no operation is performed.
    /// SETNX is short for "SET if Not eXists".
    ///
    /// # Return
    /// specifically:
    /// * `true` - if the key was set
    /// * `false` - if the key was not set
    ///
    /// # See Also
    /// [https://redis.io/commands/setnx/](https://redis.io/commands/setnx/)
    fn setnx<K, V>(&self, key: K, value: V) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("SETNX").arg(key).arg(value))
    }

    /// Overwrites part of the string stored at key,
    /// starting at the specified offset,
    /// for the entire length of value.
    ///
    /// # Return
    /// the length of the string after it was modified by the command.
    ///
    /// # See Also
    /// [https://redis.io/commands/setrange/](https://redis.io/commands/setrange/)
    fn setrange<K, V>(
        &self,
        key: K,
        offset: usize,
        value: V,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.send_into(cmd("SETRANGE").arg(key).arg(offset).arg(value))
    }

    /// Returns the length of the string value stored at key.
    ///
    /// An error is returned when key holds a non-string value.
    ///
    /// # Return
    /// the length of the string at key, or 0 when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/strlen/](https://redis.io/commands/strlen/)
    fn strlen<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("STRLEN").arg(key))
    }
}

/// Builder for the [getex](crate::StringCommands::getex) command
pub struct GetEx<'a, T: StringCommands + ?Sized> {
    string_commands: &'a T,
    cmd: Command,
}

impl<'a, T: StringCommands> GetEx<'a, T> {
    /// Set the specified expire time, in seconds.
    pub fn ex<V>(self, seconds: u64) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        V: FromValue,
    {
        self.string_commands
            .send_into(self.cmd.arg("EX").arg(seconds))
    }

    /// Set the specified expire time, in milliseconds.
    pub fn px<V>(self, milliseconds: u64) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        V: FromValue,
    {
        self.string_commands
            .send_into(self.cmd.arg("PX").arg(milliseconds))
    }

    /// Set the specified Unix time at which the key will expire, in seconds.
    pub fn exat<V>(self, timestamp_seconds: u64) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        V: FromValue,
    {
        self.string_commands
            .send_into(self.cmd.arg("EXAT").arg(timestamp_seconds))
    }

    /// Set the specified Unix time at which the key will expire, in milliseconds.
    pub fn pxat<V>(
        self,
        timestamp_milliseconds: u64,
    ) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        V: FromValue,
    {
        self.string_commands
            .send_into(self.cmd.arg("PXAT").arg(timestamp_milliseconds))
    }

    /// Remove the time to live associated with the key.
    pub fn persist<V>(self) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        V: FromValue,
    {
        self.string_commands.send_into(self.cmd.arg("PERSIST"))
    }
}

/// Builder for the [lcs](crate::StringCommands::lcs) command
pub struct Lcs<'a, T: StringCommands + ?Sized> {
    string_commands: &'a T,
    cmd: Command,
}

impl<'a, T: StringCommands + ?Sized> Lcs<'a, T> {
    /// return the length of the match
    pub fn len(self) -> Pin<Box<dyn Future<Output = Result<usize>> + 'a>> {
        self.string_commands.send_into(self.cmd.arg("LEN"))
    }

    /// execute the command
    pub fn execute<V>(self) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        V: FromValue,
    {
        self.string_commands.send_into(self.cmd)
    }

    /// return the match position in each strings
    pub fn idx(self) -> LcsIdx<'a, T> {
        LcsIdx {
            string_commands: self.string_commands,
            cmd: self.cmd.arg("IDX"),
        }
    }
}

/// Builder for the [lcs](crate::StringCommands::lcs) command
pub struct LcsIdx<'a, T: StringCommands + ?Sized> {
    string_commands: &'a T,
    cmd: Command,
}

impl<'a, T: StringCommands + ?Sized> LcsIdx<'a, T> {
    /// restrict the list of matches to the ones of a given minimal length
    pub fn minmatchlen(self, len: usize) -> Self {
        LcsIdx {
            string_commands: self.string_commands,
            cmd: self.cmd.arg("MINMATCHLEN ").arg(len),
        }
    }

    /// also return the length of the match
    pub fn withmatchlen(self) -> Self {
        LcsIdx {
            string_commands: self.string_commands,
            cmd: self.cmd.arg("WITHMATCHLEN "),
        }
    }

    /// execute the command
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<LcsResult>> + 'a>> {
        self.string_commands.send_into(self.cmd)
    }
}

/// Result for the [lcs](crate::StringCommands::lcs) command
#[derive(Debug)]
pub struct LcsResult {
    pub matches: Vec<((usize, usize), (usize, usize))>,
    pub len: usize,
}

impl FromValue for LcsResult {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(mut result)) => {
                match (result.pop(), result.pop(), result.pop(), result.pop()) {
                    (
                        Some(Value::Integer(len)),
                        Some(Value::BulkString(BulkString::Binary(len_label))),
                        Some(Value::Array(Array::Vec(matches))),
                        Some(Value::BulkString(BulkString::Binary(matches_label))),
                    ) => {
                        if matches_label.as_slice() == b"matches" && len_label.as_slice() == b"len"
                        {
                            let matches: Vec<((usize, usize), (usize, usize))> = matches
                                .into_iter()
                                .map(|m| {
                                    let mut _match: Vec<Value> = m.into().unwrap();
                                    let pos2: Vec<usize> = _match.pop().unwrap().into().unwrap();
                                    let pos1: Vec<usize> = _match.pop().unwrap().into().unwrap();

                                    ((pos1[0], pos1[1]), (pos2[0], pos2[1]))
                                })
                                .collect();

                            return Ok(LcsResult {
                                matches,
                                len: len as usize,
                            });
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }

        Err(Error::Parse("Cannot parse result to LcsResult".to_string()))
    }
}

/// Builder for the [set_with_options](crate::StringCommands::set_with_options) command
pub struct SetWithOptions<'a, T: StringCommands + ?Sized> {
    string_commands: &'a T,
    cmd: Command,
}

impl<'a, T: StringCommands + ?Sized> SetWithOptions<'a, T> {
    /// Return the old string stored at key, or nil if key did not exist.
    ///
    /// An error is returned and SET aborted if the value stored at key is not a string.
    pub fn get(self) -> Self {
        Self {
            string_commands: self.string_commands,
            cmd: self.cmd.arg("GET"),
        }
    }

    /// Only set the key if it does not already exist.
    pub fn nx(self) -> Self {
        Self {
            string_commands: self.string_commands,
            cmd: self.cmd.arg("NX"),
        }
    }

    /// Only set the key if it already exist.
    pub fn xx(self) -> Self {
        Self {
            string_commands: self.string_commands,
            cmd: self.cmd.arg("XX"),
        }
    }

    /// execute the command
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.string_commands.send(self.cmd)
    }

    /// Set the specified expire time, in seconds.
    pub fn ex(self, seconds: u64) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.string_commands.send(self.cmd.arg("EX").arg(seconds))
    }

    /// Set the specified expire time, in milliseconds.
    pub fn px(self, milliseconds: u64) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.string_commands
            .send(self.cmd.arg("PX").arg(milliseconds))
    }

    /// Set the specified Unix time at which the key will expire, in seconds.
    pub fn exat(self, timestamp_seconds: u64) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.string_commands
            .send(self.cmd.arg("EXAT").arg(timestamp_seconds))
    }

    /// Set the specified Unix time at which the key will expire, in milliseconds.
    pub fn pxat(
        self,
        timestamp_milliseconds: u64,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.string_commands
            .send(self.cmd.arg("PXAT").arg(timestamp_milliseconds))
    }

    /// Set the specified Unix time at which the key will expire, in milliseconds.
    pub fn keepttl(self) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.string_commands.send(self.cmd.arg("KEEPTTL"))
    }
}
