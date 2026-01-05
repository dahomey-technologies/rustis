use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{ExpireOption, GetExOptions, SetExpiration},
    resp::{ArgCounter, Response, cmd, deserialize_vec_of_pairs, serialize_flag},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// A group of Redis commands related to [`Hashes`](https://redis.io/docs/data-types/hashes/)
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hash)
pub trait HashCommands<'a>: Sized {
    /// Removes the specified fields from the hash stored at key.
    ///
    /// # Return
    /// the number of fields that were removed from the hash, not including specified but non existing fields.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hdel/>](https://redis.io/commands/hdel/)
    #[must_use]
    fn hdel(self, key: impl Serialize, fields: impl Serialize) -> PreparedCommand<'a, Self, usize> {
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
    fn hexists(
        self,
        key: impl Serialize,
        field: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("HEXISTS").arg(key).arg(field))
    }

    /// Set an expiration (TTL or time to live) on one or more fields of a given hash key.
    ///
    /// Field(s) will automatically be deleted from the hash key when their TTLs expire.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `seconds ` - The expiration time in seconds
    /// * `option` - The [`ExpireOption`](crate::commands::ExpireOption) option.
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
    fn hexpire<R: Response>(
        self,
        key: impl Serialize,
        seconds: u64,
        option: impl Into<Option<ExpireOption>>,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HEXPIRE")
                .arg(key)
                .arg(seconds)
                .arg(option.into())
                .arg("FIELDS")
                .arg_with_count(fields),
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
    /// * `option` - The [`ExpireOption`](crate::commands::ExpireOption) option.
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
    fn hexpireat<R: Response>(
        self,
        key: impl Serialize,
        unix_time_seconds: u64,
        option: impl Into<Option<ExpireOption>>,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HEXPIREAT")
                .arg(key)
                .arg(unix_time_seconds)
                .arg(option.into())
                .arg("FIELDS")
                .arg_with_count(fields),
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
    fn hexpiretime<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HEXPIRETIME")
                .arg(key)
                .arg("FIELDS")
                .arg_with_count(fields),
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
    fn hget<R: Response>(
        self,
        key: impl Serialize,
        field: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn hgetall<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
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
    fn hgetdel<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HGETDEL").arg(key).arg("FIELDS").arg_with_count(fields),
        )
    }

    /// Get the value of one or more fields of a given hash key
    /// and optionally set their expiration time or time-to-live (TTL).
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `options` - The [`GetExOptions`](crate::commands::GetExOptions) options.
    /// * `fields` - The fields to get.
    ///
    /// # Return
    /// a list of values associated with the given fields, in the same order as they are requested.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hgetex/>](https://redis.io/commands/hgetex/)
    #[must_use]
    fn hgetex<R: Response>(
        self,
        key: impl Serialize,
        options: GetExOptions,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HGETEX")
                .arg(key)
                .arg(options)
                .arg("FIELDS")
                .arg_with_count(fields),
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
    fn hincrby(
        self,
        key: impl Serialize,
        field: impl Serialize,
        increment: i64,
    ) -> PreparedCommand<'a, Self, i64> {
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
    fn hincrbyfloat(
        self,
        key: impl Serialize,
        field: impl Serialize,
        increment: f64,
    ) -> PreparedCommand<'a, Self, f64> {
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
    fn hkeys<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
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
    fn hlen(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
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
    fn hmget<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn hpersist<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("HPERSIST").arg(key).arg(fields))
    }

    /// This command works like [`hexpire`](HashCommands::hexpire), but the expiration of a field is specified in milliseconds instead of seconds.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `milliseconds ` - The expiration time in milliseconds
    /// * `option` - The [`ExpireOption`](crate::commands::ExpireOption) option.
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
    fn hpexpire<R: Response>(
        self,
        key: impl Serialize,
        milliseconds: u64,
        option: impl Into<Option<ExpireOption>>,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HPEXPIRE")
                .arg(key)
                .arg(milliseconds)
                .arg(option.into())
                .arg("FIELDS")
                .arg_with_count(fields),
        )
    }

    /// This command has the same effect and semantics as [`hexpireat`](HashCommands::hexpireat),
    /// but the Unix time at which the field will expire
    /// is specified in milliseconds since Unix epoch instead of seconds.
    ///
    /// # Arguments
    /// * `key` - The hash key
    /// * `unix_time_milliseconds` - The aboslute unix timestamp in milliseconds, the fields will expire at.
    /// * `option` - The [`ExpireOption`](crate::commands::ExpireOption) option.
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
    fn hpexpireat<R: Response>(
        self,
        key: impl Serialize,
        unix_time_milliseconds: u64,
        option: impl Into<Option<ExpireOption>>,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HPEXPIREAT")
                .arg(key)
                .arg(unix_time_milliseconds)
                .arg(option.into())
                .arg("FIELDS")
                .arg_with_count(fields),
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
    fn hpexpiretime<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HPEXPIRETIME")
                .arg(key)
                .arg("FIELDS")
                .arg_with_count(fields),
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
    fn hpttl<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HPTTL").arg(key).arg("FIELDS").arg_with_count(fields),
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
    fn hrandfield<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
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
    fn hrandfields<R: Response>(
        self,
        key: impl Serialize,
        count: isize,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn hrandfields_with_values<R: Response>(
        self,
        key: impl Serialize,
        count: isize,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn hscan<F: Response + DeserializeOwned, V: Response + DeserializeOwned>(
        self,
        key: impl Serialize,
        cursor: u64,
        options: HScanOptions,
    ) -> PreparedCommand<'a, Self, HScanResult<F, V>> {
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
    fn hset(self, key: impl Serialize, items: impl Serialize) -> PreparedCommand<'a, Self, usize> {
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
    fn hsetex(
        self,
        key: impl Serialize,
        condition: impl Into<Option<HSetExCondition>>,
        expiration: impl Into<Option<SetExpiration>>,
        items: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        let mut counter = ArgCounter::default();
        items.serialize(&mut counter).expect("Arg counting failed");

        prepare_command(
            self,
            cmd("HSETEX")
                .arg(key)
                .arg(condition.into())
                .arg(expiration.into())
                .arg("FIELDS")
                .arg(counter.count / 2)
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
    fn hsetnx(
        self,
        key: impl Serialize,
        field: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
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
    fn hstrlen(
        self,
        key: impl Serialize,
        field: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
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
    fn httl<R: Response>(
        self,
        key: impl Serialize,
        fields: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("HTTL").arg(key).arg("FIELDS").arg_with_count(fields),
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
    fn hvals<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("HVALS").arg(key))
    }
}

/// Options for the [`hscan`](HashCommands::hscan) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct HScanOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#match: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    novalues: bool,
}

impl<'a> HScanOptions<'a> {
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
    pub fn no_values(mut self) -> Self {
        self.novalues = true;
        self
    }
}

/// Result for the [`hscan`](HashCommands::hscan) command.
#[derive(Debug, Deserialize)]
pub struct HScanResult<F: Response + DeserializeOwned, V: Response + DeserializeOwned> {
    pub cursor: u64,
    #[serde(deserialize_with = "deserialize_vec_of_pairs")]
    pub elements: Vec<(F, V)>,
}

/// Condition option for the [`hsetex`](HashCommands::hsetex) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HSetExCondition {
    /// Only set the fields if none of them already exist.
    FNX,
    /// Only set the fields if all of them already exist.
    FXX,
}
