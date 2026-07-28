use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{RequestPolicy, ResponsePolicy},
    resp::{FastPathCommandBuilder, Response, cmd, serialize_flag},
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, SeqAccess, Visitor},
};
use std::fmt;

/// A group of Redis commands related to [`Strings`](https://redis.io/docs/data-types/strings/)
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=string)
pub trait StringCommands<'a>: Sized {
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
    fn append(
        self,
        key: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("APPEND").key(key).arg(value))
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
    fn decr(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("DECR").key(key))
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
    fn decrby(self, key: impl Serialize, decrement: i64) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("DECRBY").key(key).arg(decrement))
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
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///     client.flushall(FlushingMode::Sync).await?;
    ///
    ///     // return value can be an Option<String>...
    ///     let value: Option<String> = client.get("key").await?;
    ///     assert_eq!(None, value);
    ///
    ///     // ... or it can be directly a String.
    ///     // In this cas a `nil` value will result in an empty String
    ///     let value: String = client.get("key").await?;
    ///     assert_eq!("", value);
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
    fn get<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, FastPathCommandBuilder::get(key))
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
    fn getdel<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("GETDEL").key(key))
    }

    /// Returns the hash digest of a string value as a hexadecimal string.
    ///
    /// The digest is stable for a given value, so it can be captured and later
    /// passed to [`delex`](StringCommands::delex) or `SET`'s `IFDEQ`/`IFDNE`
    /// conditions for compare-and-delete / compare-and-set flows.
    ///
    /// # Return
    /// the hexadecimal digest of the value, or `nil` when the key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/digest/>](https://redis.io/commands/digest/)
    #[must_use]
    fn digest<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("DIGEST").key(key))
    }

    /// Conditionally removes `key` based on a value or digest comparison.
    ///
    /// With no condition the key is deleted unconditionally (like `DEL` on a
    /// single key). With a [`DelexCondition`] the key is deleted only if its
    /// current value (or its digest) satisfies the comparison.
    ///
    /// # Return
    /// * `1` if the key was deleted.
    /// * `0` if the key does not exist or the condition was not met.
    ///
    /// # See Also
    /// [<https://redis.io/commands/delex/>](https://redis.io/commands/delex/)
    #[must_use]
    fn delex<'b>(
        self,
        key: impl Serialize,
        condition: impl Into<Option<DelexCondition<'b>>>,
    ) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("DELEX").key(key).arg(condition.into()))
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
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///     client.flushall(FlushingMode::Sync).await?;
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
    fn getex<R: Response>(
        self,
        key: impl Serialize,
        options: GetExOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("GETEX").key(key).arg(options))
    }

    /// Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).
    ///
    /// Negative offsets can be used in order to provide an offset starting from the end of the string.
    /// So -1 means the last character, -2 the penultimate and so forth.
    ///
    /// The function handles out of range requests by limiting the resulting range to the actual length of the string.
    ///
    /// # Example
    /// ```
    /// use rustis::{
    ///     client::Client,
    ///     commands::{FlushingMode, ServerCommands, StringCommands},
    ///     Result,
    /// };
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///     client.flushall(FlushingMode::Sync).await?;
    ///     client.set("mykey", "This is a string").await?;
    ///
    ///     let value: String = client.getrange("mykey", 0, 3).await?;
    ///     assert_eq!("This", value);
    ///     let value: String = client.getrange("mykey", -3, -1).await?;
    ///     assert_eq!("ing", value);
    ///     let value: String = client.getrange("mykey", 0, -1).await?;
    ///     assert_eq!("This is a string", value);
    ///     let value: String = client.getrange("mykey", 10, 100).await?;
    ///     assert_eq!("string", value);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/getrange/>](https://redis.io/commands/getrange/)
    #[must_use]
    fn getrange<R: Response>(
        self,
        key: impl Serialize,
        start: isize,
        end: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("GETRANGE").key(key).arg(start).arg(end))
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
    fn getset<R: Response>(
        self,
        key: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("GETSET").key(key).arg(value))
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
    fn incr(self, key: impl Serialize) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("INCR").key(key))
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
    fn incrby(self, key: impl Serialize, increment: i64) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("INCRBY").key(key).arg(increment))
    }

    /// Increment the value at `key`, bounded, and set its expiration, atomically.
    ///
    /// The key is created at `0` when it does not exist. Without an increment
    /// it is bumped by `1` in integer mode.
    ///
    /// Where [`incr`](StringCommands::incr) and [`expire`](crate::commands::GenericCommands::expire)
    /// would need a Lua script to be atomic, this is one command — which is
    /// what makes it a window-counter rate limiter:
    /// [`ubound_int`](IncrExOptions::ubound_int) is the cap and
    /// [`enx`](IncrExOptions::enx) starts the window only once.
    ///
    /// # Return
    /// A pair of the value after the operation and the increment actually
    /// applied. The applied increment is `0` when a bound stopped the operation,
    /// and smaller than requested when [`saturate`](IncrExOptions::saturate)
    /// capped it. Both are integers in integer mode, doubles under
    /// [`by_float`](IncrExOptions::by_float) — so `(i64, i64)` or `(f64, f64)`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/increx/>](https://redis.io/commands/increx/)
    #[must_use]
    fn increx<R: Response>(
        self,
        key: impl Serialize,
        options: IncrExOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("INCREX").key(key).arg(options))
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
    fn incrbyfloat(self, key: impl Serialize, increment: f64) -> PreparedCommand<'a, Self, f64> {
        prepare_command(self, cmd("INCRBYFLOAT").key(key).arg(increment))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// The string representing the longest common substring.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lcs/>](https://redis.io/commands/lcs/)
    #[must_use]
    fn lcs<R: Response>(
        self,
        key1: impl Serialize,
        key2: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("LCS").key(key1).arg(key2))
    }

    /// The LCS command implements the longest common subsequence algorithm
    ///
    /// # Return
    /// The length of the longest common substring.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lcs/>](https://redis.io/commands/lcs/)
    #[must_use]
    fn lcs_len(
        self,
        key1: impl Serialize,
        key2: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("LCS").key(key1).key(key2).arg("LEN"))
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
    fn lcs_idx(
        self,
        key1: impl Serialize,
        key2: impl Serialize,
        min_match_len: Option<usize>,
        with_match_len: bool,
    ) -> PreparedCommand<'a, Self, LcsResult> {
        prepare_command(
            self,
            cmd("LCS")
                .key(key1)
                .key(key2)
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
    fn mget<R: Response>(self, keys: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("MGET")
                .key(keys)
                .cluster_info(RequestPolicy::MultiShard, None, 1),
        )
    }

    /// Sets the given keys to their respective values.
    ///
    /// # Return
    /// always OK since MSET can't fail.
    ///
    /// # See Also
    /// [<https://redis.io/commands/mset/>](https://redis.io/commands/mset/)
    #[must_use]
    fn mset(self, items: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("MSET").key_with_step(items, 2).cluster_info(
                RequestPolicy::MultiShard,
                ResponsePolicy::AllSucceeded,
                2,
            ),
        )
    }

    /// Atomically sets multiple string keys with an optional shared expiration in a single operation.
    ///
    /// # Return
    /// * `false` - if none of the keys were set
    /// * `true` - if all of the keys were set.
    ///
    /// # Cluster
    /// In cluster mode all keys must hash to the same slot, otherwise the
    /// command fails client-side with a mismatched-slot error.
    ///
    /// # See Also
    /// [<https://redis.io/commands/msetex/>](https://redis.io/commands/msetex/)
    #[must_use]
    fn msetex<'b>(
        self,
        items: impl Serialize,
        condition: impl Into<Option<SetCondition<'b>>>,
        expiration: impl Into<Option<SetExpiration>>,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("MSETEX")
                .key_with_count_and_step(items, 2)
                .arg(condition.into())
                .arg(expiration.into())
                .cluster_info(RequestPolicy::MultiShard, ResponsePolicy::AllSucceeded, 2),
        )
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
    /// # Cluster
    /// Unlike [`mset`](StringCommands::mset), MSETNX is routed to a single node:
    /// its all-or-nothing atomicity cannot be preserved if the keys were split
    /// across shards. All keys must therefore hash to the same slot in cluster
    /// mode, otherwise the command fails client-side with a mismatched-slot error.
    ///
    /// # See Also
    /// [<https://redis.io/commands/msetnx/>](https://redis.io/commands/msetnx/)
    #[must_use]
    fn msetnx(self, items: impl Serialize) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("MSETNX")
                .key_with_step(items, 2)
                .cluster_info(None, None, 2),
        )
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
    fn psetex(
        self,
        key: impl Serialize,
        milliseconds: u64,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("PSETEX").key(key).arg(milliseconds).arg(value))
    }

    ///Set key to hold the string value.
    ///
    /// If key already holds a value, it is overwritten, regardless of its type.
    /// Any previous time to live associated with the key is discarded on successful SET operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set(self, key: impl Serialize, value: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, FastPathCommandBuilder::set(key, value))
    }

    /// Set key to hold the string value.
    ///
    /// # Return
    /// * `true` if SET was executed correctly.
    /// * `false` if the SET operation was not performed because the user
    ///   specified the NX or XX option but the condition was not met.
    ///
    /// # See Also
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set_with_options<'b>(
        self,
        key: impl Serialize,
        value: impl Serialize,
        condition: impl Into<Option<SetCondition<'b>>>,
        expiration: impl Into<Option<SetExpiration>>,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("SET")
                .key(key)
                .arg(value)
                .arg(condition.into())
                .arg(expiration.into()),
        )
    }

    /// Set key to hold the string value wit GET option enforced
    ///
    /// # See Also
    /// [<https://redis.io/commands/set/>](https://redis.io/commands/set/)
    #[must_use]
    fn set_get_with_options<'b, R: Response>(
        self,
        key: impl Serialize,
        value: impl Serialize,
        condition: impl Into<Option<SetCondition<'b>>>,
        expiration: impl Into<Option<SetExpiration>>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("SET")
                .key(key)
                .arg(value)
                .arg(condition.into())
                .arg("GET")
                .arg(expiration.into()),
        )
    }

    /// Set key to hold the string value and set key to timeout after a given number of seconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/setex/>](https://redis.io/commands/setex/)
    #[must_use]
    fn setex(
        self,
        key: impl Serialize,
        seconds: u64,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("SETEX").key(key).arg(seconds).arg(value))
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
    fn setnx(self, key: impl Serialize, value: impl Serialize) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("SETNX").key(key).arg(value))
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
    fn setrange(
        self,
        key: impl Serialize,
        offset: usize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SETRANGE").key(key).arg(offset).arg(value))
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
    fn strlen(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("STRLEN").key(key))
    }

    /// Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).
    ///
    /// Negative offsets can be used in order to provide an offset starting from the end of the string.
    /// So -1 means the last character, -2 the penultimate and so forth.
    ///
    /// The function handles out of range requests by limiting the resulting range to the actual length of the string.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::{FlushingMode, ServerCommands, StringCommands},
    /// #    Result,
    /// # };
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// #    let client = Client::connect("127.0.0.1:6379").await?;
    /// #    client.flushdb(FlushingMode::Sync).await?;
    /// client.set("mykey", "This is a string").await?;
    ///
    /// let value: String = client.substr("mykey", 0, 3).await?;
    /// assert_eq!("This", value);
    /// let value: String = client.substr("mykey", -3, -1).await?;
    /// assert_eq!("ing", value);
    /// let value: String = client.substr("mykey", 0, -1).await?;
    /// assert_eq!("This is a string", value);
    /// let value: String = client.substr("mykey", 10, 100).await?;
    /// assert_eq!("string", value);
    /// #    Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/substr/>](https://redis.io/commands/substr/)
    #[must_use]
    fn substr<R: Response>(
        self,
        key: impl Serialize,
        start: isize,
        end: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SUBSTR").key(key).arg(start).arg(end))
    }
}

