use std::collections::HashMap;

use crate::{
    resp::{
        cmd, Array, BulkString, CommandArgs, FromValue, IntoArgs, SingleArgOrCollection, Value,
    },
    CommandResult, Error, FlushingMode, PrepareCommand, Result,
};

/// A group of Redis commands related to Scripting and Functions
/// # See Also
/// [Redis Scripting and Functions Commands](https://redis.io/commands/?group=scripting)
/// [Scripting with LUA](https://redis.io/docs/manual/programmability/eval-intro/)
/// [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
pub trait ScriptingCommands<T>: PrepareCommand<T> {
    /// Invoke the execution of a server-side Lua script.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval/>](https://redis.io/commands/eval/)
    #[must_use]
    fn eval<R>(&self, builder: CallBuilder) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.prepare_command(cmd("EVAL").arg(builder))
    }

    /// This is a read-only variant of the [eval](crate::ScriptingCommands::eval)]
    /// command that cannot execute commands that modify data.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval_ro/>](https://redis.io/commands/eval_ro/)
    #[must_use]
    fn eval_readonly<R>(&self, builder: CallBuilder) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.prepare_command(cmd("EVAL_RO").arg(builder))
    }

    /// Evaluate a script from the server's cache by its SHA1 digest.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval/>](https://redis.io/commands/eval/)
    #[must_use]
    fn evalsha<R>(&self, builder: CallBuilder) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.prepare_command(cmd("EVALSHA").arg(builder))
    }

    /// This is a read-only variant of the [evalsha](crate::ScriptingCommands::evalsha)
    /// command that cannot execute commands that modify data.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/evalsha_ro/>](https://redis.io/commands/evalsha_ro/)
    #[must_use]
    fn evalsha_readonly<R>(&self, builder: CallBuilder) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.prepare_command(cmd("EVALSHA_RO").arg(builder))
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [<https://redis.io/commands/fcall/>](https://redis.io/commands/fcall/)
    #[must_use]
    fn fcall<R>(&self, builder: CallBuilder) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.prepare_command(cmd("FCALL").arg(builder))
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [<https://redis.io/commands/fcall-ro/>](https://redis.io/commands/fcall_ro/)
    #[must_use]
    fn fcall_readonly<R>(&self, builder: CallBuilder) -> CommandResult<T, R>
    where
        R: FromValue,
    {
        self.prepare_command(cmd("FCALL_RO").arg(builder))
    }

    /// Delete a library and all its functions.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-delete/>](https://redis.io/commands/function-delete/)
    #[must_use]
    fn function_delete<L>(&self, library_name: L) -> CommandResult<T, ()>
    where
        L: Into<BulkString>,
    {
        self.prepare_command(cmd("FUNCTION").arg("DELETE").arg(library_name))
    }

    /// Return the serialized payload of loaded libraries.
    /// You can restore the serialized payload later with the
    /// [`function_restore`](crate::ScriptingCommands::function_restore) command.
    ///
    /// # Return
    /// The serialized payload
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-dump/>](https://redis.io/commands/function-dump/)
    #[must_use]
    fn function_dump<P>(&self) -> CommandResult<T, P>
    where
        P: FromValue,
    {
        self.prepare_command(cmd("FUNCTION").arg("DUMP"))
    }

    /// Deletes all the libraries.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-flush/>](https://redis.io/commands/function-flush/)
    #[must_use]
    fn function_flush(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FUNCTION").arg("FLUSH").arg(flushing_mode))
    }

    /// Kill a function that is currently executing.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-kill/>](https://redis.io/commands/function-kill/)
    #[must_use]
    fn function_kill(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FUNCTION").arg("KILL"))
    }

    /// Return information about the functions and libraries.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-list/>](https://redis.io/commands/function-list/)
    #[must_use]
    fn function_list(&self, options: FunctionListOptions) -> CommandResult<T, Vec<LibraryInfo>> {
        self.prepare_command(cmd("FUNCTION").arg("LIST").arg(options))
    }

    /// Load a library to Redis.
    ///
    /// # Return
    /// The library name that was loaded
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-load/>](https://redis.io/commands/function-load/)
    #[must_use]
    fn function_load<F, L>(&self, replace: bool, function_code: F) -> CommandResult<T, L>
    where
        F: Into<BulkString>,
        L: FromValue,
    {
        self.prepare_command(
            cmd("FUNCTION")
                .arg("LOAD")
                .arg_if(replace, "REPLACE")
                .arg(function_code),
        )
    }

    /// Restore libraries from the serialized payload.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-restore/>](https://redis.io/commands/function-restore/)
    #[must_use]
    fn function_restore<P>(
        &self,
        serialized_payload: P,
        policy: FunctionRestorePolicy,
    ) -> CommandResult<T, ()>
    where
        P: Into<BulkString>,
    {
        self.prepare_command(
            cmd("FUNCTION")
                .arg("RESTORE")
                .arg(serialized_payload)
                .arg(policy),
        )
    }

    /// Return information about the function that's currently running and information about the available execution engines.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-stats/>](https://redis.io/commands/function-stats/)
    #[must_use]
    fn function_stats(&self) -> CommandResult<T, FunctionStats> {
        self.prepare_command(cmd("FUNCTION").arg("STATS"))
    }

    /// Set the debug mode for subsequent scripts executed with EVAL.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-debug/>](https://redis.io/commands/script-debug/)
    #[must_use]
    fn script_debug(&self, debug_mode: ScriptDebugMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("SCRIPT").arg("DEBUG").arg(debug_mode))
    }

    /// Returns information about the existence of the scripts in the script cache.
    ///
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-exists/>](https://redis.io/commands/script-exists/)
    #[must_use]
    fn script_exists<S, C>(&self, sha1s: C) -> CommandResult<T, Vec<bool>>
    where
        S: Into<BulkString>,
        C: SingleArgOrCollection<S>,
    {
        self.prepare_command(cmd("SCRIPT").arg("EXISTS").arg(sha1s))
    }

    /// Flush the Lua scripts cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-flush/>](https://redis.io/commands/script-flush/)
    #[must_use]
    fn script_flush(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("SCRIPT").arg("FLUSH").arg(flushing_mode))
    }

    /// Kills the currently executing EVAL script,
    /// assuming no write operation was yet performed by the script.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-kill/>](https://redis.io/commands/script-kill/)
    #[must_use]
    fn script_kill(&self) -> CommandResult<T, ()> {
        self.prepare_command(cmd("SCRIPT").arg("KILL"))
    }

    /// Load a script into the scripts cache, without executing it.
    ///
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-load/>](https://redis.io/commands/script-load/)
    #[must_use]
    fn script_load<S, V>(&self, script: S) -> CommandResult<T, V>
    where
        S: Into<BulkString>,
        V: FromValue,
    {
        self.prepare_command(cmd("SCRIPT").arg("LOAD").arg(script))
    }
}

