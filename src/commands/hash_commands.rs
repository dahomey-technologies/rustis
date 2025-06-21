use crate::{
    client::{prepare_command, PreparedCommand},
    commands::{ExpireOption, GetExOptions, SetExpiration},
    resp::{
        cmd, deserialize_vec_of_pairs, CollectionResponse, CommandArgs, KeyValueArgsCollection,
        KeyValueCollectionResponse, PrimitiveResponse, SingleArg, SingleArgCollection, ToArgs,
    },
};
use serde::{de::DeserializeOwned, Deserialize};

/// A group of Redis commands related to [`Hashes`](https://redis.io/docs/data-types/hashes/)
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hash)
pub trait HashCommands<'a> {
    /// Removes the specified fields from the hash stored at key.
    ///
    /// # Return
    /// the number of fields that were removed from the hash, not including specified but non existing fields.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hdel/>](https://redis.io/commands/hdel/)
    #[must_use]
    fn hdel<K, F, C>(self, key: K, fields: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        C: SingleArgCollection<F>,
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
    fn hexists<K, F>(self, key: K, field: F) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
    {
        prepare_command(self, cmd("HEXISTS").arg(key).arg(field))
    }

    /// Set an expiration (TTL or time to live) on one or more fields of a given hash key.
    ///
    /// Field(s) will automatically be deleted from the hash key when their TTLs expire.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `seconds ` - The expiration time in seconds
    /// * `option` - The [`ExpireOption`](commands::ExpireOption) option.
    /// * `fields` - The fields to expire.
    ///
    /// # Return
    /// For each field:
    /// * `-2` - if no such field exists in the provided hash key, or the provided key does not exist.
    /// * `0` - if the specified NX | XX | GT | LT condition has not been met.
    /// * `1` - if the expiration time was set/updated.
    /// * `2` - when the command is called with 0 seconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hexpire/>](https://redis.io/commands/hexpire/)
    #[must_use]
    fn hexpire<K, F, FF, R>(
        self,
        key: K,
        seconds: u64,
        option: ExpireOption,
        fields: FF,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HEXPIRE")
                .arg(key)
                .arg(seconds)
                .arg(option)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// HEXPIREAT has the same effect and semantics as HEXPIRE,
    /// but instead of specifying the number of seconds for the TTL (time to live),
    /// it takes an absolute Unix timestamp in seconds since Unix epoch.
    ///
    /// A timestamp in the past will delete the field immediately.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `unix_time_seconds ` - The aboslute unix timestamp the fields will expire at.
    /// * `option` - The [`ExpireOption`](commands::ExpireOption) option.
    /// * `fields` - The fields to expire.
    ///
    /// # Return
    /// For each field:
    /// * `-2` - if no such field exists in the provided hash key, or the provided key does not exist.
    /// * `0` - if the specified NX | XX | GT | LT condition has not been met.
    /// * `1` - if the expiration time was set/updated.
    /// * `2` - when the command is called with a past Unix time in seconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hexpireat/>](https://redis.io/commands/hexpireat/)
    #[must_use]
    fn hexpireat<K, F, FF, R>(
        self,
        key: K,
        unix_time_seconds: u64,
        option: ExpireOption,
        fields: FF,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HEXPIREAT")
                .arg(key)
                .arg(unix_time_seconds)
                .arg(option)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// Returns the absolute Unix timestamp in seconds since Unix epoch
    /// at which the given key's field(s) will expire.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `fields` - The fields to get the expiration time from.
    ///
    /// # Return
    /// For each field, the expiration (Unix timestamp) in seconds.
    /// - The command returns -2 if no such field exists in the provided hash key, or the provided key does not exist.
    /// - The command returns -1 if the field exists but has no associated expiration set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hexpiretime/>](https://redis.io/commands/hexpiretime/)
    #[must_use]
    fn hexpiretime<K, F, FF, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HEXPIRETIME")
                .arg(key)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// Returns the value associated with field in the hash stored at key.
    ///
    /// # Return
    /// The value associated with field, or nil when field is not present in the hash or key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hget/>](https://redis.io/commands/hget/)
    #[must_use]
    fn hget<K, F, V>(self, key: K, field: F) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        V: PrimitiveResponse,
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
    fn hgetall<K, F, V, R>(self, key: K) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: PrimitiveResponse,
        V: PrimitiveResponse,
        R: KeyValueCollectionResponse<F, V>,
    {
        prepare_command(self, cmd("HGETALL").arg(key))
    }

    /// Get and delete the value of one or more fields of a given hash key.
    ///
    /// When the last field is deleted, the key will also be deleted.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `fields` - The fields to get and delete.
    ///
    /// # Return
    /// A list of deleted fields and their values or nil for fields that do not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hgetdel/>](https://redis.io/commands/hgetdel/)
    #[must_use]
    fn hgetdel<K, F, FF, RV, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        RV: PrimitiveResponse + DeserializeOwned,
        R: CollectionResponse<RV> + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("HGETDEL")
                .arg(key)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// Get the value of one or more fields of a given hash key
    /// and optionally set their expiration time or time-to-live (TTL).
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `options` - The [`GetExOptions`](StringCommands::GetOptions) options.
    /// * `fields` - The fields to get.
    ///
    /// # Return
    /// a list of values associated with the given fields, in the same order as they are requested.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hgetex/>](https://redis.io/commands/hgetex/)
    #[must_use]
    fn hgetex<K, F, FF, RV, R>(
        self,
        key: K,
        options: GetExOptions,
        fields: FF,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        RV: PrimitiveResponse + DeserializeOwned,
        R: CollectionResponse<RV> + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("HGETEX")
                .arg(key)
                .arg(options)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// Increments the number stored at field in the hash stored at key by increment.
    ///
    /// # Return
    /// The value at field after the increment operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hincrby/>](https://redis.io/commands/hincrby/)
    #[must_use]
    fn hincrby<K, F>(self, key: K, field: F, increment: i64) -> PreparedCommand<'a, Self, i64>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
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
    fn hincrbyfloat<K, F>(self, key: K, field: F, increment: f64) -> PreparedCommand<'a, Self, f64>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
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
    fn hkeys<K, F, A>(self, key: K) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        F: PrimitiveResponse + DeserializeOwned,
        A: CollectionResponse<F> + DeserializeOwned,
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
    fn hlen<K>(self, key: K) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
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
    fn hmget<K, F, FF, RV, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        RV: PrimitiveResponse + DeserializeOwned,
        R: CollectionResponse<RV> + DeserializeOwned,
    {
        prepare_command(self, cmd("HMGET").arg(key).arg(fields))
    }

    /// Remove the existing expiration on a hash key's field(s),
    /// turning the field(s) from volatile (a field with expiration set)
    /// to persistent (a field that will never expire as no TTL (time to live) is associated).
    ///
    /// # Return
    /// For each field:
    /// * `-2` - if no such field exists in the provided hash key, or the provided key does not exist.
    /// * `-1` - if the field exists but has no associated expiration set.
    /// * `1` - the expiration was removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hpersist/>](https://redis.io/commands/hpersist/)
    #[must_use]
    fn hpersist<K, F, FF, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64> + DeserializeOwned,
    {
        prepare_command(self, cmd("HPERSIST").arg(key).arg(fields))
    }

    /// This command works like [`hexpire`](HashCommands::hexpire), but the expiration of a field is specified in milliseconds instead of seconds.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `milliseconds ` - The expiration time in milliseconds
    /// * `option` - The [`ExpireOption`](commands::ExpireOption) option.
    /// * `fields` - The fields to expire.
    ///
    /// # Return
    /// For each field:
    /// * `-2` - if no such field exists in the provided hash key, or the provided key does not exist.
    /// * `0` - if the specified NX | XX | GT | LT condition has not been met.
    /// * `1` - if the expiration time was set/updated.
    /// * `2` - when the command is called with 0 milliseconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hpexpire/>](https://redis.io/commands/hpexpire/)
    #[must_use]
    fn hpexpire<K, F, FF, R>(
        self,
        key: K,
        milliseconds: u64,
        option: ExpireOption,
        fields: FF,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HPEXPIRE")
                .arg(key)
                .arg(milliseconds)
                .arg(option)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// This command has the same effect and semantics as [`hexpireat`](HashCommands::hexpireat),
    /// but the Unix time at which the field will expire
    /// is specified in milliseconds since Unix epoch instead of seconds.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `unix_time_milliseconds` - The aboslute unix timestamp in milliseconds, the fields will expire at.
    /// * `option` - The [`ExpireOption`](commands::ExpireOption) option.
    /// * `fields` - The fields to expire.
    ///
    /// # Return
    /// For each field:
    /// * `-2` - if no such field exists in the provided hash key, or the provided key does not exist.
    /// * `0` - if the specified NX | XX | GT | LT condition has not been met.
    /// * `1` - if the expiration time was set/updated.
    /// * `2` - when the command is called with a past Unix time in milliseconds.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hpexpireat/>](https://redis.io/commands/hpexpireat/)
    #[must_use]
    fn hpexpireat<K, F, FF, R>(
        self,
        key: K,
        unix_time_milliseconds: u64,
        option: ExpireOption,
        fields: FF,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HPEXPIREAT")
                .arg(key)
                .arg(unix_time_milliseconds)
                .arg(option)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// This command has the same semantics as [`hexpiretime`](HashCommands::hexpiretime),
    /// but returns the absolute Unix expiration timestamp
    /// in milliseconds since Unix epoch instead of seconds.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `fields` - The fields to get the expiration time from.
    ///
    /// # Return
    /// For each field, the expiration (Unix timestamp) in milliseconds.
    /// - The command returns -2 if no such field exists in the provided hash key, or the provided key does not exist.
    /// - The command returns -1 if the field exists but has no associated expiration set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hpexpiretime/>](https://redis.io/commands/hpexpiretime/)
    #[must_use]
    fn hpexpiretime<K, F, FF, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HPEXPIRETIME")
                .arg(key)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// Like [`httl`](HashCommands::httl), this command returns the remaining TTL (time to live)
    /// of a field that has an expiration set, but in milliseconds instead of seconds.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `fields` - The fields to get the TTL from.
    ///
    /// # Return
    /// the TTL in milliseconds.
    /// - The command returns -2 if no such field exists in the provided hash key, or the provided key does not exist.
    /// - The command returns -1 if the field exists but has no associated expiration set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hpttl/>](https://redis.io/commands/hpttl/)
    #[must_use]
    fn hpttl<K, F, FF, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HPTTL")
                .arg(key)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// return random fields from the hash value stored at key.
    ///
    /// # Return
    /// * When called with just the key argument, return a random field from the hash value stored at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hrandfield/>](https://redis.io/commands/hrandfield/)
    #[must_use]
    fn hrandfield<K, F>(self, key: K) -> PreparedCommand<'a, Self, F>
    where
        Self: Sized,
        K: SingleArg,
        F: PrimitiveResponse,
    {
        prepare_command(self, cmd("HRANDFIELD").arg(key))
    }

    /// return random fields from the hash value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct fields.
    ///   The array's length is either count or the hash's number of fields (HLEN), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed to return the same field multiple times.
    ///   In this case, the number of returned fields is the absolute value of the specified count.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hrandfield/>](https://redis.io/commands/hrandfield/)
    #[must_use]
    fn hrandfields<K, F, A>(self, key: K, count: isize) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        F: PrimitiveResponse + DeserializeOwned,
        A: CollectionResponse<F> + DeserializeOwned,
    {
        prepare_command(self, cmd("HRANDFIELD").arg(key).arg(count))
    }

    /// return random fields from the hash value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct fields.
    ///   The array's length is either count or the hash's number of fields (HLEN), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed to return the same field multiple times.
    ///   In this case, the number of returned fields is the absolute value of the specified count.
    ///   The optional WITHVALUES modifier changes the reply so it includes the respective values of the randomly selected hash fields.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hrandfield/>](https://redis.io/commands/hrandfield/)
    #[must_use]
    fn hrandfields_with_values<K, F, V, A>(
        self,
        key: K,
        count: isize,
    ) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        F: PrimitiveResponse,
        V: PrimitiveResponse,
        A: KeyValueCollectionResponse<F, V>,
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
        self,
        key: K,
        cursor: u64,
        options: HScanOptions,
    ) -> PreparedCommand<'a, Self, HScanResult<F, V>>
    where
        Self: Sized,
        K: SingleArg,
        F: PrimitiveResponse + DeserializeOwned,
        V: PrimitiveResponse + DeserializeOwned,
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
    fn hset<K, F, V, I>(self, key: K, items: I) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        V: SingleArg,
        I: KeyValueArgsCollection<F, V>,
    {
        prepare_command(self, cmd("HSET").arg(key).arg(items))
    }

    /// Set the value of one or more fields of a given hash key,
    /// and optionally set their expiration time or time-to-live (TTL).
    ///
    /// # Return
    /// * `true` if all the fields wereset.
    /// * `false` if no fields were set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hsetex/>](https://redis.io/commands/hsetex/)
    #[must_use]
    fn hsetex<K, F, V, I>(
        self,
        key: K,
        condition: HSetExCondition,
        expiration: SetExpiration,
        keep_ttl: bool,
        items: I,
    ) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        V: SingleArg,
        I: KeyValueArgsCollection<F, V>,
    {
        prepare_command(
            self,
            cmd("HSETEX")
                .arg(key)
                .arg(condition)
                .arg(expiration)
                .arg_if(keep_ttl, "KEEPTTL")
                .arg("FIELDS")
                .arg(items.num_args() / 2)
                .arg(items),
        )
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
    fn hsetnx<K, F, V>(self, key: K, field: F, value: V) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        V: SingleArg,
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
    fn hstrlen<K, F>(self, key: K, field: F) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
    {
        prepare_command(self, cmd("HSTRLEN").arg(key).arg(field))
    }

    /// Returns the remaining TTL (time to live) of a hash key's field(s) that have a set expiration.
    /// This introspection capability allows you to check how many seconds
    /// a given hash field will continue to be part of the hash key.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `fields` - The fields to get the TTL from.
    ///
    /// # Return
    /// The TTL in seconds.
    /// - The command returns -2 if no such field exists in the provided hash key, or the provided key does not exist.
    /// - The command returns -1 if the field exists but has no associated expiration set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/httl/>](https://redis.io/commands/httl/)
    #[must_use]
    fn httl<K, F, FF, R>(self, key: K, fields: FF) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        K: SingleArg,
        F: SingleArg,
        FF: SingleArgCollection<F>,
        R: CollectionResponse<i64>,
    {
        prepare_command(
            self,
            cmd("HTTL")
                .arg(key)
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields),
        )
    }

    /// list of values in the hash, or an empty list when key does not exist.
    ///
    /// # Return
    /// The list of values in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hvals/>](https://redis.io/commands/hvals/)
    #[must_use]
    fn hvals<K, V, A>(self, key: K) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        V: PrimitiveResponse + DeserializeOwned,
        A: CollectionResponse<V> + DeserializeOwned,
    {
        prepare_command(self, cmd("HVALS").arg(key))
    }
}

/// Options for the [`hscan`](HashCommands::hscan) command
#[derive(Default)]
pub struct HScanOptions {
    command_args: CommandArgs,
}

impl HScanOptions {
    #[must_use]
    pub fn match_pattern<P: SingleArg>(mut self, match_pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("MATCH").arg(match_pattern).build(),
        }
    }

    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }
}

impl ToArgs for HScanOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`hscan`](HashCommands::hscan) command.
#[derive(Debug, Deserialize)]
pub struct HScanResult<F, V>
where
    F: PrimitiveResponse + DeserializeOwned,
    V: PrimitiveResponse + DeserializeOwned,
{
    pub cursor: u64,
    #[serde(deserialize_with = "deserialize_vec_of_pairs")]
    pub elements: Vec<(F, V)>,
}

/// Condition option for the [`hsetex`](HashCommands::hsetex) command
#[derive(Default)]
pub enum HSetExCondition {
    /// No condition
    #[default]
    None,
    /// Only set the fields if none of them already exist.
    FNX,
    /// Only set the fields if all of them already exist.
    FXX,
}

impl ToArgs for HSetExCondition {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            HSetExCondition::None => {}
            HSetExCondition::FNX => {
                args.arg("FNX");
            }
            HSetExCondition::FXX => {
                args.arg("FXX");
            }
        }
    }
}