/// Options for the [`getex`](StringCommands::getex) and the [`hgetex`](crate::commands::HashCommands::hgetex) commands
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
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

/// Part of the result for the [`lcs`](StringCommands::lcs) command
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct LcsResult {
    pub matches: Vec<LcsMatch>,
    pub len: usize,
}

/// Options for the [`increx`](StringCommands::increx) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct IncrExOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    byint: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    byfloat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lbound: Option<IncrExBound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ubound: Option<IncrExBound>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    saturate: bool,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    expiration: Option<GetExOptions>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    enx: bool,
}

/// A bound of the [`increx`](StringCommands::increx) command, in whichever mode
/// the increment put it in.
#[derive(Serialize)]
#[serde(untagged)]
enum IncrExBound {
    Int(i64),
    Float(f64),
}

impl IncrExOptions {
    /// Increment by a 64-bit signed integer. Negative decrements.
    ///
    /// The stored value must be integer-typed: a stored `"1.5"` cannot be read
    /// back as an integer, exactly as with [`incrby`](StringCommands::incrby).
    #[must_use]
    pub fn by_int(increment: i64) -> Self {
        Self {
            byint: Some(increment),
            ..Default::default()
        }
    }

    /// Increment by a floating-point value.
    ///
    /// The stored value may be an integer or a float, since integers promote to
    /// floats losslessly. A result of NaN or infinity is rejected.
    #[must_use]
    pub fn by_float(increment: f64) -> Self {
        Self {
            byfloat: Some(increment),
            ..Default::default()
        }
    }

