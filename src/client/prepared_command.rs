use crate::resp::{Command, Response};
use std::marker::PhantomData;

/// Wrapper around a command about to be send with a marker for the response type
/// and a few options to decide how the response send back by Redis should be processed.
pub struct PreparedCommand<'a, E, R = ()>
where
    R: Response,
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
    R: Response,
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
/// This is the crate's own extension point, and it is public because a
/// downstream crate needs it for the same reason every built-in command trait
/// does: to add a command rustis does not implement while keeping the fluent
/// `client.mycommand("key").await` shape.
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
/// // Implement it for the executors the command should be usable on. `&Client`
/// // sends it directly; `&mut Pipeline` and `&mut Transaction` would let it be
/// // queued into a batch.
/// impl<'a> MyCommands<'a> for &'a Client {}
/// ```
///
/// Use [`CommandBuilder::key`](crate::resp::CommandBuilder::key) for arguments
/// that are Redis keys and `arg` for the rest: cluster routing reads the keys,
/// so a key passed as a plain argument is a command sent to the wrong node.
pub fn prepare_command<'a, E, R: Response>(
    executor: E,
    command: impl Into<Command>,
) -> PreparedCommand<'a, E, R> {
    PreparedCommand::new(executor, command.into())
}
