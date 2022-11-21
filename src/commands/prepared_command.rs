use crate::resp::{Command, FromValue};
use std::marker::PhantomData;

pub struct PreparedCommand<'a, T, R>
where
    R: FromValue,
{
    pub phantom: PhantomData<R>,
    pub executor: &'a mut T,
    pub command: Command,
}

impl<'a, T, R> PreparedCommand<'a, T, R>
where
    R: FromValue,
{
    #[must_use]
    pub fn new(executor: &'a mut T, command: Command) -> Self {
        PreparedCommand {
            phantom: PhantomData,
            executor,
            command,
        }
    }

    pub fn command(&self) -> &Command {
        &self.command
    }
}

pub(crate) fn prepare_command<T, R: FromValue>(
    executor: &mut T,
    command: Command,
) -> PreparedCommand<T, R> {
    PreparedCommand::new(executor, command)
}
