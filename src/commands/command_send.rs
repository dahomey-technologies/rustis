use crate::{
    resp::{FromValue, Value},
    Command, Result,
};
use futures::Future;
use std::pin::Pin;

pub trait CommandSend {
    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `name` - Command name in uppercase.
    /// * `args` - Command arguments which can be provided as arrays (up to 4 elements) or vectors of [BulkString](crate::resp::BulkString).
    ///
    /// # Example
    /// ```ignore
    /// let connection = ConnectionMultiplexer::connect("127.0.0.1:6379").await?;
    /// let database = connection.get_default_database();
    ///
    /// let values: Vec<String> = database
    ///     .send(cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"))
    ///     .await?
    ///     .into()?;
    /// ```
    fn send(&self, command: Command) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>>;

    fn send_into<T: FromValue>(
        &self,
        command: Command,
    ) -> Pin<Box<dyn Future<Output = Result<T>> + Send + '_>> {
        let fut = self.send(command);
        Box::pin(async move { fut.await?.into() })
    }
}
