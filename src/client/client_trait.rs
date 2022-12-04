use crate::{
    client::{Cache, Pipeline, Transaction},
    resp::{Command, Value},
    Future, Result,
};

/// Interface that brings together common features for [`Client`](crate::client::Client)
/// and [`MultiplexedClient`](crate::client::MultiplexedClient)
pub trait ClientTrait: Send {
    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    fn send(&mut self, command: Command) -> Future<Value>;

    /// Send command to the Redis server and forget its response.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    fn send_and_forget(&mut self, command: Command) -> Result<()>;

    /// Send a batch of commands to the Redis server.
    ///
    /// # Arguments
    /// * `commands` - batch of generic [`Command`](crate::resp::Command)s meant to be sent to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    fn send_batch(&mut self, commands: Vec<Command>) -> Future<Value>;

    /// Create a new transaction
    fn create_pipeline(&mut self) -> Pipeline;

    /// Create a new transaction
    fn create_transaction(&mut self) -> Transaction;

    /// Get the cache which gives generic access to attach any state to the client instance
    fn get_cache(&mut self) -> &mut Cache;
}
