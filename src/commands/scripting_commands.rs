use crate::{cmd, resp::{BulkString, Value, FromValue}, Command, CommandSend, Result, IntoArgs};
use futures::Future;
use std::pin::Pin;

/// A group of Redis commands related to Scripting and Functions
/// # See Also
/// [Redis Scripting and Functions Commands](https://redis.io/commands/?group=scripting)
/// [Scripting with LUA](https://redis.io/docs/manual/programmability/eval-intro/)
/// [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
pub trait ScriptingCommands: CommandSend {
    /// This command copies the value stored at the source key to the destination key.
    ///
    /// # See Also
    /// [https://redis.io/commands/eval/](https://redis.io/commands/eval/)
    fn eval<S>(&self, script: S) -> Eval<Self>
    where
        S: Into<BulkString> + Send,
    {
        Eval {
            scripting_commands: &self,
            cmd: cmd("EVAL").arg(script),
            keys_added: false
        }
    }

    /// Evaluate a script from the server's cache by its SHA1 digest.
    /// 
    /// # See Also
    /// [https://redis.io/commands/eval/](https://redis.io/commands/eval/)
    fn evalsha<S>(&self, sha1: S) -> Eval<Self>
    where
        S: Into<BulkString> + Send,
    {
        Eval {
            scripting_commands: &self,
            cmd: cmd("EVALSHA").arg(sha1),
            keys_added: false
        }
    }

    /// Load a script into the scripts cache, without executing it. 
    ///
    /// # See Also
    /// [https://redis.io/commands/script-load/](https://redis.io/commands/script-load/)
    fn script_load<'a, S, V>(&'a self, script: S) -> Pin<Box<dyn Future<Output = Result<V>> + 'a>>
    where
        S: Into<BulkString> + Send,
        V: FromValue + Send + 'a,
    {
        self.send_into(cmd("SCRIPT").arg("LOAD").arg(script))
    }
}

/// Builder for the [eval](crate::ScriptingCommands::eval) command
pub struct Eval<'a, T: ScriptingCommands + ?Sized> {
    scripting_commands: &'a T,
    cmd: Command,
    keys_added: bool
}

impl<'a, T: ScriptingCommands> Eval<'a, T> {
    pub fn new( scripting_commands: &'a T, cmd: Command) -> Self {
        Self {
            scripting_commands,
            cmd,
            keys_added: false,
        }
    }

    /// All the keys accessed by the script.
    pub fn keys<K>(self, keys: K) -> Self
    where
        K: IntoArgs + Send,
    {
        Self {
            scripting_commands: self.scripting_commands,
            cmd: self.cmd.arg(keys.num_args()).args(keys),
            keys_added: true,
        }
    }

    /// Additional input arguments that should not represent names of keys.
    pub fn args<A>(self, args: A) -> Self
    where
        A: IntoArgs + Send,
    {
        let cmd = 
        if !self.keys_added {
            // numkeys = 0
            self.cmd.arg(0).args(args)
        } else {
            self.cmd.args(args)
        };

        Self {
            scripting_commands: self.scripting_commands,
            cmd: cmd,
            keys_added: true,
        }
    }

    /// execute with no option
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<Value>> + 'a>> {
        self.scripting_commands.send_into(self.cmd)
    }
}
