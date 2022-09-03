use crate::{
    cmd,
    resp::{BulkString, FromValue, Value},
    Command, CommandSend, Error, IntoArgs, Result,
};
use futures::Future;
use std::{iter::once, pin::Pin};

/// A group of Redis commands related to Hashes
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hash)
pub trait HashCommands: CommandSend {
    /// Removes the specified fields from the hash stored at key.
    ///
    /// # Return
    /// the number of fields that were removed from the hash, not including specified but non existing fields.
    ///
    /// # See Also
    /// [https://redis.io/commands/hdel/](https://redis.io/commands/hdel/)
    fn hdel<K, F>(&self, key: K, fields: F) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString> + Send,
        F: IntoArgs + Send,
    {
        self.send_into(cmd("HDEL").arg(key).args(fields))
    }

    /// Returns if field is an existing field in the hash stored at key.
    ///
    /// # Return
    /// - true if the hash contains field.
    /// - false if the hash does not contain field, or key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hexists/](https://redis.io/commands/hexists/)
    fn hexists<K, F>(&self, key: K, field: F) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        K: Into<BulkString> + Send,
        F: Into<BulkString> + Send,
    {
        self.send_into(cmd("HEXISTS").arg(key).arg(field))
    }

    /// Returns the value associated with field in the hash stored at key.
    ///
    /// # Return
    /// The value associated with field, or nil when field is not present in the hash or key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hget/](https://redis.io/commands/hget/)
    fn hget<'a, K, F, V>(
        &'a self,
        key: K,
        field: F,
    ) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        K: Into<BulkString> + Send,
        F: Into<BulkString> + Send,
        V: FromValue + Send + 'a,
    {
        self.send_into(cmd("HGET").arg(key).arg(field))
    }

    /// Returns all fields and values of the hash stored at key.
    ///
    /// # Return
    /// The list of fields and their values stored in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hgetall/](https://redis.io/commands/hgetall/)
    fn hgetall<'a, K, F, V>(
        &'a self,
        key: K,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(F, V)>>> + 'a>>
    where
        K: Into<BulkString> + Send,
        F: FromValue + Send + 'a,
        V: FromValue + Send + 'a,
    {
        let fut = self.send(cmd("HGETALL").arg(key));
        Box::pin(async move {
            let values: Vec<Value> = fut.await?.into()?;

            let mut result: Vec<(F, V)> = Vec::with_capacity(values.len() / 2);
            let mut it = values.into_iter();
            while let Some(value1) = it.next() {
                if let Some(value2) = it.next() {
                    result.push((value1.into()?, value2.into()?));
                }
            }

            Ok(result)
        })
    }

    /// Increments the number stored at field in the hash stored at key by increment.
    ///
    /// # Return
    /// The value at field after the increment operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/hincrby/](https://redis.io/commands/hincrby/)
    fn hincrby<K, F>(
        &self,
        key: K,
        field: F,
        increment: i64,
    ) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString> + Send,
        F: Into<BulkString> + Send,
    {
        self.send_into(cmd("HINCRBY").arg(key).arg(field).arg(increment))
    }

    /// Increment the specified field of a hash stored at key,
    /// and representing a floating point number, by the specified increment.
    ///
    /// # Return
    /// The value at field after the increment operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/hincrbyfloat/](https://redis.io/commands/hincrbyfloat/)
    fn hincrbyfloat<K, F>(
        &self,
        key: K,
        field: F,
        increment: f64,
    ) -> Pin<Box<dyn Future<Output = Result<f64>> + '_>>
    where
        K: Into<BulkString> + Send,
        F: Into<BulkString> + Send,
    {
        self.send_into(cmd("HINCRBYFLOAT").arg(key).arg(field).arg(increment))
    }

    /// Returns all field names in the hash stored at key.
    ///
    /// # Return
    /// The list of fields in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hkeys/](https://redis.io/commands/hkeys/)
    fn hkeys<'a, K, F>(&'a self, key: K) -> Pin<Box<dyn Future<Output = Result<Vec<F>>> + 'a>>
    where
        K: Into<BulkString> + Send,
        F: FromValue + Send + 'a,
    {
        self.send_into(cmd("HKEYS").arg(key))
    }

    /// Returns the number of fields contained in the hash stored at key.
    ///
    /// # Return
    /// The number of fields in the hash, or 0 when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hlen/](https://redis.io/commands/hlen/)
    fn hlen<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString> + Send,
    {
        self.send_into(cmd("HLEN").arg(key))
    }

    /// Returns the values associated with the specified fields in the hash stored at key.
    ///
    /// # Return
    /// The list of values associated with the given fields, in the same order as they are requested.
    ///
    /// # See Also
    /// [https://redis.io/commands/hmget/](https://redis.io/commands/hmget/)
    fn hmget<'a, K, F, V>(
        &'a self,
        key: K,
        fields: F,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<V>>> + 'a>>
    where
        K: Into<BulkString> + Send,
        F: IntoArgs + Send,
        V: FromValue + Send + 'a,
    {
        self.send_into(cmd("HMGET").arg(key).args(fields))
    }

    /// When called with just the key argument, return a random field from the hash value stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/hrandfield/](https://redis.io/commands/hrandfield/)
    fn hrandfield<K>(&self, key: K) -> HRandField<Self>
    where
        K: Into<BulkString> + Send,
    {
        HRandField {
            hash_commands: &self,
            cmd: cmd("HRANDFIELD").arg(key),
        }
    }

    /// Iterates fields of Hash types and their associated values.
    ///
    /// # Return
    /// array of elements contain two elements, a field and a value,
    /// for every returned element of the Hash.
    ///
    /// # See Also
    /// [https://redis.io/commands/hlen/](https://redis.io/commands/hscan/)
    fn hscan<K>(&self, key: K, cursor: usize) -> HScan<Self>
    where
        K: Into<BulkString> + Send,
    {
        HScan {
            hash_commands: self,
            cmd: cmd("HSCAN").arg(key).arg(cursor),
        }
    }

    /// Sets field in the hash stored at key to value.
    ///
    /// # Return
    /// The number of fields that were added.
    ///
    /// # See Also
    /// [https://redis.io/commands/hset/](https://redis.io/commands/hset/)
    fn hset<K, F, V>(
        &self,
        key: K,
        items: &[(F, V)],
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString> + Send + Sync,
        F: Into<BulkString> + Send + Sync + Clone,
        V: Into<BulkString> + Send + Sync + Clone,
    {
        let flatten_items: Vec<BulkString> = items
            .iter()
            .flat_map(|i| once(i.0.clone().into()).chain(once(i.1.clone().into())))
            .collect();
        self.send_into(cmd("HSET").arg(key).args(flatten_items))
    }

    /// Sets field in the hash stored at key to value, only if field does not yet exist.
    ///
    /// # Return
    /// - *true* if field is a new field in the hash and value was set.
    /// - *false* if field already exists in the hash and no operation was performed.
    ///
    /// # See Also
    /// [https://redis.io/commands/hsetnx/](https://redis.io/commands/hsetnx/)
    fn hsetnx<K, F, V>(
        &self,
        key: K,
        field: F,
        value: V,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        F: Into<BulkString> + Send + Sync + Copy,
        V: Into<BulkString> + Send + Sync + Copy,
    {
        self.send_into(cmd("HSETNX").arg(key).arg(field).arg(value))
    }

    /// Returns the string length of the value associated with field in the hash stored at key.
    ///
    /// # Return
    /// the string length of the value associated with field,
    /// or zero when field is not present in the hash or key does not exist at all.
    ///
    /// # See Also
    /// [https://redis.io/commands/hstrlen/](https://redis.io/commands/hstrlen/)
    fn hstrlen<K, F>(
        &self,
        key: K,
        field: F,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        F: Into<BulkString> + Send + Sync + Copy,
    {
        self.send_into(cmd("HSTRLEN").arg(key).arg(field))
    }

    /// list of values in the hash, or an empty list when key does not exist.
    ///
    /// # Return
    /// The list of values in the hash, or an empty list when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hvals/](https://redis.io/commands/hvals/)
    fn hvals<'a, K, V>(&'a self, key: K) -> Pin<Box<dyn Future<Output = Result<Vec<V>>> + 'a>>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        V: FromValue + Send + 'a,
    {
        self.send_into(cmd("HVALS").arg(key))
    }
}

/// Builder for the [hrandfield](crate::HashCommands::hrandfield) command
pub struct HRandField<'a, T: HashCommands + ?Sized> {
    hash_commands: &'a T,
    cmd: Command,
}

