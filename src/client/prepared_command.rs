use crate::{
    client::Client,
    resp::{Command, RespBuf, Response},
    Future,
};
use std::marker::PhantomData;

type CustomConverter<'a, R> =
    dyn Fn(RespBuf, Command, &'a Client) -> Future<'a, R> + Send + Sync;

/// Wrapper around a command about to be send with a marker for the response type
/// and a few options to decide how the response send back by Redis should be processed.
pub struct PreparedCommand<'a, E, R = ()>
where
    R: Response,
{
    /// Marker of the type in which the command response will be transformed
    phantom: PhantomData<R>,
    /// Client, Transaction or Pipeline that will actually
    /// send the command to the Redis server.
    pub executor: E,
    /// Command to send
    pub command: Command,
    /// Custom converter to transform a RESP Buffer in to `R` type
    pub custom_converter: Option<Box<CustomConverter<'a, R>>>,
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
            custom_converter: None,
            retry_on_error: None,
        }
    }

    /// Set the functor [`self.custom_converter`]
    pub fn custom_converter(mut self, custom_converter: Box<CustomConverter<'a, R>>) -> Self {
        self.custom_converter = Some(custom_converter);
        self
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
pub(crate) fn prepare_command<'a, E, R: Response>(executor: E, command: Command) -> PreparedCommand<'a, E, R> {
    PreparedCommand::new(executor, command)
}
