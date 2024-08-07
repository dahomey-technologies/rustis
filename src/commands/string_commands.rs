use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CollectionResponse, CommandArgs, KeyValueArgsCollection, PrimitiveResponse, SingleArg,
        SingleArgCollection, ToArgs,
    },
};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

/// A group of Redis commands related to [`Strings`](https://redis.io/docs/data-types/strings/)
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=string)
pub trait StringCommands<'a> {
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
    fn append<K, V>(self, key: K, value: V) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
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
    fn decr<K>(self, key: K) -> PreparedCommand<'a, Self, i64>
    where
        Self: Sized,
        K: SingleArg,
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
    fn decrby<K>(self, key: K, decrement: i64) -> PreparedCommand<'a, Self, i64>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("DECRBY").arg(key).arg(decrement))
    }

    /// Get the value of key.
    ///
    /// Get the value of key. If the key does not exist the special 
    /// value nil is returned. An error is returned if the value 
    /// stored at key is not a string, because GET only handles 
    /// string values.
    ///
    /// # Return
    /// the value of key, or `nil` when key does not exist.
    ///
    /// # Example
    /// ```
    /// use rustis::{
    ///     client::{Client, ClientPreparedCommand},
    ///     commands::{FlushingMode, ServerCommands, StringCommands},
    ///     resp::{cmd},
    ///     Result
    /// };
    ///
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
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
    fn get<K, V>(self, key: K) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        K: SingleArg,
        V: PrimitiveResponse,
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
    fn getdel<K, V>(self, key: K) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        K: SingleArg,
        V: PrimitiveResponse,
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
    ///     client::{Client, ClientPreparedCommand},
    ///     commands::{FlushingMode, GetExOptions, GenericCommands, ServerCommands, StringCommands},
    ///     resp::cmd,
    ///     Result,
    /// };
    ///
    /// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
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
    fn getex<K, V>(self, key: K, options: GetExOptions) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        K: SingleArg,
        V: PrimitiveResponse,
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
    fn getrange<K, V>(self, key: K, start: usize, end: isize) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        K: SingleArg,
        V: PrimitiveResponse,
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
    fn getset<K, V, R>(self, key: K, value: V) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
        R: PrimitiveResponse,
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
    fn incr<K>(self, key: K) -> PreparedCommand<'a, Self, i64>
    where
        Self: Sized,
        K: SingleArg,
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
    /// See [incr](StringCommands::incr) for extra information on increment/decrement operations.
    ///
    /// # Return
    /// the value of key after the increment
    ///
    /// # See Also
    /// [<https://redis.io/commands/incrby/>](https://redis.io/commands/incrby/)
    #[must_use]
    fn incrby<K>(self, key: K, increment: i64) -> PreparedCommand<'a, Self, i64>
    where
        Self: Sized,
        K: SingleArg,
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
    fn incrbyfloat<K>(self, key: K, increment: f64) -> PreparedCommand<'a, Self, f64>
    where
        Self: Sized,
        K: SingleArg,
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
    fn lcs<K, V>(self, key1: K, key2: K) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        K: SingleArg,
        V: PrimitiveResponse,
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
    fn lcs_len<K>(self, key1: K, key2: K) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
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
        self,
        key1: K,
        key2: K,
        min_match_len: Option<usize>,
        with_match_len: bool,
    ) -> PreparedCommand<'a, Self, LcsResult>
    where
        Self: Sized,
        K: SingleArg,
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
    fn mget<K, KK, V, VV>(self, keys: KK) -> PreparedCommand<'a, Self, VV>
    where
        Self: Sized,
        K: SingleArg,
        KK: SingleArgCollection<K>,
        V: PrimitiveResponse + serde::de::DeserializeOwned,
        VV: CollectionResponse<V>,
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
    fn mset<K, V, C>(self, items: C) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        C: KeyValueArgsCollection<K, V>,
        K: SingleArg,
        V: SingleArg,
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
    fn msetnx<K, V, C>(self, items: C) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        C: KeyValueArgsCollection<K, V>,
        K: SingleArg,
        V: SingleArg,
    {
        prepare_command(self, cmd("MSETNX").arg(items))
    }

    /// Works exactly like [setex](StringCommands::setex) with the sole
    /// difference that the expire time is specified in milliseconds instead of seconds.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/psetex/>](https://redis.io/commands/psetex/)
    #[must_use]
    fn psetex<K, V>(self, key: K, milliseconds: u64, value: V) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
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
    fn set<K, V>(self, key: K, value: V) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
        Self: Sized,
    {
        prepare_command(self, cmd("SET").arg(key).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// # Return
    /// * `true` if SET was executed correctly.
    /// * `false` if the SET operation was not performed because the user
    ///   specified the NX or XX option but the condition was not met.
    ///
    /// # See Also
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set_with_options<K, V>(
        self,
        key: K,
        value: V,
        condition: SetCondition,
        expiration: SetExpiration,
        keep_ttl: bool,
    ) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
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
        self,
        key: K,
        value: V1,
        condition: SetCondition,
        expiration: SetExpiration,
        keep_ttl: bool,
    ) -> PreparedCommand<'a, Self, V2>
    where
        Self: Sized,
        K: SingleArg,
        V1: SingleArg,
        V2: PrimitiveResponse,
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
    fn setex<K, V>(self, key: K, seconds: u64, value: V) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
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
    fn setnx<K, V>(self, key: K, value: V) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
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
    fn setrange<K, V>(self, key: K, offset: usize, value: V) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        V: SingleArg,
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
    fn strlen<K>(self, key: K) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("STRLEN").arg(key))
    }
}