impl<'a, T: HashCommands + ?Sized> HRandField<'a, T> {
    /// return a random field from the hash value stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/hrandfield/](https://redis.io/commands/hrandfield/)
    pub fn execute<F>(self) -> Pin<Box<dyn Future<Output = Result<F>> + 'a>>
    where
        F: FromValue + Send + 'a,
    {
        self.hash_commands.send_into(self.cmd)
    }

    /// If the provided count argument is positive, return an array of distinct fields.
    /// The array's length is either count or the hash's number of fields (HLEN), whichever is lower.
    ///
    /// # See Also
    /// [https://redis.io/commands/hrandfield/](https://redis.io/commands/hrandfield/)
    pub fn count<F>(self, count: i64) -> Pin<Box<dyn Future<Output = Result<Vec<F>>> + 'a>>
    where
        F: FromValue + Send + 'a,
    {
        self.hash_commands.send_into(self.cmd.arg(count))
    }

    /// The optional WITHVALUES modifier changes the reply so it includes
    /// the respective values of the randomly selected hash fields.
    ///
    /// # See Also
    /// [https://redis.io/commands/hrandfield/](https://redis.io/commands/hrandfield/)
    pub fn count_with_values<F, V>(
        self,
        count: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(F, V)>>> + 'a>>
    where
        F: FromValue + Send + 'a,
        V: FromValue + Send + 'a,
    {
        let fut = self
            .hash_commands
            .send(self.cmd.arg(count).arg("WITHVALUES"));
        Box::pin(async move {
            let values: Vec<Value> = fut.await?.into()?;

            let mut result: Vec<(F, V)> = Vec::with_capacity(values.len() / 2);
            let mut it = values.into_iter();
            while let Some(value1) = it.next() {
                if let Some(value2) = it.next() {
                    result.push((value1.into()?, value2.into()?));
                }
            }

            Ok(result)
        })
    }
}

/// Builder for the [hscan](crate::HashCommands::hscan) command
pub struct HScan<'a, T: HashCommands + ?Sized> {
    hash_commands: &'a T,
    cmd: Command,
}

