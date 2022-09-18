use std::collections::HashMap;

use crate::{
    cmd,
    resp::{Array, BulkString, FromValue, Value},
    CommandArgs, CommandSend, Future, IntoArgs, Result, SingleArgOrCollection, FlushingMode,
};

/// A group of Redis commands related to Scripting and Functions
/// # See Also
/// [Redis Scripting and Functions Commands](https://redis.io/commands/?group=scripting)
/// [Scripting with LUA](https://redis.io/docs/manual/programmability/eval-intro/)
/// [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
pub trait ScriptingCommands: CommandSend {
    /// Invoke the execution of a server-side Lua script.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [https://redis.io/commands/eval/](https://redis.io/commands/eval/)
    fn eval<R>(&self, builder: CallBuilder) -> Future<'_, R>
    where
        R: FromValue,
    {
        self.send_into(cmd("EVAL").arg(builder))
    }

    /// This is a read-only variant of the [eval](crate::ScriptingCommands::eval)]
    /// command that cannot execute commands that modify data.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [https://redis.io/commands/eval_ro/](https://redis.io/commands/eval_ro/)
    fn eval_readonly<R>(&self, builder: CallBuilder) -> Future<'_, R>
    where
        R: FromValue,
    {
        self.send_into(cmd("EVAL_RO").arg(builder))
    }

    /// Evaluate a script from the server's cache by its SHA1 digest.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [https://redis.io/commands/eval/](https://redis.io/commands/eval/)
    fn evalsha<R>(&self, builder: CallBuilder) -> Future<'_, R>
    where
        R: FromValue,
    {
        self.send_into(cmd("EVALSHA").arg(builder))
    }

