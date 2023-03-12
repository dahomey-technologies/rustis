use crate::resp::{CommandArgs, ToArgs};

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
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub kill_connection_on_write: usize,
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
            #[cfg(debug_assertions)]
            kill_connection_on_write: 0,
        }
    }

    /// Builder function to add an argument to an existing command.
    #[must_use]
    #[inline(always)]
    pub fn arg<A>(mut self, arg: A) -> Self
    where
        A: ToArgs,
    {
        arg.write_args(&mut self.args);
        self
    }

    /// Builder function to add an argument to an existing command, only if a condition is `true`.
    #[must_use]
    #[inline(always)]
    pub fn arg_if<A>(mut self, condition: bool, arg: A) -> Self
    where
        A: ToArgs,
    {
        if condition {
            arg.write_args(&mut self.args); 
        }
        self
    }

    #[cfg(debug_assertions)]
    pub fn kill_connection_on_write(mut self, num_kills: usize) -> Self {
        self.kill_connection_on_write = num_kills;
        self
    }
}
