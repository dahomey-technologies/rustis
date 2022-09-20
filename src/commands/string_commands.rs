use crate::{
    cmd,
    resp::{Array, BulkString, FromValue, Value},
    CommandArgs, CommandResult, Error, IntoArgs, IntoCommandResult, KeyValueArgOrCollection,
    Result, SingleArgOrCollection,
};

/// A group of Redis commands related to Strings
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=string)
pub trait StringCommands<T>: IntoCommandResult<T> {
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
    fn append<K, V>(&self, key: K, value: V) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("APPEND").arg(key).arg(value))
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
    fn decr<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("DECR").arg(key))
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
    fn decrby<K>(&self, key: K, decrement: i64) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("DECRBY").arg(key).arg(decrement))
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
    /// the value of key, or `nil` when key does not exist.
    /// 
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     cmd, ConnectionMultiplexer, DatabaseCommandResult, FlushingMode,
    ///     GetExOptions, ServerCommands, StringCommands, Result
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let connection = ConnectionMultiplexer::connect("127.0.0.1:6379").await?;
    ///     let database = connection.get_default_database();
    ///     database.flushdb(FlushingMode::Sync).send().await?;
    /// 
    ///     // return value can be an Option<String>...
    ///     let value: Option<String> = database.get("key").send().await?;
    ///     assert_eq!(None, value);
    /// 
    ///     // ... or it can be directly a String. 
    ///     // In this cas a `nil` value will result in an empty String
    ///     let value: String = database.get("key").send().await?;
    ///     assert_eq!("", &value);
    /// 
    ///     database.set("key", "value").send().await?;
    ///     let value: String = database.get("key").send().await?;
    ///     assert_eq!("value", value);
    /// 
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [https://redis.io/commands/get/](https://redis.io/commands/get/)
    fn get<K, V>(&self, key: K) -> CommandResult<T, V>
    where
        K: Into<BulkString>,
        V: FromValue,
        Self: Sized,
    {
        self.into_command_result(cmd("GET").arg(key))
    }

    /// Get the value of key and delete the key.
    ///
    /// This command is similar to GET, except for the fact that it also deletes the key on success
    /// (if and only if the key's value type is a string).
    ///
    /// # Return
    /// the value of key, `nil` when key does not exist, or an error if the key's value type isn't a string.
    ///
    /// # See Also
    /// [https://redis.io/commands/getdel/](https://redis.io/commands/getdel/)
    fn getdel<K, V>(&self, key: K) -> CommandResult<T, V>
    where
        K: Into<BulkString>,
        V: FromValue,
    {
        self.into_command_result(cmd("GETDEL").arg(key))
    }

    /// Get the value of key and optionally set its expiration. GETEX is similar to GET, but is a write command with additional options.
    ///
    /// Decrements the number stored at key by decrement.
    /// If the key does not exist, it is set to 0 before performing the operation.
    /// An error is returned if the key contains a value of the wrong type
    /// or contains a string that can not be represented as integer.
    /// This operation is limited to 64 bit signed integers.
    ///
    /// # Return
    /// the value of key, or `nil` when key does not exist.
    ///
    /// # Example
    /// ```
    /// use redis_driver::{
    ///     cmd, ConnectionMultiplexer, DatabaseCommandResult, FlushingMode,
    ///     GetExOptions, ServerCommands, StringCommands, Result
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let connection = ConnectionMultiplexer::connect("127.0.0.1:6379").await?;
    ///     let database = connection.get_default_database();
    ///     database.flushdb(FlushingMode::Sync).send().await?;
    /// 
    ///     database.set("key", "value").send().await?;
    ///     let value: String = database.getex("key", GetExOptions::Ex(1)).send().await?;
    ///     assert_eq!("value", value);
    /// 
    ///     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    /// 
    ///     let value: Option<String> = database.get("key").send().await?;
    ///     assert_eq!(None, value);
    /// 
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [https://redis.io/commands/getex/](https://redis.io/commands/getex/)
    fn getex<K, V>(&self, key: K, options: GetExOptions) -> CommandResult<T, V>
    where
        K: Into<BulkString>,
        V: FromValue,
    {
        self.into_command_result(cmd("GETEX").arg(key).arg(options))
    }

    /// Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).
    ///
    /// Negative offsets can be used in order to provide an offset starting from the end of the string.
    /// So -1 means the last character, -2 the penultimate and so forth.
    ///
    /// The function handles out of range requests by limiting the resulting range to the actual length of the string.

    /// # See Also
    /// [https://redis.io/commands/getrange/](https://redis.io/commands/getrange/)
    fn getrange<K, V>(&self, key: K, start: usize, end: isize) -> CommandResult<T, V>
    where
        K: Into<BulkString>,
        V: FromValue,
    {
        self.into_command_result(cmd("GETRANGE").arg(key).arg(start).arg(end))
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
    fn getset<K, V, R>(&self, key: K, value: V) -> CommandResult<T, R>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
        R: FromValue,
    {
        self.into_command_result(cmd("GETSET").arg(key).arg(value))
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
    fn incr<K>(&self, key: K) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("INCR").arg(key))
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
    fn incrby<K>(&self, key: K, increment: i64) -> CommandResult<T, i64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("INCRBY").arg(key).arg(increment))
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
    fn incrbyfloat<K>(&self, key: K, increment: f64) -> CommandResult<T, f64>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("INCRBYFLOAT").arg(key).arg(increment))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// The string representing the longest common substring.
    ///
    /// # See Also
    /// [https://redis.io/commands/lcs/](https://redis.io/commands/lcs/)
    fn lcs<K, V>(&self, key1: K, key2: K) -> CommandResult<T, V>
    where
        K: Into<BulkString>,
        V: FromValue,
    {
        self.into_command_result(cmd("LCS").arg(key1).arg(key2))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// The length of the longest common substring.
    ///
    /// # See Also
    /// [https://redis.io/commands/lcs/](https://redis.io/commands/lcs/)
    fn lcs_len<K>(&self, key1: K, key2: K) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("LCS").arg(key1).arg(key2).arg("LEN"))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// An array with the LCS length and all the ranges in both the strings,
    /// start and end offset for each string, where there are matches.
    /// When `with_match_len` is given each match will also have the length of the match
    ///
    /// # See Also
    /// [https://redis.io/commands/lcs/](https://redis.io/commands/lcs/)
    fn lcs_idx<K>(
        &self,
        key1: K,
        key2: K,
        min_match_len: Option<usize>,
        with_match_len: bool,
    ) -> CommandResult<T, LcsResult>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(
            cmd("LCS")
                .arg(key1)
                .arg(key2)
                .arg("IDX")
                .arg(min_match_len.map(|len| ("MINMATCHLEN", len)))
                .arg_if(with_match_len, "WITHMATCHLEN"),
        )
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
    fn mget<'a, K, V, C>(&'a self, keys: C) -> CommandResult<T, Vec<Option<V>>>
    where
        K: Into<BulkString>,
        V: FromValue,
        C: SingleArgOrCollection<K>,
    {
        self.into_command_result(cmd("MGET").arg(keys))
    }

    /// Sets the given keys to their respective values.
    ///
    /// # Return
    /// always OK since MSET can't fail.
    ///
    /// # See Also
    /// [https://redis.io/commands/mset/](https://redis.io/commands/mset/)
    fn mset<'a, K, V, C>(&'a self, items: C) -> CommandResult<T, ()>
    where
        C: KeyValueArgOrCollection<K, V>,
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("MSET").arg(items))
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
    fn msetnx<'a, K, V, C>(&'a self, items: C) -> CommandResult<T, bool>
    where
        C: KeyValueArgOrCollection<K, V>,
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("MSETNX").arg(items))
    }

    /// Works exactly like [setex](crate::StringCommands::setex) with the sole
    /// difference that the expire time is specified in milliseconds instead of seconds.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/psetex/](https://redis.io/commands/psetex/)
    fn psetex<K, V>(&self, key: K, milliseconds: u64, value: V) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("PSETEX").arg(key).arg(milliseconds).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    fn set<K, V>(&self, key: K, value: V) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
        Self: Sized,
    {
        self.into_command_result(cmd("SET").arg(key).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// # Return
    /// * `true` if SET was executed correctly.
    /// * `false` if the SET operation was not performed because the user
    ///  specified the NX or XX option but the condition was not met.
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    fn set_with_options<K, V>(
        &self,
        key: K,
        value: V,
        condition: Option<SetCondition>,
        expiration: Option<SetExpiration>,
        keep_ttl: bool,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(
            cmd("SET")
                .arg(key)
                .arg(value)
                .arg(condition)
                .arg(expiration)
                .arg_if(keep_ttl, "KEEPTTL"),
        )
    }

    /// Set key to hold the string value wit GET option enforced
    ///
    /// # See Also
    /// [https://redis.io/commands/set/](https://redis.io/commands/set/)
    fn set_get_with_options<K, V1, V2>(
        &self,
        key: K,
        value: V1,
        condition: Option<SetCondition>,
        expiration: Option<SetExpiration>,
        keep_ttl: bool,
    ) -> CommandResult<T, V2>
    where
        K: Into<BulkString>,
        V1: Into<BulkString>,
        V2: FromValue,
    {
        self.into_command_result(
            cmd("SET")
                .arg(key)
                .arg(value)
                .arg(condition)
                .arg("GET")
                .arg(expiration)
                .arg_if(keep_ttl, "KEEPTTL"),
        )
    }

    /// Set key to hold the string value and set key to timeout after a given number of seconds.
    ///
    /// # See Also
    /// [https://redis.io/commands/setex/](https://redis.io/commands/setex/)
    fn setex<K, V>(&self, key: K, seconds: u64, value: V) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("SETEX").arg(key).arg(seconds).arg(value))
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
    fn setnx<K, V>(&self, key: K, value: V) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("SETNX").arg(key).arg(value))
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
    fn setrange<K, V>(&self, key: K, offset: usize, value: V) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        V: Into<BulkString>,
    {
        self.into_command_result(cmd("SETRANGE").arg(key).arg(offset).arg(value))
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
    fn strlen<K>(&self, key: K) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.into_command_result(cmd("STRLEN").arg(key))
    }
}