    /// Lower bound, in integer mode.
    #[must_use]
    pub fn lbound_int(mut self, lower_bound: i64) -> Self {
        self.lbound = Some(IncrExBound::Int(lower_bound));
        self
    }

    /// Lower bound, in [`by_float`](IncrExOptions::by_float) mode.
    #[must_use]
    pub fn lbound_float(mut self, lower_bound: f64) -> Self {
        self.lbound = Some(IncrExBound::Float(lower_bound));
        self
    }

    /// Upper bound, in integer mode.
    #[must_use]
    pub fn ubound_int(mut self, upper_bound: i64) -> Self {
        self.ubound = Some(IncrExBound::Int(upper_bound));
        self
    }

    /// Upper bound, in [`by_float`](IncrExOptions::by_float) mode.
    #[must_use]
    pub fn ubound_float(mut self, upper_bound: f64) -> Self {
        self.ubound = Some(IncrExBound::Float(upper_bound));
        self
    }

    /// Cap an out-of-bounds result at the bound instead of skipping the
    /// operation. Without it, a bound violation leaves the key and its TTL
    /// untouched and reports a zero increment.
    #[must_use]
    pub fn saturate(mut self) -> Self {
        self.saturate = true;
        self
    }

    /// Set the expiration, in seconds.
    #[must_use]
    pub fn ex(mut self, seconds: u64) -> Self {
        self.expiration = Some(GetExOptions::Ex(seconds));
        self
    }

