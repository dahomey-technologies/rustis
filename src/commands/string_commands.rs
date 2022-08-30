use crate::{
    cmd,
    resp::{Array, BulkString, FromValue, ResultValueExt, Value},
    Command, Database, Error, IntoArgs, Result,
};
use async_trait::async_trait;
use std::iter::once;

/// A group of Redis commands related to Strings
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=string)
#[async_trait]
pub trait StringCommands {
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
    async fn append<K, V>(&self, key: K, value: V) -> Result<usize>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

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
    async fn decr<K>(&self, key: K) -> Result<i64>
    where
        K: Into<BulkString> + Send;

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
    async fn decrby<K>(&self, key: K, decrement: i64) -> Result<i64>
    where
        K: Into<BulkString> + Send;

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
    async fn get<K, V>(&self, key: K) -> Result<V>
    where
        K: Into<BulkString> + Send,
        V: FromValue;

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
    async fn getdel<K, V>(&self, key: K) -> Result<V>
    where
        K: Into<BulkString> + Send,
        V: FromValue;

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
    fn getex<K>(&self, key: K) -> GetEx
    where
        K: Into<BulkString> + Send;

    /// Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).
    ///
    /// Negative offsets can be used in order to provide an offset starting from the end of the string.
    /// So -1 means the last character, -2 the penultimate and so forth.
    ///
    /// The function handles out of range requests by limiting the resulting range to the actual length of the string.

    /// # See Also
    /// [https://redis.io/commands/getrange/](https://redis.io/commands/getrange/)
    async fn getrange<K, V>(&self, key: K, start: usize, end: isize) -> Result<V>
    where
        K: Into<BulkString> + Send,
        V: FromValue;

    /// Atomically sets key to value and returns the old value stored at key.
    /// Returns an error when key exists but does not hold a string value.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # Return
    /// the old value stored at key, or nil when key did not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/getset/](https://redis.io/commands/getset/)
    async fn getset<K, V, R>(&self, key: K, value: V) -> Result<R>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
        R: FromValue;

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
    async fn incr<K>(&self, key: K) -> Result<i64>
    where
        K: Into<BulkString> + Send;

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
    async fn incrby<K>(&self, key: K, increment: i64) -> Result<i64>
    where
        K: Into<BulkString> + Send;

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
    async fn incrbyfloat<K>(&self, key: K, increment: f64) -> Result<f64>
    where
        K: Into<BulkString> + Send;

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # See Also
    /// [https://redis.io/commands/lcs/](https://redis.io/commands/lcs/)
    fn lcs<K>(&self, key1: K, key2: K) -> Lcs
    where
        K: Into<BulkString> + Send;

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
    async fn mget<'a, K, V>(&'a self, keys: K) -> Result<Vec<Option<V>>>
    where
        K: IntoArgs + Send + Sync,
        V: FromValue;

    /// Sets the given keys to their respective values.
    ///
    /// # Return
    /// always OK since MSET can't fail.
    ///
    /// # See Also
    /// [https://redis.io/commands/mset/](https://redis.io/commands/mset/)
    async fn mset<'a, K, V>(&'a self, items: &'a [(K, V)]) -> Result<()>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        V: Into<BulkString> + Send + Sync + Copy;

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
    async fn msetnx<'a, K, V>(&'a self, items: &'a [(K, V)]) -> Result<bool>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        V: Into<BulkString> + Send + Sync + Copy;

    /// Works exactly like [setex](crate::StringCommands::setex) with the sole
    /// difference that the expire time is specified in milliseconds instead of seconds.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/psetex/](https://redis.io/commands/psetex/)
    async fn psetex<K, V>(&self, key: K, milliseconds: u64, value: V) -> Result<()>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    async fn set<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    fn set_with_options<K, V>(&self, key: K, value: V) -> SetWithOptions
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

    /// Set key to hold the string value and set key to timeout after a given number of seconds.
    ///
    /// # See Also
    /// [https://redis.io/commands/setex/](https://redis.io/commands/setex/)
    async fn setex<K, V>(&self, key: K, seconds: u64, value: V) -> Result<()>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

    /// Set key to hold string value if key does not exist.
    ///
    /// In that case, it is equal to SET.
    /// When key already holds a value, no operation is performed.
    /// SETNX is short for "SET if Not eXists".
    ///
    /// # Return
    /// specifically:
    /// - true if the key was set
    /// - false if the key was not set
    ///
    /// # See Also
    /// [https://redis.io/commands/setnx/](https://redis.io/commands/setnx/)
    async fn setnx<K, V>(&self, key: K, value: V) -> Result<bool>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

    /// Overwrites part of the string stored at key,
    /// starting at the specified offset,
    /// for the entire length of value.
    ///
    /// # Return
    /// the length of the string after it was modified by the command.
    ///
    /// # See Also
    /// [https://redis.io/commands/setrange/](https://redis.io/commands/setrange/)
    async fn setrange<K, V>(&self, key: K, offset: usize, value: V) -> Result<usize>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send;

    /// Returns the length of the string value stored at key.
    ///
    /// An error is returned when key holds a non-string value.
    ///
    /// # Return
    /// the length of the string at key, or 0 when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/strlen/](https://redis.io/commands/strlen/)
    async fn strlen<K>(&self, key: K) -> Result<usize>
    where
        K: Into<BulkString> + Send;
}

