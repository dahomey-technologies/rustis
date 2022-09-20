use crate::{
    resp::{FromValue, Value},
    BitmapCommands, Command, CommandResult, ConnectionMultiplexer, DatabaseResult, GenericCommands,
    GeoCommands, HashCommands, IntoCommandResult, ListCommands, Result, ScriptingCommands,
    ServerCommands, SetCommands, SortedSetCommands, StringCommands, Transaction,
    TransactionResult0,
};

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
    /// ```
    /// use redis_driver::{cmd, ConnectionMultiplexer, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let connection = ConnectionMultiplexer::connect("127.0.0.1:6379").await?;
    ///     let database = connection.get_default_database();
    ///
    ///    let values: Vec<String> = database
    ///         .send(cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"))
    ///         .await?
    ///         .into()?;
    ///     println!("{:?}", values);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn send<'a>(
        &'a self,
        command: Command,
    ) -> impl futures::Future<Output = Result<Value>> + 'a {
        self.multiplexer.send(self.db, command)
    }

    pub fn send_and_forget<'a>(
        &'a self,
        command: Command,
    ) -> impl futures::Future<Output = Result<()>> + 'a {
        self.multiplexer.send_and_forget(self.db, command)
    }

    pub async fn create_transaction(&self) -> Result<Transaction<TransactionResult0>> {
        Transaction::initialize(self.clone()).await
    }
}

impl IntoCommandResult<DatabaseResult> for Database {
    fn into_command_result<R: FromValue>(
        &self,
        command: Command,
    ) -> CommandResult<DatabaseResult, R> {
        CommandResult::from_database(command, &self)
    }
}

impl BitmapCommands<DatabaseResult> for Database {}
impl GenericCommands<DatabaseResult> for Database {}
impl GeoCommands<DatabaseResult> for Database {}
impl HashCommands<DatabaseResult> for Database {}
impl ListCommands<DatabaseResult> for Database {}
impl ScriptingCommands<DatabaseResult> for Database {}
impl ServerCommands<DatabaseResult> for Database {}
impl SetCommands<DatabaseResult> for Database {}
impl SortedSetCommands<DatabaseResult> for Database {}
impl StringCommands<DatabaseResult> for Database {}