    /// Set the expiration, in milliseconds.
    #[must_use]
    pub fn px(mut self, milliseconds: u64) -> Self {
        self.expiration = Some(GetExOptions::Px(milliseconds));
        self
    }

    /// Set the Unix time at which the key expires, in seconds.
    #[must_use]
    pub fn exat(mut self, unix_time_seconds: u64) -> Self {
        self.expiration = Some(GetExOptions::Exat(unix_time_seconds));
        self
    }

    /// Set the Unix time at which the key expires, in milliseconds.
    #[must_use]
    pub fn pxat(mut self, unix_time_milliseconds: u64) -> Self {
        self.expiration = Some(GetExOptions::Pxat(unix_time_milliseconds));
        self
    }

    /// Remove the expiration of the key.
    #[must_use]
    pub fn persist(mut self) -> Self {
        self.expiration = Some(GetExOptions::Persist);
        self
    }

    /// Set the expiration only when the key has none. An existing TTL is kept
    /// as it is, while the increment still applies. Requires one of
    /// [`ex`](IncrExOptions::ex), [`px`](IncrExOptions::px),
    /// [`exat`](IncrExOptions::exat) or [`pxat`](IncrExOptions::pxat), and is
    /// incompatible with [`persist`](IncrExOptions::persist).
    #[must_use]
    pub fn enx(mut self) -> Self {
        self.enx = true;
        self
    }
}

/// Expiration option for the [`set_with_options`](StringCommands::set_with_options) and [`hsetex`](crate::commands::HashCommands::hsetex) commands
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum SetExpiration {
    /// Set the specified expire time, in seconds.
    Ex(u64),
    /// Set the specified expire time, in milliseconds.
    Px(u64),
    /// Set the specified Unix time at which the key will expire, in seconds.
    Exat(u64),
    /// Set the specified Unix time at which the key will expire, in milliseconds.
    Pxat(u64),
    /// Retain the time to live associated with the key.
    KeepTtl,
}

/// Condition option for the [`delex`](StringCommands::delex) command.
///
/// Mirrors the `IFEQ`/`IFNE`/`IFDEQ`/`IFDNE` value/digest comparisons of
/// [`SetCondition`], without the `NX`/`XX` existence conditions, which `DELEX`
/// does not accept.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum DelexCondition<'a> {
    /// Delete only if the current value is equal to the provided value.
    IFEQ(&'a str),
    /// Delete only if the current value is not equal to the provided value.
    IFNE(&'a str),
    /// Delete only if the digest of the current value is equal to the provided digest.
    IFDEQ(&'a str),
    /// Delete only if the digest of the current value is not equal to the provided digest.
    IFDNE(&'a str),
}

/// Condition option for the [`set_with_options`](StringCommands::set_with_options) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum SetCondition<'a> {
    /// Only set the key if it does not already exist.
    NX,
    /// Only set the key if it already exist.
    XX,
    /// Set the key’s value and expiration only if its current value is equal to the provided value.
    /// If the key doesn’t exist, it won’t be created.
    IFEQ(&'a str),
    /// Set the key’s value and expiration only if its current value is not equal to the provided value.
    /// If the key doesn’t exist, it will be created.
    IFNE(&'a str),
    /// Set the key’s value and expiration only if the hash digest of its current value is equal to the provided digest.
    /// If the key doesn’t exist, it won’t be created.
    IFDEQ(&'a str),
    /// Set the key’s value and expiration only if the hash digest of its current value is not equal to the provided digest.
    /// If the key doesn’t exist, it will be created.
    IFDNE(&'a str),
}
