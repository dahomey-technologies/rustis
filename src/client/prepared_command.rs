use crate::resp::{CommandBuilder, Response};
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
    pub command: CommandBuilder,
    /// Flag to retry sending the command on network error.
    pub retry_on_error: Option<bool>,
}

impl<'a, E, R> PreparedCommand<'a, E, R>
where
    R: Response,
{
    /// Create a new prepared command.
    #[must_use]
    pub fn new(executor: E, command: CommandBuilder) -> Self {
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
}

/// Shortcut function to creating a [`PreparedCommand`](PreparedCommand).
pub(crate) fn prepare_command<'a, E, R: Response>(
    executor: E,
    command: CommandBuilder,
) -> PreparedCommand<'a, E, R> {
    PreparedCommand::new(executor, command)
}

pub(crate) fn prepare_command2<'a, E, R: Response>(
    command: CommandBuilder,
    executor: E,
) -> PreparedCommand<'a, E, R> {
    PreparedCommand::new(executor, command)
}
