use crate::{
    resp::{cmd, Array, BulkString, CommandArgs, FromValue, IntoArgs, Value},
    CommandResult, Error, PrepareCommand, Result,
};
use std::collections::HashMap;

/// A group of Redis commands related to connection management
///
/// # See Also
/// [Redis Connection Management Commands](https://redis.io/commands/?group=connection)
pub trait ConnectionCommands<T>: PrepareCommand<T> {
    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushdb/](https://redis.io/commands/flushdb/)
    #[must_use]
    fn hello(&self, options: HelloOptions) -> CommandResult<T, HelloResult> {
        self.prepare_command(cmd("HELLO").arg(options))
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    ///
    /// # See Also
    /// [https://redis.io/commands/ping/](https://redis.io/commands/ping/)
    #[must_use]
    fn ping<M, R>(&self, message: Option<M>) -> CommandResult<T, R>
    where
        M: Into<BulkString>,
        R: FromValue,
    {
        self.prepare_command(cmd("PING").arg(message))
    }

    /// Ask the server to close the connection.
    ///
    /// # See Also
    /// [https://redis.io/commands/quit/](https://redis.io/commands/quit/)
    #[must_use]
    fn quit(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("QUIT"))
    }

    /// This command performs a full reset of the connection's server-side context,
    /// mimicking the effect of disconnecting and reconnecting again.
    ///
    /// # See Also
    /// [https://redis.io/commands/reset/](https://redis.io/commands/reset/)
    #[must_use]
    fn reset(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("RESET"))
    }

    /// Select the Redis logical database having the specified zero-based numeric index.
    ///
    /// # See Also
    /// [https://redis.io/commands/reset/](https://redis.io/commands/reset/)
    #[must_use]
    fn select(&self, index: usize) -> CommandResult<T, ()> {
        self.prepare_command(cmd("SELECT").arg(index))
    }
}

/// Options for the [hello](crate::ConnectionCommands::hello) command.
#[derive(Default)]
pub struct HelloOptions {
    command_args: CommandArgs,
}

impl HelloOptions {
    #[must_use]
    pub fn new(protover: usize) -> Self {
        Self {
            command_args: CommandArgs::default().arg(protover),
        }
    }

    #[must_use]
    pub fn auth<U, P>(self, username: U, password: P) -> Self
    where
        U: Into<BulkString>,
        P: Into<BulkString>,
    {
        Self {
            command_args: self.command_args.arg("AUTH").arg(username).arg(password),
        }
    }

    #[must_use]
    pub fn set_name<C>(self, client_name: C) -> Self
    where
        C: Into<BulkString>,
    {
        Self {
            command_args: self.command_args.arg("SETNAME").arg(client_name),
        }
    }
}

impl IntoArgs for HelloOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

pub struct HelloResult {
    pub server: String,
    pub version: String,
    pub proto: usize,
    pub id: i64,
    pub mode: String,
    pub role: String,
    pub modules: Vec<String>,
}

impl FromValue for HelloResult {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(Array::Vec(v)) if v.len() == 14 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<HelloResult> {
                    Some(HelloResult {
                        server: values.remove("server")?.into().ok()?,
                        version: values.remove("version")?.into().ok()?,
                        proto: values.remove("proto")?.into().ok()?,
                        id: values.remove("id")?.into().ok()?,
                        mode: values.remove("mode")?.into().ok()?,
                        role: values.remove("role")?.into().ok()?,
                        modules: values.remove("modules")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Internal("Cannot parse HelloResult".to_owned()))
            }
            _ => Err(Error::Internal("Cannot parse HelloResult".to_owned())),
        }
    }
}
