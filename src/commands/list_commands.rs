use crate::{
    cmd,
    resp::{BulkString, FromValue},
    Database, Result, IntoArgs,
};
use async_trait::async_trait;

/// A group of Redis commands related to Lists
///
/// # See Also
/// [Redis List Commands](https://redis.io/commands/?group=list)
#[async_trait]
pub trait ListCommands {
    /// Insert all the specified values at the head of the list stored at key
    /// 
    /// # Return
    /// the length of the list after the push operations.
    /// 
    /// # See Also
    /// [https://redis.io/commands/lpush/](https://redis.io/commands/lpush/)
    async fn lpush<K, E>(&self, key: K, elements: E) -> Result<i64>
    where
        K: Into<BulkString> + Send,
        E: IntoArgs + Send;

    /// Removes and returns the first elements of the list stored at key.
    /// 
    /// # Return
    /// collection of popped elements, or empty collection when key does not exist.
    /// 
    /// # See Also
    /// [https://redis.io/commands/lpop/](https://redis.io/commands/lpop/)
    async fn lpop<K, E>(&self, key: K, count: usize) -> Result<Vec<E>>
    where
        K: Into<BulkString> + Send,
        E: FromValue;
}

#[async_trait]
impl ListCommands for Database {
    async fn lpush<K, E>(&self, key: K, elements: E) -> Result<i64>
    where
        K: Into<BulkString> + Send,
        E: IntoArgs + Send,
    {
        self.send(cmd("LPUSH").arg(key).args(elements)).await?.into()
    }

    async fn lpop<K, E>(&self, key: K, count: usize) -> Result<Vec<E>>
    where
        K: Into<BulkString> + Send,
        E: FromValue,
    {
        self.send(cmd("LPOP").arg(key).arg(count)).await?.into()
    }
}
