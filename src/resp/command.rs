use crate::resp::{CommandArgs, IntoArgs};

#[must_use]
#[inline(always)]
pub fn cmd(name: &'static str) -> Command {
    Command::new(name)
}

#[derive(Debug)]
pub struct Command {
    pub name: &'static str,
    pub args: CommandArgs,
}

impl Command {
    #[must_use]
    #[inline(always)]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            args: CommandArgs::default(),
        }
    }

    #[must_use]
    #[inline(always)]
    pub fn arg<A>(self, arg: A) -> Self
    where
        A: IntoArgs,
    {
        Self {
            name: self.name,
            args: self.args.arg(arg),
        }
    }

    #[must_use]
    #[inline(always)]
    pub fn arg_if<A>(self, condition: bool, arg: A) -> Self
    where
        A: IntoArgs,
    {
        Self {
            name: self.name,
            args: self.args.arg_if(condition, arg),
        }
    }
}