/// Builder for calling a script/function for the following commands:
/// * [`eval`](crate::ScriptingCommands::eval)
/// * [`eval_readonly`](crate::ScriptingCommands::eval_readonly)
/// * [`eval_sha`](crate::ScriptingCommands::evalsha)
/// * [`evalsha_readonly`](crate::ScriptingCommands::evalsha_readonly)
/// * [`fcall`](crate::ScriptingCommands::fcall)
/// * [`fcall_readonly`](crate::ScriptingCommands::fcall_readonly)
pub struct CallBuilder {
    command_args: CommandArgs,
    keys_added: bool,
}

impl CallBuilder {
    #[must_use]
    pub fn script<S: Into<BulkString>>(script: S) -> Self {
        Self {
            command_args: CommandArgs::default().arg(script),
            keys_added: false,
        }
    }

    #[must_use]
    pub fn sha1<S: Into<BulkString>>(sha1: S) -> Self {
        Self {
            command_args: CommandArgs::default().arg(sha1),
            keys_added: false,
        }
    }

    #[must_use]
    pub fn function<F: Into<BulkString>>(function: F) -> Self {
        Self {
            command_args: CommandArgs::default().arg(function),
            keys_added: false,
        }
    }

    /// All the keys accessed by the script.
    #[must_use]
    pub fn keys<K, C>(self, keys: C) -> Self
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        Self {
            command_args: self.command_args.arg(keys.num_args()).arg(keys),
            keys_added: true,
        }
    }

    /// Additional input arguments that should not represent names of keys.
    #[must_use]
    pub fn args<A, C>(self, args: C) -> Self
    where
        A: Into<BulkString>,
        C: SingleArgOrCollection<A>,
    {
        let command_args = if self.keys_added {
            self.command_args.arg(args)
        } else {
            // numkeys = 0
            self.command_args.arg(0).arg(args)
        };

        Self {
            command_args,
            keys_added: true,
        }
    }
}

