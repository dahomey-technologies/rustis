use crate::resp::Command;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// Wrapper around a command about to be send with a marker for the response type
/// and a few options to decide how the response send back by Redis should be processed.
pub struct PreparedCommand<'a, E, R = ()>
where
    R: DeserializeOwned,
{
    /// Marker of the type in which the command response will be transformed
    phantom: PhantomData<fn(&'a ()) -> R>,
    /// Client, Transaction or Pipeline that will actually
    /// send the command to the Redis server.
    pub executor: E,
    /// Command to send
    pub command: Command,
    /// Flag to retry sending the command on network error.
    pub retry_on_error: Option<bool>,
}

impl<'a, E, R> PreparedCommand<'a, E, R>
where
    R: DeserializeOwned,
{
    /// Create a new prepared command.
    #[must_use]
    pub fn new(executor: E, command: Command) -> Self {
        PreparedCommand {
            phantom: PhantomData,
            executor,
            command,
            retry_on_error: None,
        }
    }

    /// Set a flag to override default `retry_on_error` behavior.
    ///
    /// See [Config::retry_on_error](crate::client::Config::retry_on_error)
    pub fn retry_on_error(mut self, retry_on_error: bool) -> Self {
        self.retry_on_error = Some(retry_on_error);
        self
    }

    /// Get a reference to the command to send
    pub fn command(&self) -> &Command {
        &self.command
    }
}

/// Shortcut function to creating a [`PreparedCommand`](PreparedCommand).
///
/// Use it to add a command rustis does not implement, keeping the fluent
/// `client.mycommand("key").await` shape. Every built-in command trait is
/// written this way.
///
/// ```
/// use rustis::{
///     client::{Client, PreparedCommand, prepare_command},
///     resp::cmd,
/// };
/// use serde::Serialize;
///
/// trait MyCommands<'a> {
///     #[must_use]
///     fn myget(self, key: impl Serialize) -> PreparedCommand<'a, Self, String>
///     where
///         Self: Sized,
///     {
///         prepare_command(self, cmd("MYGET").key(key))
///     }
/// }
///
/// // Implement it per executor. Add `&mut Pipeline` and `&mut Transaction` to
/// // make the command queueable into a batch.
/// impl<'a> MyCommands<'a> for &'a Client {}
/// ```
///
/// Add Redis keys with [`CommandBuilder::key`](crate::resp::CommandBuilder::key),
/// everything else with `arg`. Cluster routing reads the keys, so a key passed as
/// an argument is sent to the wrong node.
pub fn prepare_command<'a, E, R: DeserializeOwned>(
    executor: E,
    command: impl Into<Command>,
) -> PreparedCommand<'a, E, R> {
    PreparedCommand::new(executor, command.into())
}