/// Options for the [getex](crate::StringCommands::getex) command
pub enum GetExOptions {
    /// Set the specified expire time, in seconds.
    Ex(u64),
    /// Set the specified expire time, in milliseconds.
    Px(u64),
    /// Set the specified Unix time at which the key will expire, in seconds.
    Exat(u64),
    /// Set the specified Unix time at which the key will expire, in milliseconds.
    Pxat(u64),
    /// Remove the time to live associated with the key.
    Persist,
}

impl IntoArgs for GetExOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            GetExOptions::Ex(duration) => ("EX", duration).into_args(args),
            GetExOptions::Px(duration) => ("PX", duration).into_args(args),
            GetExOptions::Exat(timestamp) => ("EXAT", timestamp).into_args(args),
            GetExOptions::Pxat(timestamp) => ("PXAT", timestamp).into_args(args),
            GetExOptions::Persist => "PERSIST".into_args(args),
        }
    }
}

/// Result for the [lcs](crate::StringCommands::lcs) command
#[derive(Debug)]
pub struct LcsResult {
    pub matches: Vec<((usize, usize), (usize, usize), Option<usize>)>,
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
                            let matches: Result<
                                Vec<((usize, usize), (usize, usize), Option<usize>)>,
                            > = matches
                                .into_iter()
                                .map(|m| {
                                    let mut _match: Vec<Value> = m.into()?;

                                    match (_match.pop(), _match.pop(), _match.pop(), _match.pop()) {
                                        (Some(len), Some(pos2), Some(pos1), None) => {
                                            let mut pos1: Vec<usize> = pos1.into()?;
                                            let mut pos2: Vec<usize> = pos2.into()?;
                                            let len: usize = len.into()?;

                                            match (pos1.pop(), pos1.pop(), pos1.pop()) {
                                                (Some(pos1_right), Some(pos1_left), None) => {
                                                    match (pos2.pop(), pos2.pop(), pos2.pop()) {
                                                        (
                                                            Some(pos2_right),
                                                            Some(pos2_left),
                                                            None,
                                                        ) => Ok((
                                                            (pos1_left, pos1_right),
                                                            (pos2_left, pos2_right),
                                                            Some(len),
                                                        )),
                                                        _ => Err(Error::Internal(
                                                            "Cannot parse LCS result".to_owned(),
                                                        )),
                                                    }
                                                }
                                                _ => Err(Error::Internal(
                                                    "Cannot parse LCS result".to_owned(),
                                                )),
                                            }
                                        }
                                        (Some(pos2), Some(pos1), None, None) => {
                                            let mut pos1: Vec<usize> = pos1.into()?;
                                            let mut pos2: Vec<usize> = pos2.into()?;

                                            match (pos1.pop(), pos1.pop(), pos1.pop()) {
                                                (Some(pos1_right), Some(pos1_left), None) => {
                                                    match (pos2.pop(), pos2.pop(), pos2.pop()) {
                                                        (
                                                            Some(pos2_right),
                                                            Some(pos2_left),
                                                            None,
                                                        ) => Ok((
                                                            (pos1_left, pos1_right),
                                                            (pos2_left, pos2_right),
                                                            None,
                                                        )),
                                                        _ => Err(Error::Internal(
                                                            "Cannot parse LCS result".to_owned(),
                                                        )),
                                                    }
                                                }
                                                _ => Err(Error::Internal(
                                                    "Cannot parse LCS result".to_owned(),
                                                )),
                                            }
                                        }
                                        _ => Err(Error::Internal(
                                            "Cannot parse LCS result".to_owned(),
                                        )),
                                    }
                                })
                                .collect();

