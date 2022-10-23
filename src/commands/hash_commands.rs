use crate::{
    prepare_command,
    resp::{
        cmd, BulkString, CommandArgs, FromKeyValueValueArray, FromSingleValueArray, FromValue,
        IntoArgs, KeyValueArgOrCollection, SingleArgOrCollection,
    },
    PreparedCommand,
};

/// A group of Redis commands related to [`Hashes`](https://redis.io/docs/data-types/hashes/)
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hash)
pub trait HashCommands {
    /// Removes the specified fields from the hash stored at key.
    ///
    /// # Return
    /// the number of fields that were removed from the hash, not including specified but non existing fields.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hdel/>](https://redis.io/commands/hdel/)
    #[must_use]
    fn hdel<K, F, C>(&mut self, key: K, fields: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
        C: SingleArgOrCollection<F>,
    {
        prepare_command(self, cmd("HDEL").arg(key).arg(fields))
    }

    /// Returns if field is an existing field in the hash stored at key.
    ///
    /// # Return
    /// * `true` - if the hash contains field.
    /// * `false` - if the hash does not contain field, or key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hexists/>](https://redis.io/commands/hexists/)
    #[must_use]
    fn hexists<K, F>(&mut self, key: K, field: F) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
    {
        prepare_command(self, cmd("HEXISTS").arg(key).arg(field))
    }

    /// Returns the value associated with field in the hash stored at key.
    ///
    /// # Return
    /// The value associated with field, or nil when field is not present in the hash or key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hget/>](https://redis.io/commands/hget/)
    #[must_use]
    fn hget<K, F, V>(&mut self, key: K, field: F) -> PreparedCommand<Self, V>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
        V: FromValue,
    {
        prepare_command(self, cmd("HGET").arg(key).arg(field))
    }

    /// Returns all fields and values of the hash stored at key.
    ///
    /// # Return
    /// The list of fields and their values stored in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hgetall/>](https://redis.io/commands/hgetall/)
    #[must_use]
    fn hgetall<K, F, V, A>(&mut self, key: K) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: FromValue,
        V: FromValue,
        A: FromKeyValueValueArray<F, V>,
    {
        prepare_command(self, cmd("HGETALL").arg(key))
    }

    /// Increments the number stored at field in the hash stored at key by increment.
    ///
    /// # Return
    /// The value at field after the increment operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hincrby/>](https://redis.io/commands/hincrby/)
    #[must_use]
    fn hincrby<K, F>(&mut self, key: K, field: F, increment: i64) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
    {
        prepare_command(self, cmd("HINCRBY").arg(key).arg(field).arg(increment))
    }

    /// Increment the specified field of a hash stored at key,
    /// and representing a floating point number, by the specified increment.
    ///
    /// # Return
    /// The value at field after the increment operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hincrbyfloat/>](https://redis.io/commands/hincrbyfloat/)
    #[must_use]
    fn hincrbyfloat<K, F>(&mut self, key: K, field: F, increment: f64) -> PreparedCommand<Self, f64>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
    {
        prepare_command(self, cmd("HINCRBYFLOAT").arg(key).arg(field).arg(increment))
    }

    /// Returns all field names in the hash stored at key.
    ///
    /// # Return
    /// The list of fields in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hkeys/>](https://redis.io/commands/hkeys/)
    #[must_use]
    fn hkeys<K, F, A>(&mut self, key: K) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: FromValue,
        A: FromSingleValueArray<F>,
    {
        prepare_command(self, cmd("HKEYS").arg(key))
    }

    /// Returns the number of fields contained in the hash stored at key.
    ///
    /// # Return
    /// The number of fields in the hash, or 0 when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hlen/>](https://redis.io/commands/hlen/)
    #[must_use]
    fn hlen<K>(&mut self, key: K) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<BulkString>,
    {
        prepare_command(self, cmd("HLEN").arg(key))
    }

    /// Returns the values associated with the specified fields in the hash stored at key.
    ///
    /// # Return
    /// The list of values associated with the given fields, in the same order as they are requested.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hmget/>](https://redis.io/commands/hmget/)
    #[must_use]
    fn hmget<K, F, V, C, A>(&mut self, key: K, fields: C) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
        C: SingleArgOrCollection<F>,
        V: FromValue,
        A: FromSingleValueArray<V>,
    {
        prepare_command(self, cmd("HMGET").arg(key).arg(fields))
    }

    /// return random fields from the hash value stored at key.
    ///
    /// # Return
    /// * When called with just the key argument, return a random field from the hash value stored at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hrandfield/>](https://redis.io/commands/hrandfield/)
    #[must_use]
    fn hrandfield<K, F>(&mut self, key: K) -> PreparedCommand<Self, F>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: FromValue,
    {
        prepare_command(self, cmd("HRANDFIELD").arg(key))
    }

    /// return random fields from the hash value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct fields.
    /// The array's length is either count or the hash's number of fields (HLEN), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed to return the same field multiple times.
    /// In this case, the number of returned fields is the absolute value of the specified count.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hrandfield/>](https://redis.io/commands/hrandfield/)
    #[must_use]
    fn hrandfields<K, F, A>(&mut self, key: K, count: isize) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: FromValue,
        A: FromSingleValueArray<F>,
    {
        prepare_command(self, cmd("HRANDFIELD").arg(key).arg(count))
    }

    /// return random fields from the hash value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct fields.
    /// The array's length is either count or the hash's number of fields (HLEN), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed to return the same field multiple times.
    /// In this case, the number of returned fields is the absolute value of the specified count.
    /// The optional WITHVALUES modifier changes the reply so it includes the respective values of the randomly selected hash fields.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hrandfield/>](https://redis.io/commands/hrandfield/)
    #[must_use]
    fn hrandfields_with_values<K, F, V, A>(
        &mut self,
        key: K,
        count: isize,
    ) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: FromValue,
        V: FromValue,
        A: FromKeyValueValueArray<F, V>,
    {
        prepare_command(
            self,
            cmd("HRANDFIELD").arg(key).arg(count).arg("WITHVALUES"),
        )
    }

    /// Iterates fields of Hash types and their associated values.
    ///
    /// # Return
    /// array of elements contain two elements, a field and a value,
    /// for every returned element of the Hash.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hlen/>](https://redis.io/commands/hscan/)
    #[must_use]
    fn hscan<K, F, V>(
        &mut self,
        key: K,
        cursor: u64,
        options: HScanOptions,
    ) -> PreparedCommand<Self, (u64, Vec<(F, V)>)>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: FromValue,
        V: FromValue,
    {
        prepare_command(self, cmd("HSCAN").arg(key).arg(cursor).arg(options))
    }

    /// Sets field in the hash stored at key to value.
    ///
    /// # Return
    /// The number of fields that were added.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hset/>](https://redis.io/commands/hset/)
    #[must_use]
    fn hset<K, F, V, I>(&mut self, key: K, items: I) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
        V: Into<BulkString>,
        I: KeyValueArgOrCollection<F, V>,
    {
        prepare_command(self, cmd("HSET").arg(key).arg(items))
    }

    /// Sets field in the hash stored at key to value, only if field does not yet exist.
    ///
    /// # Return
    /// * `true` - if field is a new field in the hash and value was set.
    /// * `false` - if field already exists in the hash and no operation was performed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hsetnx/>](https://redis.io/commands/hsetnx/)
    #[must_use]
    fn hsetnx<K, F, V>(&mut self, key: K, field: F, value: V) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
        V: Into<BulkString>,
    {
        prepare_command(self, cmd("HSETNX").arg(key).arg(field).arg(value))
    }

    /// Returns the string length of the value associated with field in the hash stored at key.
    ///
    /// # Return
    /// the string length of the value associated with field,
    /// or zero when field is not present in the hash or key does not exist at all.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hstrlen/>](https://redis.io/commands/hstrlen/)
    #[must_use]
    fn hstrlen<K, F>(&mut self, key: K, field: F) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: Into<BulkString>,
        F: Into<BulkString>,
    {
        prepare_command(self, cmd("HSTRLEN").arg(key).arg(field))
    }

    /// list of values in the hash, or an empty list when key does not exist.
    ///
    /// # Return
    /// The list of values in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hvals/>](https://redis.io/commands/hvals/)
    #[must_use]
    fn hvals<K, V, A>(&mut self, key: K) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: Into<BulkString>,
        V: FromValue,
        A: FromSingleValueArray<V>,
    {
        prepare_command(self, cmd("HVALS").arg(key))
    }
}

/// Options for the [`hscan`](crate::HashCommands::hscan) command
#[derive(Default)]
pub struct HScanOptions {
    command_args: CommandArgs,
}

impl HScanOptions {
    #[must_use]
    pub fn match_pattern<P: Into<BulkString>>(self, match_pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("MATCH").arg(match_pattern),
        }
    }

    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }
}

impl IntoArgs for HScanOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
