use crate::{cmd, resp::{BulkString, FromValue}, CommandSend, Result};
use futures::Future;
use std::{iter::once, pin::Pin};

/// A group of Redis commands related to Hashes
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hash)
pub trait HashCommands: CommandSend {
    /// Returns the value associated with field in the hash stored at key.
    ///
    /// # Return
    /// he value associated with field, or nil when field is not present in the hash or key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/hget/](https://redis.io/commands/hget/)
    fn hget<'a, K, F, V>(&'a self, key: K, field: F) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        K: Into<BulkString> + Send,
        F: Into<BulkString> + Send,
        V: FromValue + Send + 'a,
        Self: Sized,
    {
        self.send_into(cmd("HGET").arg(key).arg(field))
    }

    /// Sets field in the hash stored at key to value.
    ///
    /// # Return
    /// The number of fields that were added.
    ///
    /// # See Also
    /// [https://redis.io/commands/hset/](https://redis.io/commands/hset/)
    fn hset<'a, K, F, V>(
        &'a self,
        key: K,
        items: &'a [(F, V)],
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString> + Send + Sync + Copy,
        F: Into<BulkString> + Send + Sync + Copy,
        V: Into<BulkString> + Send + Sync + Copy,
    {
        let flatten_items: Vec<BulkString> = items
            .iter()
            .flat_map(|i| once(i.0.into()).chain(once(i.1.into())))
            .collect();
        self.send_into(cmd("HSET").arg(key).args(flatten_items))
    }    
}