/// Options for the [`getex`](StringCommands::getex) command
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

impl ToArgs for GetExOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            GetExOptions::Ex(duration) => args.arg(("EX", *duration)),
            GetExOptions::Px(duration) => args.arg(("PX", *duration)),
            GetExOptions::Exat(timestamp) => args.arg(("EXAT", *timestamp)),
            GetExOptions::Pxat(timestamp) => args.arg(("PXAT", *timestamp)),
            GetExOptions::Persist => args.arg("PERSIST"),
        };
    }
}

/// Part of the result for the [`lcs`](StringCommands::lcs) command
#[derive(Debug, PartialEq, Eq)]
pub struct LcsMatch(pub (usize, usize), pub (usize, usize), pub Option<usize>);

impl<'de> Deserialize<'de> for LcsMatch {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LcsMatchVisitor;

        impl<'de> Visitor<'de> for LcsMatchVisitor {
            type Value = LcsMatch;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("LcsMatch")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let Some(first): Option<(usize, usize)> = seq.next_element()? else {
                    return Err(de::Error::invalid_length(0, &"fewer elements in tuple"));
                };

                let Some(second): Option<(usize, usize)> = seq.next_element()? else {
                    return Err(de::Error::invalid_length(1, &"fewer elements in tuple"));
                };

                let match_len: Option<usize> = seq.next_element()?;

                Ok(LcsMatch(first, second, match_len))
            }
        }

        deserializer.deserialize_seq(LcsMatchVisitor)
    }
}

/// Result for the [`lcs`](StringCommands::lcs) command
#[derive(Debug, Deserialize)]
pub struct LcsResult {
    pub matches: Vec<LcsMatch>,
    pub len: usize,
}

/// Expiration option for the [`set_with_options`](StringCommands::set_with_options) command
#[derive(Default)]
pub enum SetExpiration {
    /// No expiration
    #[default]
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

impl ToArgs for SetExpiration {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            SetExpiration::None => {}
            SetExpiration::Ex(duration) => {
                args.arg(("EX", *duration));
            }
            SetExpiration::Px(duration) => {
                args.arg(("PX", *duration));
            }
            SetExpiration::Exat(timestamp) => {
                args.arg(("EXAT", *timestamp));
            }
            SetExpiration::Pxat(timestamp) => {
                args.arg(("PXAT", *timestamp));
            }
        };
    }
}

/// Condition option for the [`set_with_options`](StringCommands::set_with_options) command
#[derive(Default)]
pub enum SetCondition {
    /// No condition
    #[default]
    None,
    /// Only set the key if it does not already exist.
    NX,
    /// Only set the key if it already exist.
    XX,
}

impl ToArgs for SetCondition {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            SetCondition::None => {}
            SetCondition::NX => {
                args.arg("NX");
            }
            SetCondition::XX => {
                args.arg("XX");
            }
        }
    }
}
