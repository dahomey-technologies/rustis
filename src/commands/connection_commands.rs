use crate::{
    cmd,
    resp::{Array, BulkString, FromValue, Value},
    CommandArgs, CommandResult, Error, IntoArgs, IntoCommandResult, Result,
};
use std::collections::HashMap;

/// A group of Redis commands related to connection management
///
/// # See Also
/// [Redis Connection Management Commands](https://redis.io/commands/?group=connection)
pub trait ConnectionCommands<T>: IntoCommandResult<T> {
    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [https://redis.io/commands/flushdb/](https://redis.io/commands/flushdb/)
    fn hello(&self, options: HelloOptions) -> CommandResult<T, HelloResult> {
        self.into_command_result(cmd("HELLO").arg(options))
    }
}

/// Options for the [hello](crate::ConnectionCommands::hello) command.
#[derive(Default)]
pub struct HelloOptions {
    command_args: CommandArgs,
}

impl HelloOptions {
    pub fn new(protover: usize) -> Self {
        Self {
            command_args: CommandArgs::Single(protover.into()),
        }
    }

    pub fn auth<U, P>(self, username: U, password: P) -> Self
    where
        U: Into<BulkString>,
        P: Into<BulkString>,
    {
        Self {
            command_args: self.command_args.arg("AUTH").arg(username).arg(password),
        }
    }

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
                    .ok_or(Error::Internal("Cannot parse HelloResult".to_owned()))
            },
            _ => Err(Error::Internal("Cannot parse HelloResult".to_owned())),
        }
    }
}