#[async_trait]
impl StringCommands for Database {
    async fn append<K, V>(&self, key: K, value: V) -> Result<usize>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        self.send(cmd("APPEND").arg(key).arg(value)).await?.into()
    }

    async fn decr<K>(&self, key: K) -> Result<i64>
    where
        K: Into<BulkString> + Send,
    {
        self.send(cmd("DECR").arg(key)).await?.into()
    }

    async fn decrby<K>(&self, key: K, decrement: i64) -> Result<i64>
    where
        K: Into<BulkString> + Send,
    {
        self.send(cmd("DECRBY").arg(key).arg(decrement))
            .await?
            .into()
    }

    async fn get<K, V>(&self, key: K) -> Result<V>
    where
        K: Into<BulkString> + Send,
        V: FromValue,
    {
        self.send(cmd("GET").arg(key)).await?.into()
    }

    async fn getdel<K, V>(&self, key: K) -> Result<V>
    where
        K: Into<BulkString> + Send,
        V: FromValue,
    {
        self.send(cmd("GETDEL").arg(key)).await?.into()
    }

    fn getex<K>(&self, key: K) -> GetEx
    where
        K: Into<BulkString> + Send,
    {
        GetEx {
            database: &self,
            cmd: cmd("GETEX").arg(key),
        }
    }

    async fn getrange<K, V>(&self, key: K, start: usize, end: isize) -> Result<V>
    where
        K: Into<BulkString> + Send,
        V: FromValue,
    {
        self.send(cmd("GETRANGE").arg(key).arg(start).arg(end))
            .await?
            .into()
    }

    async fn getset<K, V, R>(&self, key: K, value: V) -> Result<R>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
        R: FromValue,
    {
        self.send(cmd("GETSET").arg(key).arg(value)).await?.into()
    }

    async fn incr<K>(&self, key: K) -> Result<i64>
    where
        K: Into<BulkString> + Send,
    {
        self.send(cmd("INCR").arg(key)).await?.into()
    }

    async fn incrby<K>(&self, key: K, increment: i64) -> Result<i64>
    where
        K: Into<BulkString> + Send,
    {
        self.send(cmd("INCRBY").arg(key).arg(increment))
            .await?
            .into()
    }

    async fn incrbyfloat<K>(&self, key: K, increment: f64) -> Result<f64>
    where
        K: Into<BulkString> + Send,
    {
        self.send(cmd("INCRBYFLOAT").arg(key).arg(increment))
            .await?
            .into()
    }

    fn lcs<K>(&self, key1: K, key2: K) -> Lcs
    where
        K: Into<BulkString> + Send,
    {
        Lcs {
            database: &self,
            cmd: cmd("LCS").arg(key1).arg(key2),
        }
    }

    async fn mget<'a, K, V>(&'a self, keys: K) -> Result<Vec<Option<V>>>
    where
        K: IntoArgs + Send + Sync,
        V: FromValue,
    {
        self.send(cmd("MGET").args(keys)).await?.into()
    }

    async fn mset<'a, K, V>(&'a self, items: &'a [(K, V)]) -> Result<()>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        V: Into<BulkString> + Send + Sync + Copy,
    {
        let flatten_items: Vec<BulkString> = items
            .iter()
            .flat_map(|i| once(i.0.into()).chain(once(i.1.into())))
            .collect();
        self.send(cmd("MSET").args(flatten_items)).await.into_unit()
    }

    async fn msetnx<'a, K, V>(&'a self, items: &'a [(K, V)]) -> Result<bool>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        V: Into<BulkString> + Send + Sync + Copy,
    {
        let flatten_items: Vec<BulkString> = items
            .iter()
            .flat_map(|i| once(i.0.into()).chain(once(i.1.into())))
            .collect();
        self.send(cmd("MSETNX").args(flatten_items)).await?.into()
    }

    async fn psetex<K, V>(&self, key: K, milliseconds: u64, value: V) -> Result<()>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        self.send(cmd("PSETEX").arg(key).arg(milliseconds).arg(value))
            .await
            .into_unit()
    }

    async fn set<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        self.send(cmd("SET").arg(key).arg(value)).await.into_unit()
    }

    fn set_with_options<K, V>(&self, key: K, value: V) -> SetWithOptions
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        SetWithOptions {
            database: &self,
            cmd: cmd("SET").arg(key).arg(value),
        }
    }

    async fn setex<K, V>(&self, key: K, seconds: u64, value: V) -> Result<()>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        self.send(cmd("PSETEX").arg(key).arg(seconds).arg(value))
            .await
            .into_unit()
    }

    async fn setnx<K, V>(&self, key: K, value: V) -> Result<bool>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        self.send(cmd("SETNX").arg(key).arg(value)).await?.into()
    }

    async fn setrange<K, V>(&self, key: K, offset: usize, value: V) -> Result<usize>
    where
        K: Into<BulkString> + Send,
        V: Into<BulkString> + Send,
    {
        self.send(cmd("SETRANGE").arg(key).arg(offset).arg(value))
            .await?
            .into()
    }

    async fn strlen<K>(&self, key: K) -> Result<usize>
    where
        K: Into<BulkString> + Send,
    {
        self.send(cmd("STRLEN").arg(key)).await?.into()
    }
}

