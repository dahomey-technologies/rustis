use crate::resp::{CommandArgs, IntoArgs};

/// Shortcut function for creating a command.
#[must_use]
#[inline(always)]
pub fn cmd(name: &'static str) -> Command {
    Command::new(name)
}

/// Generic command meant to be sent to the Redis Server
#[derive(Debug, Clone)]
pub struct Command {
    /// Name of the command.
    /// 
    /// Note: Sub commands are expressed as the first argument of the command.
    /// 
    /// e.g. `cmd("CONFIG").arg("SET").arg("hash-max-listpack-entries").arg("1024")`
    pub name: &'static str,
    /// Collection of arguments of the command.
    pub args: CommandArgs,
}

impl Command {
    /// Creates an new command.
    /// 
    /// [`cmd`](crate::resp::cmd) function can be used as a shortcut.
    #[must_use]
    #[inline(always)]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            args: CommandArgs::default(),
        }
    }

    /// Builder function to add an argument to an existing command.
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

    /// Builder function to add an argument to an existing command, only if a condition is `true`.
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
