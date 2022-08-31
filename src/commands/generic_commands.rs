use crate::{cmd, CommandSend, IntoArgs, Result};
use futures::Future;
use std::pin::Pin;

/// A group of generic Redis commands
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=generic)
pub trait GenericCommands: CommandSend {
    /// Removes the specified keys. A key is ignored if it does not exist.
    ///
    /// # Return
    /// The number of keys that were removed.
    ///
    /// # See Also
    /// [Redis Generic Commands](https://redis.io/commands/?group=generic)
    fn del<K>(&self, keys: K) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: IntoArgs + Send,
    {
        self.send_into(cmd("DEL").args(keys))
    }
}