                            return Ok(LcsResult {
                                matches: matches?,
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

/// Expiration option for the [set_with_options](crate::StringCommands::set_with_options) command
pub enum SetExpiration {
    /// Set the specified expire time, in seconds.
    Ex(u64),
    /// Set the specified expire time, in milliseconds.
    Px(u64),
    /// Set the specified Unix time at which the key will expire, in seconds.
    Exat(u64),
    /// Set the specified Unix time at which the key will expire, in milliseconds.
    Pxat(u64),
}

impl IntoArgs for SetExpiration {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            SetExpiration::Ex(duration) => ("EX", duration).into_args(args),
            SetExpiration::Px(duration) => ("PX", duration).into_args(args),
            SetExpiration::Exat(timestamp) => ("EXAT", timestamp).into_args(args),
            SetExpiration::Pxat(timestamp) => ("PXAT", timestamp).into_args(args),
        }
    }
}

/// Condition option for the [set_with_options](crate::StringCommands::set_with_options) command
pub enum SetCondition {
    /// Only set the key if it does not already exist.
    NX,
    /// Only set the key if it already exist.
    XX,
}

impl From<SetCondition> for BulkString {
    fn from(cond: SetCondition) -> Self {
        match cond {
            SetCondition::NX => BulkString::Str("NX"),
            SetCondition::XX => BulkString::Str("XX"),
        }
    }
}