impl IntoArgs for CallBuilder {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Policy option for the [`function_restore`](crate::ScriptingCommands::function_restore) command.
pub enum FunctionRestorePolicy {
    /// Append
    Default,
    /// Appends the restored libraries to the existing libraries and aborts on collision.
    /// This is the default policy.
    Append,
    /// Deletes all existing libraries before restoring the payload.
    Flush,
    /// appends the restored libraries to the existing libraries,
    /// replacing any existing ones in case of name collisions.
    /// Note that this policy doesn't prevent function name collisions, only libraries.
    Replace,
}

impl Default for FunctionRestorePolicy {
    fn default() -> Self {
        Self::Default
    }
}

impl IntoArgs for FunctionRestorePolicy {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            FunctionRestorePolicy::Default => args,
            FunctionRestorePolicy::Append => args.arg("APPEND"),
            FunctionRestorePolicy::Flush => args.arg("FLUSH"),
            FunctionRestorePolicy::Replace => args.arg("REPLACE"),
        }
    }
}

#[derive(Debug)]
pub struct LibraryInfo {
    pub library_name: String,
    pub engine: String,
    pub functions: Vec<FunctionInfo>,
    pub library_code: Option<String>,
}

impl FromValue for LibraryInfo {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(Array::Vec(v)) if v.len() == 8 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<LibraryInfo> {
                    Some(LibraryInfo {
                        library_name: values.remove("library_name")?.into().ok()?,
                        engine: values.remove("engine")?.into().ok()?,
                        functions: values.remove("functions")?.into().ok()?,
                        library_code: values.remove("library_code")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse LibraryInfo".to_owned()))
            }
            _ => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<LibraryInfo> {
                    Some(LibraryInfo {
                        library_name: values.remove("library_name")?.into().ok()?,
                        engine: values.remove("engine")?.into().ok()?,
                        functions: values.remove("functions")?.into().ok()?,
                        library_code: None,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse LibraryInfo".to_owned()))
            }
        }
    }
}

#[derive(Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub description: String,
    pub flags: Vec<String>,
}

impl FromValue for FunctionInfo {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(Array::Vec(v)) if v.len() == 6 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<FunctionInfo> {
                    Some(FunctionInfo {
                        name: values.remove("name")?.into().ok()?,
                        description: values.remove("description")?.into().ok()?,
                        flags: values.remove("flags")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse FunctionInfo".to_owned()))
            }
            _ => Err(Error::Client("Cannot parse FunctionInfo".to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct FunctionStats {
    pub running_script: Option<RunningScript>,
    pub engines: HashMap<String, EngineStats>,
}

impl FromValue for FunctionStats {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(Array::Vec(v)) if v.len() == 4 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<FunctionStats> {
                    Some(FunctionStats {
                        running_script: values.remove("running_script")?.into().ok()?,
                        engines: values.remove("engines")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse FunctionStats".to_owned()))
            }
            _ => Err(Error::Client("Cannot parse FunctionStats".to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct RunningScript {
    pub name: String,
    pub command: Vec<String>,
    pub duration_ms: u64,
}

impl FromValue for RunningScript {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(Array::Vec(v)) if v.len() == 6 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<RunningScript> {
                    Some(RunningScript {
                        name: values.remove("name")?.into().ok()?,
                        command: values.remove("command")?.into().ok()?,
                        duration_ms: values.remove("duration_ms")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse RunningScript".to_owned()))
            }
            _ => Err(Error::Client("Cannot parse RunningScript".to_owned())),
        }
    }
}

#[derive(Debug, Default)]
pub struct EngineStats {
    pub libraries_count: usize,
    pub functions_count: usize,
}

impl FromValue for EngineStats {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(Array::Vec(v)) if v.len() == 4 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<EngineStats> {
                    Some(EngineStats {
                        libraries_count: values.remove("libraries_count")?.into().ok()?,
                        functions_count: values.remove("functions_count")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse EngineStats".to_owned()))
            }
            _ => Err(Error::Client("Cannot parse EngineStats".to_owned())),
        }
    }
}

pub enum ScriptDebugMode {
    Default,
    Yes,
    Sync,
    No,
}

impl IntoArgs for ScriptDebugMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ScriptDebugMode::Default => args,
            ScriptDebugMode::Yes => args.arg("YES"),
            ScriptDebugMode::Sync => args.arg("SYNC"),
            ScriptDebugMode::No => args.arg("NO"),
        }
    }
}

/// Options for the [`function_list`](crate::ScriptingCommands::function_list) command
#[derive(Default)]
pub struct FunctionListOptions {
    command_args: CommandArgs,
}

impl FunctionListOptions {
    #[must_use]
    pub fn library_name_pattern<P: Into<BulkString>>(self, library_name_pattern: P) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LIBRARYNAME")
                .arg(library_name_pattern),
        }
    }

    #[must_use]
    pub fn with_code(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHCODE"),
        }
    }
}

impl IntoArgs for FunctionListOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
