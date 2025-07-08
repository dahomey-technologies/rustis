use crate::resp::{CommandArgs, ToArgs};
#[cfg(debug_assertions)]
use std::{
    hash::{Hash, Hasher},
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(debug_assertions)]
static COMMAND_SEQUENCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Shortcut function for creating a command.
#[must_use]
#[inline(always)]
pub fn cmd(name: &'static str) -> Command {
    Command::new(name)
}

/// Generic command meant to be sent to the Redis Server
#[derive(Debug, Clone, Eq)]
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
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
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
            #[cfg(debug_assertions)]
            command_seq: COMMAND_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
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
    #[inline]
    pub fn kill_connection_on_write(mut self, num_kills: usize) -> Self {
        self.kill_connection_on_write = num_kills;
        self
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.args == other.args
    }
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.args.hash(state);
    }
}
