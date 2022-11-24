use crate::{
    resp::{Command, FromValue, Value},
    ClientTrait, Future,
};
use std::marker::PhantomData;

type PostProcessFunc<'a, R> = dyn Fn(Value, Command, &'a mut dyn ClientTrait) -> Future<'a, R> + Send + Sync;

pub struct PreparedCommand<'a, T, R>
where
    R: FromValue,
{
    pub phantom: PhantomData<R>,
    pub executor: &'a mut T,
    pub command: Command,
    pub keep_command_for_result: bool,
    pub post_process: Option<Box<PostProcessFunc<'a, R>>>,
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
            keep_command_for_result: false,
            post_process: None,
        }
    }

    pub fn keep_command_for_result(mut self) -> Self {
        self.keep_command_for_result = true;
        self
    }

    pub fn post_process(mut self, post_process: Box<PostProcessFunc<'a, R>>) -> Self {
        self.post_process = Some(post_process);
        self
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
