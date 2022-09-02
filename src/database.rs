use crate::{
    resp::Value, Command, CommandSend, ConnectionMultiplexer, GenericCommands, HashCommands,
    ListCommands, Result, ScriptingCommands, ServerCommands, SetCommands, StringCommands,
    Transaction,
};
use futures::Future;
use std::pin::Pin;

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

    pub fn create_transaction(&self) -> Transaction {
        Transaction::new(self.clone())
    }
}

impl CommandSend for Database {
    fn send(&self, command: Command) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        Box::pin(self.send(command))
    }
}

impl GenericCommands for Database {}
impl HashCommands for Database {}
impl ListCommands for Database {}
impl ScriptingCommands for Database {}
impl ServerCommands for Database {}
impl SetCommands for Database {}
impl StringCommands for Database {}