/// Builder for the [getex](crate::StringCommands::getex) command
pub struct GetEx<'a> {
    database: &'a Database,
    cmd: Command,
}

impl<'a> GetEx<'a> {
    /// Set the specified expire time, in seconds.
    pub async fn ex<V>(self, seconds: u64) -> Result<V>
    where
        V: FromValue,
    {
        self.database
            .send(self.cmd.arg("EX").arg(seconds))
            .await?
            .into()
    }

    /// Set the specified expire time, in milliseconds.
    pub async fn px<V>(self, milliseconds: u64) -> Result<V>
    where
        V: FromValue,
    {
        self.database
            .send(self.cmd.arg("PX").arg(milliseconds))
            .await?
            .into()
    }

    /// Set the specified Unix time at which the key will expire, in seconds.
    pub async fn exat<V>(self, timestamp_seconds: u64) -> Result<V>
    where
        V: FromValue,
    {
        self.database
            .send(self.cmd.arg("EXAT").arg(timestamp_seconds))
            .await?
            .into()
    }

    /// Set the specified Unix time at which the key will expire, in milliseconds.
    pub async fn pxat<V>(self, timestamp_milliseconds: u64) -> Result<V>
    where
        V: FromValue,
    {
        self.database
            .send(self.cmd.arg("PXAT").arg(timestamp_milliseconds))
            .await?
            .into()
    }

