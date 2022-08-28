use crate::{cmd, Database, Result, IntoArgs};
use async_trait::async_trait;

/// A group of generic Redis commands
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=generic)
#[async_trait]
pub trait GenericCommands {
    /// Removes the specified keys. A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were removed.
    ///
    /// # See Also
    /// [Redis Generic Commands](https://redis.io/commands/?group=generic)
    async fn del<K>(&self, keys: K) -> Result<usize>
    where
        K: IntoArgs + Send;
}

#[async_trait]
impl GenericCommands for Database {
    async fn del<K>(&self, keys: K) -> Result<usize>
    where
        K: IntoArgs + Send,
    {
        self.send(cmd("DEL").args(keys)).await?.into()
    }
}
