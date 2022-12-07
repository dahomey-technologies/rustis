use crate::{
    client::Client,
    resp::{Command, FromValue, Value},
    Future,
};
use std::marker::PhantomData;

type PostProcessFunc<'a, R> = dyn Fn(Value, Command, &'a mut Client) -> Future<'a, R> + Send + Sync;

/// Wrapper around a command about to be send with a marker for the result type
/// and a few options to decide how the result send back by Redis
pub struct PreparedCommand<'a, T, R>
where
    R: FromValue,
{
    /// Marker of the type in which the command result will be transformed 
    /// with the help of the [`FromValue`](crate::resp::FromValue) trait.
    pub phantom: PhantomData<R>,
    /// Client, Transaction or Pipeline that will actually 
    /// send the command to the Redis server.
    pub executor: &'a mut T,
    /// Command to send
    pub command: Command,
    /// Flag to know if the result will be transformed by 
    /// [`FromValue::from_value_with_command`](crate::resp::FromValue::from_value_with_command)
    /// instead of [`FromValue::from_value`](crate::resp::FromValue::from_value)
    pub keep_command_for_result: bool,
    /// Post process functor te be called instead of 
    /// the [`FromValue`](crate::resp::FromValue) trait.
    pub post_process: Option<Box<PostProcessFunc<'a, R>>>,
}

impl<'a, T, R> PreparedCommand<'a, T, R>
where
    R: FromValue,
{
    /// Create a new prepared command. 
    #[must_use]
    pub fn new(executor: &'a mut T, command: Command) -> Self {
        PreparedCommand {
            phantom: PhantomData,
            executor,
            command,
            keep_command_for_result: false,
            post_process: None,
        }
    }

    /// Set the flag [`self.keep_command_for_result`]
    pub fn keep_command_for_result(mut self) -> Self {
        self.keep_command_for_result = true;
        self
    }

    /// Set the functor [`self.post_process`]
    pub fn post_process(mut self, post_process: Box<PostProcessFunc<'a, R>>) -> Self {
        self.post_process = Some(post_process);
        self
    }

    /// Get a reference to the command to send
    pub fn command(&self) -> &Command {
        &self.command
    }
}

/// Shortcut function to creating a [`PreparedCommand`](PreparedCommand).
pub(crate) fn prepare_command<T, R: FromValue>(
    executor: &mut T,
    command: Command,
) -> PreparedCommand<T, R> {
    PreparedCommand::new(executor, command)
}
