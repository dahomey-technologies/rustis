use crate::{
    cmd,
    resp::{BulkString, FromValue},
    CommandSend, IntoArgs, Result,
};
use futures::Future;
use std::pin::Pin;

/// A group of Redis commands related to Lists
///
/// # See Also
/// [Redis List Commands](https://redis.io/commands/?group=list)
pub trait ListCommands: CommandSend {
    /// Insert all the specified values at the head of the list stored at key
    ///
    /// # Return
    /// the length of the list after the push operations.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpush/](https://redis.io/commands/lpush/)
    fn lpush<K, E>(&self, key: K, elements: E) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString> + Send,
        E: IntoArgs + Send,
    {
        self.send_into(cmd("LPUSH").arg(key).args(elements))
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// # Return
    /// collection of popped elements, or empty collection when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpop/](https://redis.io/commands/lpop/)
    fn lpop<K, E>(&self, key: K, count: usize) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + '_>>
    where
        K: Into<BulkString> + Send,
        E: FromValue,
    {
        self.send_into(cmd("LPOP").arg(key).arg(count))
    }

    /// Insert all the specified values at the tail of the list stored at key
    ///
    /// # Return
    /// the length of the list after the push operations.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpush/](https://redis.io/commands/rpush/)
    fn rpush<K, E>(&self, key: K, elements: E) -> Pin<Box<dyn Future<Output = Result<i64>> + '_>>
    where
        K: Into<BulkString> + Send,
        E: IntoArgs + Send,
    {
        self.send_into(cmd("RPUSH").arg(key).args(elements))
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// # Return
    /// collection of popped elements, or empty collection when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpop/](https://redis.io/commands/rpop/)
    fn rpop<K, E>(&self, key: K, count: usize) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + '_>>
    where
        K: Into<BulkString> + Send,
        E: FromValue,
    {
        self.send_into(cmd("RPOP").arg(key).arg(count))
    }
}