impl<'a, T: HashCommands + ?Sized> HScan<'a, T> {
    /// return a random field from the hash value stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/hrandfield/](https://redis.io/commands/hrandfield/)
    pub fn execute<F, V>(self) -> Pin<Box<dyn Future<Output = Result<HScanResult<F, V>>> + 'a>>
    where
        F: FromValue + Send + 'a,
        V: FromValue + Send + 'a,
    {
        self.hash_commands.send_into(self.cmd)
    }

    pub fn match_<P>(self, pattern: P) -> Self
    where
        P: Into<BulkString> + Send,
    {
        Self {
            hash_commands: self.hash_commands,
            cmd: self.cmd.arg("MATCH").arg(pattern),
        }
    }

    pub fn count(self, count: usize) -> Self
    {
        Self {
            hash_commands: self.hash_commands,
            cmd: self.cmd.arg("COUNT").arg(count),
        }
    }
}

#[derive(Debug)]
pub struct HScanResult<F, V>
where
    F: FromValue,
    V: FromValue,
{
    pub cursor: usize,
    pub fields_and_values: Vec<(F, V)>,
}

impl<F, V> FromValue for HScanResult<F, V>
where
    F: FromValue,
    V: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;

        let mut fields_and_values: Vec<(F, V)> = Vec::with_capacity(values.len() / 2);
        let mut it = values.into_iter();
        let cursor: usize = if let Some(value) = it.next() {
            value.into()?
        } else {
            0
        };

        let values: Vec<Value> = if let Some(value) = it.next() {
            value.into()?
        } else {
            return Err(Error::Internal("unexpected hscan result".to_owned()))
        };

        let mut it = values.into_iter();

        while let Some(value1) = it.next() {
            if let Some(value2) = it.next() {
                fields_and_values.push((value1.into()?, value2.into()?));
            }
        }

        Ok(HScanResult {
            cursor,
            fields_and_values,
        })
    }
}