    /// This is a read-only variant of the [evalsha](crate::ScriptingCommands::evalsha)
    /// command that cannot execute commands that modify data.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [https://redis.io/commands/evalsha_ro/](https://redis.io/commands/evalsha_ro/)
    fn evalsha_readonly<R>(&self, builder: CallBuilder) -> Future<'_, R>
    where
        R: FromValue,
    {
        self.send_into(cmd("EVALSHA_RO").arg(builder))
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [https://redis.io/commands/fcall/](https://redis.io/commands/fcall/)
    fn fcall<R>(&self, builder: CallBuilder) -> Future<'_, R>
    where
        R: FromValue,
    {
        self.send_into(cmd("FCALL").arg(builder))
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [https://redis.io/commands/fcall-ro/](https://redis.io/commands/fcall_ro/)
    fn fcall_readonly<R>(&self, builder: CallBuilder) -> Future<'_, R>
    where
        R: FromValue,
    {
        self.send_into(cmd("FCALL_RO").arg(builder))
    }

    /// Delete a library and all its functions.
    ///
    /// # See Also
    /// [https://redis.io/commands/function-delete/](https://redis.io/commands/function-delete/)
    fn function_delete<L>(&self, library_name: L) -> Future<'_, ()>
    where
        L: Into<BulkString>,
    {
        self.send_into(cmd("FUNCTION").arg("DELETE").arg(library_name))
    }

    /// Return the serialized payload of loaded libraries.
    /// You can restore the serialized payload later with the
    /// [function_restore](crate::ScriptingCommands::function_restore) command.
    ///
    /// # Return
    /// The serialized payload
    ///
    /// # See Also
    /// [https://redis.io/commands/function-dump/](https://redis.io/commands/function-dump/)
    fn function_dump<P>(&self) -> Future<'_, P>
    where
        P: FromValue,
    {
        self.send_into(cmd("FUNCTION").arg("DUMP"))
    }

    /// Deletes all the libraries.
    ///
    /// # See Also
    /// [https://redis.io/commands/function-flush/](https://redis.io/commands/function-flush/)
    fn function_flush(&self, flushing_mode: FlushingMode) -> Future<'_, ()> {
        self.send_into(cmd("FUNCTION").arg("FLUSH").arg(flushing_mode))
    }

    /// Kill a function that is currently executing.
    ///
    /// # See Also
    /// [https://redis.io/commands/function-kill/](https://redis.io/commands/function-kill/)
    fn function_kill(&self) -> Future<'_, ()> {
        self.send_into(cmd("FUNCTION").arg("KILL"))
    }

    /// Return information about the functions and libraries.
    ///
    /// # See Also
    /// [https://redis.io/commands/function-list/](https://redis.io/commands/function-list/)
    fn function_list<P>(
        &self,
        library_name_pattern: Option<P>,
        with_code: bool,
    ) -> Future<'_, Vec<LibraryInfo>>
    where
        P: Into<BulkString>,
    {
        self.send_into(
            cmd("FUNCTION")
                .arg("LIST")
                .arg(library_name_pattern)
                .arg_if(with_code, "WITHCODE"),
        )
    }

    /// Load a library to Redis.
    ///
    /// # Return
    /// The library name that was loaded
    ///
    /// # See Also
    /// [https://redis.io/commands/function-load/](https://redis.io/commands/function-load/)
    fn function_load<F, L>(&self, replace: bool, function_code: F) -> Future<'_, L>
    where
        F: Into<BulkString>,
        L: FromValue,
    {
        self.send_into(
            cmd("FUNCTION")
                .arg("LOAD")
                .arg_if(replace, "REPLACE")
                .arg(function_code),
        )
    }

    /// Restore libraries from the serialized payload.
    ///
    /// # See Also
    /// [https://redis.io/commands/function-restore/](https://redis.io/commands/function-restore/)
    fn function_restore<P>(
        &self,
        serialized_payload: P,
        policy: FunctionRestorePolicy,
    ) -> Future<'_, ()>
    where
        P: Into<BulkString>,
    {
        self.send_into(
            cmd("FUNCTION")
                .arg("RESTORE")
                .arg(serialized_payload)
                .arg(policy),
        )
    }

    /// Return information about the function that's currently running and information about the available execution engines.
    ///
    /// # See Also
    /// [https://redis.io/commands/function-stats/](https://redis.io/commands/function-stats/)
    fn function_stats(&self) -> Future<'_, FunctionStats> {
        self.send_into(cmd("FUNCTION").arg("STATS"))
    }

    /// Set the debug mode for subsequent scripts executed with EVAL.
    ///
    /// # See Also
    /// [https://redis.io/commands/script-debug/](https://redis.io/commands/script-debug/)
    fn script_debug(&self, debug_mode: ScriptDebugMode) -> Future<'_, ()>
    {
        self.send_into(cmd("SCRIPT").arg("DEBUG").arg(debug_mode))
    }

    /// Returns information about the existence of the scripts in the script cache.
    /// 
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [https://redis.io/commands/script-exists/](https://redis.io/commands/script-exists/)
    fn script_exists<S, C>(&self, sha1s: C) -> Future<'_, Vec<bool>>
    where
        S: Into<BulkString>,
        C: SingleArgOrCollection<S>
    {
        self.send_into(cmd("SCRIPT").arg("EXISTS").arg(sha1s))
    }

    /// Flush the Lua scripts cache.
    ///
    /// # See Also
    /// [https://redis.io/commands/script-flush/](https://redis.io/commands/script-flush/)
    fn script_flush(&self, flushing_mode: FlushingMode) -> Future<'_, ()>
    {
        self.send_into(cmd("SCRIPT").arg("FLUSH").arg(flushing_mode))
    }

    /// Kills the currently executing EVAL script, 
    /// assuming no write operation was yet performed by the script.
    ///
    /// # See Also
    /// [https://redis.io/commands/script-kill/](https://redis.io/commands/script-kill/)
    fn script_kill(&self) -> Future<'_, ()>
    {
        self.send_into(cmd("SCRIPT").arg("KILL"))
    }

    /// Load a script into the scripts cache, without executing it.
    ///
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [https://redis.io/commands/script-load/](https://redis.io/commands/script-load/)
    fn script_load<S, V>(&self, script: S) -> Future<'_, V>
    where
        S: Into<BulkString>,
        V: FromValue,
    {
        self.send_into(cmd("SCRIPT").arg("LOAD").arg(script))
    }
}

/// Builder for calling a script/function for the following commands:
/// * [eval](crate::ScriptingCommands::eval)
/// * [eval_readonly](crate::ScriptingCommands::eval_readonly)
/// * [eval_sha](crate::ScriptingCommands::evalsha)
/// * [evalsha_readonly](crate::ScriptingCommands::evalsha_readonly)
/// * [fcall](crate::ScriptingCommands::fcall)
/// * [fcall_readonly](crate::ScriptingCommands::fcall_readonly)
pub struct CallBuilder {
    command_args: CommandArgs,
    keys_added: bool,
}

impl CallBuilder {
    pub fn script<S: Into<BulkString>>(script: S) -> Self {
        Self {
            command_args: CommandArgs::Single(script.into()),
            keys_added: false,
        }
    }

    pub fn sha1<S: Into<BulkString>>(sha1: S) -> Self {
        Self {
            command_args: CommandArgs::Single(sha1.into()),
            keys_added: false,
        }
    }

    pub fn function<F: Into<BulkString>>(function: F) -> Self {
        Self {
            command_args: CommandArgs::Single(function.into()),
            keys_added: false,
        }
    }

    /// All the keys accessed by the script.
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
    pub fn args<A, C>(self, args: C) -> Self
    where
        A: Into<BulkString>,
        C: SingleArgOrCollection<A>,
    {
        let command_args = if !self.keys_added {
            // numkeys = 0
            self.command_args.arg(0).arg(args)
        } else {
            self.command_args.arg(args)
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

/// Policy option for the [function_restore](crate::ScriptingCommands::function_restore) command.
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
                let (
                    library_name_title,
                    library_name,
                    engine_title,
                    engine,
                    functions_title,
                    functions,
                    library_code_title,
                    library_code,
                ) = value.into::<(
                    String,
                    String,
                    String,
                    String,
                    String,
                    Vec<FunctionInfo>,
                    String,
                    String,
                )>()?;

                if library_name_title != "library_name"
                    || engine_title != "engine"
                    || functions_title != "functions"
                    || library_code_title != "library_code"
                {
                    return Err(crate::Error::Internal(
                        "Cannot parse LibraryInfo".to_owned(),
                    ));
                }

                Ok(Self {
                    library_name,
                    engine,
                    functions,
                    library_code: Some(library_code),
                })
            }
            _ => {
                let (
                    library_name_title,
                    library_name,
                    engine_title,
                    engine,
                    functions_title,
                    functions,
                ) = value.into::<(String, String, String, String, String, Vec<FunctionInfo>)>()?;

                if library_name_title != "library_name"
                    || engine_title != "engine"
                    || functions_title != "functions"
                {
                    return Err(crate::Error::Internal(
                        "Cannot parse LibraryInfo".to_owned(),
                    ));
                }

                Ok(Self {
                    library_name,
                    engine,
                    functions,
                    library_code: None,
                })
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
        let (name_title, name, desc_title, description, flags_title, flags) =
            value.into::<(String, String, String, String, String, Vec<String>)>()?;

        if name_title != "name" || desc_title != "description" || flags_title != "flags" {
            return Err(crate::Error::Internal(
                "Cannot parse FunctionInfo".to_owned(),
            ));
        }

        Ok(Self {
            name,
            description,
            flags,
        })
    }
}

#[derive(Debug)]
pub struct FunctionStats {
    pub running_script: Option<RunningScript>,
    pub engines: HashMap<String, EngineStats>,
}

impl FromValue for FunctionStats {
    fn from_value(value: Value) -> Result<Self> {
        let (running_script_title, running_script, engines_title, engines) =
            value.into::<(String, Option<RunningScript>, String, HashMap<String, EngineStats>)>()?;

        if running_script_title != "running_script" || engines_title != "engines" {
            return Err(crate::Error::Internal(
                "Cannot parse FunctionStat".to_owned(),
            ));
        }

        Ok(Self {
            running_script,
            engines,
        })
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
        let (name_title, name, command_title, command, duration_ms_title, duration_ms) =
            value.into::<(String, String, String, Vec<String>, String, u64)>()?;

        if name_title != "name" || command_title != "command" || duration_ms_title != "duration_ms"
        {
            return Err(crate::Error::Internal(
                "Cannot parse RunningScript".to_owned(),
            ));
        }

        Ok(Self {
            name,
            command,
            duration_ms,
        })
    }
}

#[derive(Debug, Default)]
pub struct EngineStats {
    pub libraries_count: usize,
    pub functions_count: usize,
}

impl FromValue for EngineStats {
    fn from_value(value: Value) -> Result<Self> {
        let (libraries_count_title, libraries_count, functions_count_title, functions_count) =
        value.into::<(String, usize, String, usize)>()?;

        if libraries_count_title != "libraries_count" || functions_count_title != "functions_count"
        {
            return Err(crate::Error::Internal("Cannot parse EngineStat".to_owned()));
        }

        Ok(Self {
            libraries_count,
            functions_count,
        })
    }
}

pub enum ScriptDebugMode {
    Default,
    Yes,
    Sync,
    No
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
