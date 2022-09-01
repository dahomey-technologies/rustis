use crate::{
    cmd,
    resp::{BulkString, FromValue},
    CommandSend, IntoArgs, Result,
};
use futures::Future;
use std::{collections::HashSet, hash::Hash, pin::Pin};

/// A group of Redis commands related to Sets
/// # See Also
/// [Redis Set Commands](https://redis.io/commands/?group=set)
pub trait SetCommands: CommandSend {
    /// Add the specified members to the set stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/sadd/](https://redis.io/commands/sadd/)
    fn sadd<K, M>(&self, key: K, members: M) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString> + Send,
        M: IntoArgs + Send,
    {
        self.send_into(cmd("SADD").arg(key).args(members))
    }

    /// Returns all the members of the set value stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/smembers/](https://redis.io/commands/smembers/)
    fn smembers<K, M>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString> + Send,
        M: FromValue + Send + Eq + Hash,
    {
        self.send_into(cmd("SMEMBERS").arg(key))
    }
}
