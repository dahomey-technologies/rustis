use crate::{resp::Value, Command, ConnectionMultiplexer, Result};
use futures::Future;

#[derive(Clone)]
pub struct Database {
    multiplexer: ConnectionMultiplexer,
    db: usize,
}

/// Set of Redis commands related to a specific database
impl Database {
    pub(crate) fn new(multiplexer: ConnectionMultiplexer, db: usize) -> Self {
        Self { multiplexer, db }
    }

    /// The numeric identifier of this database
    pub fn get_database(&self) -> usize {
        self.db
    }

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
    /// using redis::{cmd, ConnectionMultiplexer};
    ///
    /// let connection = ConnectionMultiplexer::connect("127.0.0.1:6379").await?;
    /// let database = connection.get_default_database();
    ///
    /// let values: Vec<String> = database
    ///     .send(cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"))
    ///     .await?
    ///     .into()?;
    /// ```
    pub fn send<'a>(&'a self, command: Command) -> impl Future<Output = Result<Value>> + 'a {
        self.multiplexer.send(self.db, command)
    }
}