    /// Remove the time to live associated with the key.
    pub async fn persist<V>(self) -> Result<V>
    where
        V: FromValue,
    {
        self.database.send(self.cmd.arg("PERSIST")).await?.into()
    }
}

/// Builder for the [lcs](crate::StringCommands::lcs) command
pub struct Lcs<'a> {
    database: &'a Database,
    cmd: Command,
}

impl<'a> Lcs<'a> {
    /// return the length of the match
    pub async fn len(self) -> Result<usize> {
        self.database.send(self.cmd.arg("LEN")).await?.into()
    }

    /// execute the command
    pub async fn execute<V>(self) -> Result<V>
    where
        V: FromValue,
    {
        self.database.send(self.cmd).await?.into()
    }

    /// return the match position in each strings
    pub fn idx(self) -> LcsIdx<'a> {
        LcsIdx {
            database: self.database,
            cmd: self.cmd.arg("IDX"),
        }
    }
}

/// Builder for the [lcs](crate::StringCommands::lcs) command
pub struct LcsIdx<'a> {
    database: &'a Database,
    cmd: Command,
}

impl<'a> LcsIdx<'a> {
    /// restrict the list of matches to the ones of a given minimal length
    pub fn minmatchlen(self, len: usize) -> LcsIdx<'a> {
        LcsIdx {
            database: self.database,
            cmd: self.cmd.arg("MINMATCHLEN ").arg(len),
        }
    }

    /// also return the length of the match
    pub fn withmatchlen(self) -> LcsIdx<'a> {
        LcsIdx {
            database: self.database,
            cmd: self.cmd.arg("WITHMATCHLEN "),
        }
    }

    /// execute the command
    pub async fn execute(self) -> Result<LcsResult> {
        self.database.send(self.cmd).await?.into()
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
pub struct SetWithOptions<'a> {
    database: &'a Database,
    cmd: Command,
}

impl<'a> SetWithOptions<'a> {
    /// Return the old string stored at key, or nil if key did not exist.
    ///
    /// An error is returned and SET aborted if the value stored at key is not a string.
    pub fn get(self) -> SetWithOptions<'a> {
        Self {
            database: self.database,
            cmd: self.cmd.arg("GET"),
        }
    }

    /// Only set the key if it does not already exist.
    pub fn nx(self) -> SetWithOptions<'a> {
        Self {
            database: self.database,
            cmd: self.cmd.arg("NX"),
        }
    }

    /// Only set the key if it already exist.
    pub fn xx(self) -> SetWithOptions<'a> {
        Self {
            database: self.database,
            cmd: self.cmd.arg("XX"),
        }
    }

    /// execute the command
    pub async fn execute(self) -> Result<Value> {
        self.database.send(self.cmd).await
    }

    /// Set the specified expire time, in seconds.
    pub async fn ex(self, seconds: u64) -> Result<Value> {
        self.database.send(self.cmd.arg("EX").arg(seconds)).await
    }

    /// Set the specified expire time, in milliseconds.
    pub async fn px(self, milliseconds: u64) -> Result<Value> {
        self.database
            .send(self.cmd.arg("PX").arg(milliseconds))
            .await
    }

    /// Set the specified Unix time at which the key will expire, in seconds.
    pub async fn exat(self, timestamp_seconds: u64) -> Result<Value> {
        self.database
            .send(self.cmd.arg("EXAT").arg(timestamp_seconds))
            .await
    }

    /// Set the specified Unix time at which the key will expire, in milliseconds.
    pub async fn pxat(self, timestamp_milliseconds: u64) -> Result<Value> {
        self.database
            .send(self.cmd.arg("PXAT").arg(timestamp_milliseconds))
            .await
    }

    /// Set the specified Unix time at which the key will expire, in milliseconds.
    pub async fn keepttl(self) -> Result<Value> {
        self.database.send(self.cmd.arg("KEEPTTL")).await
    }
}
