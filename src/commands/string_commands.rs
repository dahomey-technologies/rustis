use std::collections::HashMap;

use crate::{
    prepare_command,
    resp::{
        cmd, CommandArg, CommandArgs, FromSingleValueArray, FromValue, HashMapExt, IntoArgs,
        KeyValueArgOrCollection, SingleArgOrCollection, Value,
    },
    Error, PreparedCommand, Result,
};

/// A group of Redis commands related to [`Strings`](https://redis.io/docs/data-types/strings/)
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=string)
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
    /// [<https://redis.io/commands/append/>](https://redis.io/commands/append/)
    #[must_use]
    fn append<K, V>(&mut self, key: K, value: V) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("APPEND").arg(key).arg(value))
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
    /// [<https://redis.io/commands/decr/>](https://redis.io/commands/decr/)
    #[must_use]
    fn decr<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("DECR").arg(key))
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
    /// [<https://redis.io/commands/decrby/>](https://redis.io/commands/decrby/)
    #[must_use]
    fn decrby<K>(&mut self, key: K, decrement: i64) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("DECRBY").arg(key).arg(decrement))
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
    /// use rustis::{
    ///     resp::{cmd}, Client, ClientPreparedCommand, FlushingMode,
    ///     ServerCommands, StringCommands, Result
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut client = Client::connect("127.0.0.1:6379").await?;
    ///     client.flushdb(FlushingMode::Sync).await?;
    ///
    ///     // return value can be an Option<String>...
    ///     let value: Option<String> = client.get("key").await?;
    ///     assert_eq!(None, value);
    ///
    ///     // ... or it can be directly a String.
    ///     // In this cas a `nil` value will result in an empty String
    ///     let value: String = client.get("key").await?;
    ///     assert_eq!("", &value);
    ///
    ///     client.set("key", "value").await?;
    ///     let value: String = client.get("key").await?;
    ///     assert_eq!("value", value);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/get/>](https://redis.io/commands/get/)
    #[must_use]
    fn get<K, V>(&mut self, key: K) -> PreparedCommand<Self, V>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: FromValue,
        Self: Sized,
    {
        prepare_command(self, cmd("GET").arg(key))
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
    /// [<https://redis.io/commands/getdel/>](https://redis.io/commands/getdel/)
    #[must_use]
    fn getdel<K, V>(&mut self, key: K) -> PreparedCommand<Self, V>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: FromValue,
    {
        prepare_command(self, cmd("GETDEL").arg(key))
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
    /// use rustis::{
    ///     resp::cmd, Client, ClientPreparedCommand, FlushingMode,
    ///     GetExOptions, GenericCommands, ServerCommands, StringCommands, Result
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let mut client = Client::connect("127.0.0.1:6379").await?;
    ///     client.flushdb(FlushingMode::Sync).await?;
    ///
    ///     client.set("key", "value").await?;
    ///     let value: String = client.getex("key", GetExOptions::Ex(60)).await?;
    ///     assert_eq!("value", value);
    ///
    ///     let ttl = client.ttl("key").await?;
    ///     assert!(59 <= ttl && ttl <= 60);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/getex/>](https://redis.io/commands/getex/)
    #[must_use]
    fn getex<K, V>(&mut self, key: K, options: GetExOptions) -> PreparedCommand<Self, V>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: FromValue,
    {
        prepare_command(self, cmd("GETEX").arg(key).arg(options))
    }

    /// Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).
    ///
    /// Negative offsets can be used in order to provide an offset starting from the end of the string.
    /// So -1 means the last character, -2 the penultimate and so forth.
    ///
    /// The function handles out of range requests by limiting the resulting range to the actual length of the string.

    /// # See Also
    /// [<https://redis.io/commands/getrange/>](https://redis.io/commands/getrange/)
    #[must_use]
    fn getrange<K, V>(&mut self, key: K, start: usize, end: isize) -> PreparedCommand<Self, V>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: FromValue,
    {
        prepare_command(self, cmd("GETRANGE").arg(key).arg(start).arg(end))
    }

    /// Atomically sets key to value and returns the old value stored at key.
    /// Returns an error when key exists but does not hold a string value.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # Return
    /// the old value stored at key, or nil when key did not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/getset/>](https://redis.io/commands/getset/)
    #[must_use]
    fn getset<K, V, R>(&mut self, key: K, value: V) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
        R: FromValue,
    {
        prepare_command(self, cmd("GETSET").arg(key).arg(value))
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
    /// [<https://redis.io/commands/incr/>](https://redis.io/commands/incr/)
    #[must_use]
    fn incr<K>(&mut self, key: K) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("INCR").arg(key))
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
    /// [<https://redis.io/commands/incrby/>](https://redis.io/commands/incrby/)
    #[must_use]
    fn incrby<K>(&mut self, key: K, increment: i64) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("INCRBY").arg(key).arg(increment))
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
    /// [<https://redis.io/commands/incrbyfloat/>](https://redis.io/commands/incrbyfloat/)
    #[must_use]
    fn incrbyfloat<K>(&mut self, key: K, increment: f64) -> PreparedCommand<Self, f64>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("INCRBYFLOAT").arg(key).arg(increment))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// The string representing the longest common substring.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lcs/>](https://redis.io/commands/lcs/)
    #[must_use]
    fn lcs<K, V>(&mut self, key1: K, key2: K) -> PreparedCommand<Self, V>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: FromValue,
    {
        prepare_command(self, cmd("LCS").arg(key1).arg(key2))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// The length of the longest common substring.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lcs/>](https://redis.io/commands/lcs/)
    #[must_use]
    fn lcs_len<K>(&mut self, key1: K, key2: K) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("LCS").arg(key1).arg(key2).arg("LEN"))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// An array with the LCS length and all the ranges in both the strings,
    /// start and end offset for each string, where there are matches.
    /// When `with_match_len` is given each match will also have the length of the match
    ///
    /// # See Also
    /// [<https://redis.io/commands/lcs/>](https://redis.io/commands/lcs/)
    #[must_use]
    fn lcs_idx<K>(
        &mut self,
        key1: K,
        key2: K,
        min_match_len: Option<usize>,
        with_match_len: bool,
    ) -> PreparedCommand<Self, LcsResult>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(
            self,
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
    /// [<https://redis.io/commands/mget/>](https://redis.io/commands/mget/)
    #[must_use]
    fn mget<K, KK, V, VV>(&mut self, keys: KK) -> PreparedCommand<Self, VV>
    where
        Self: Sized,
        K: Into<CommandArg>,
        KK: SingleArgOrCollection<K>,
        V: FromValue,
        VV: FromSingleValueArray<V>,
    {
        prepare_command(self, cmd("MGET").arg(keys))
    }

    /// Sets the given keys to their respective values.
    ///
    /// # Return
    /// always OK since MSET can't fail.
    ///
    /// # See Also
    /// [<https://redis.io/commands/mset/>](https://redis.io/commands/mset/)
    #[must_use]
    fn mset<K, V, C>(&mut self, items: C) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        C: KeyValueArgOrCollection<K, V>,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("MSET").arg(items))
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
    /// [<https://redis.io/commands/msetnx/>](https://redis.io/commands/msetnx/)
    #[must_use]
    fn msetnx<K, V, C>(&mut self, items: C) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        C: KeyValueArgOrCollection<K, V>,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("MSETNX").arg(items))
    }

    /// Works exactly like [setex](crate::StringCommands::setex) with the sole
    /// difference that the expire time is specified in milliseconds instead of seconds.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/psetex/>](https://redis.io/commands/psetex/)
    #[must_use]
    fn psetex<K, V>(&mut self, key: K, milliseconds: u64, value: V) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("PSETEX").arg(key).arg(milliseconds).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set<K, V>(&mut self, key: K, value: V) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
        Self: Sized,
    {
        prepare_command(self, cmd("SET").arg(key).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// # Return
    /// * `true` if SET was executed correctly.
    /// * `false` if the SET operation was not performed because the user
    ///  specified the NX or XX option but the condition was not met.
    ///
    /// # See Also
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set_with_options<K, V>(
        &mut self,
        key: K,
        value: V,
        condition: SetCondition,
        expiration: SetExpiration,
        keep_ttl: bool,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(
            self,
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
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set_get_with_options<K, V1, V2>(
        &mut self,
        key: K,
        value: V1,
        condition: SetCondition,
        expiration: SetExpiration,
        keep_ttl: bool,
    ) -> PreparedCommand<Self, V2>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V1: Into<CommandArg>,
        V2: FromValue,
    {
        prepare_command(
            self,
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
    /// [<https://redis.io/commands/setex/>](https://redis.io/commands/setex/)
    #[must_use]
    fn setex<K, V>(&mut self, key: K, seconds: u64, value: V) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("SETEX").arg(key).arg(seconds).arg(value))
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
    /// [<https://redis.io/commands/setnx/>](https://redis.io/commands/setnx/)
    #[must_use]
    fn setnx<K, V>(&mut self, key: K, value: V) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("SETNX").arg(key).arg(value))
    }

    /// Overwrites part of the string stored at key,
    /// starting at the specified offset,
    /// for the entire length of value.
    ///
    /// # Return
    /// the length of the string after it was modified by the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/setrange/>](https://redis.io/commands/setrange/)
    #[must_use]
    fn setrange<K, V>(&mut self, key: K, offset: usize, value: V) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
        V: Into<CommandArg>,
    {
        prepare_command(self, cmd("SETRANGE").arg(key).arg(offset).arg(value))
    }

    /// Returns the length of the string value stored at key.
    ///
    /// An error is returned when key holds a non-string value.
    ///
    /// # Return
    /// the length of the string at key, or 0 when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/strlen/>](https://redis.io/commands/strlen/)
    #[must_use]
    fn strlen<K>(&mut self, key: K) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<CommandArg>,
    {
        prepare_command(self, cmd("STRLEN").arg(key))
    }
}

/// Options for the [`getex`](crate::StringCommands::getex) command
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

pub type LcsMatch = ((usize, usize), (usize, usize), Option<usize>);

/// Result for the [`lcs`](crate::StringCommands::lcs) command
#[derive(Debug)]
pub struct LcsResult {
    pub matches: Vec<LcsMatch>,
    pub len: usize,
}

impl FromValue for LcsResult {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        let matches: Result<Vec<LcsMatch>> = values
            .remove_with_result("matches")?
            .into::<Vec<Value>>()?
            .into_iter()
            .map(|m| {
                let mut match_: Vec<Value> = m.into()?;

                match (match_.pop(), match_.pop(), match_.pop(), match_.pop()) {
                    (Some(len), Some(pos2), Some(pos1), None) => {
                        let mut pos1: Vec<usize> = pos1.into()?;
                        let mut pos2: Vec<usize> = pos2.into()?;
                        let len: usize = len.into()?;

                        match (pos1.pop(), pos1.pop(), pos1.pop()) {
                            (Some(pos1_right), Some(pos1_left), None) => {
                                match (pos2.pop(), pos2.pop(), pos2.pop()) {
                                    (Some(pos2_right), Some(pos2_left), None) => Ok((
                                        (pos1_left, pos1_right),
                                        (pos2_left, pos2_right),
                                        Some(len),
                                    )),
                                    _ => Err(Error::Client("Cannot parse LCS result".to_owned())),
                                }
                            }
                            _ => Err(Error::Client("Cannot parse LCS result".to_owned())),
                        }
                    }
                    (Some(pos2), Some(pos1), None, None) => {
                        let mut pos1: Vec<usize> = pos1.into()?;
                        let mut pos2: Vec<usize> = pos2.into()?;

                        match (pos1.pop(), pos1.pop(), pos1.pop()) {
                            (Some(pos1_right), Some(pos1_left), None) => {
                                match (pos2.pop(), pos2.pop(), pos2.pop()) {
                                    (Some(pos2_right), Some(pos2_left), None) => {
                                        Ok(((pos1_left, pos1_right), (pos2_left, pos2_right), None))
                                    }
                                    _ => Err(Error::Client("Cannot parse LCS result".to_owned())),
                                }
                            }
                            _ => Err(Error::Client("Cannot parse LCS result".to_owned())),
                        }
                    }
                    _ => Err(Error::Client("Cannot parse LCS result".to_owned())),
                }
            })
            .collect();

        Ok(Self {
            matches: matches?,
            len: values.remove_with_result("len")?.into()?,
        })
    }
}

/// Expiration option for the [`set_with_options`](crate::StringCommands::set_with_options) command
pub enum SetExpiration {
    /// No expiration
    None,
    /// Set the specified expire time, in seconds.
    Ex(u64),
    /// Set the specified expire time, in milliseconds.
    Px(u64),
    /// Set the specified Unix time at which the key will expire, in seconds.
    Exat(u64),
    /// Set the specified Unix time at which the key will expire, in milliseconds.
    Pxat(u64),
}

impl Default for SetExpiration {
    fn default() -> Self {
        SetExpiration::None
    }
}

impl IntoArgs for SetExpiration {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            SetExpiration::None => args,
            SetExpiration::Ex(duration) => ("EX", duration).into_args(args),
            SetExpiration::Px(duration) => ("PX", duration).into_args(args),
            SetExpiration::Exat(timestamp) => ("EXAT", timestamp).into_args(args),
            SetExpiration::Pxat(timestamp) => ("PXAT", timestamp).into_args(args),
        }
    }
}

/// Condition option for the [`set_with_options`](crate::StringCommands::set_with_options) command
pub enum SetCondition {
    /// No condition
    None,
    /// Only set the key if it does not already exist.
    NX,
    /// Only set the key if it already exist.
    XX,
}

impl Default for SetCondition {
    fn default() -> Self {
        SetCondition::None
    }
}

impl IntoArgs for SetCondition {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            SetCondition::None => args,
            SetCondition::NX => args.arg("NX"),
            SetCondition::XX => args.arg("XX"),
        }
    }
}
