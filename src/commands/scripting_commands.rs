use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{FlushingMode, RequestPolicy, ResponsePolicy},
    resp::{BulkString, Response, cmd, serialize_flag},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A group of Redis commands related to Scripting and Functions
/// # See Also
/// [Redis Scripting and Functions Commands](https://redis.io/commands/?group=scripting)
/// [Scripting with LUA](https://redis.io/docs/manual/programmability/eval-intro/)
/// [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
pub trait ScriptingCommands<'a>: Sized {
    /// Invoke the execution of a server-side Lua script.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval/>](https://redis.io/commands/eval/)
    #[must_use]
    fn eval<R: Response>(
        self,
        script: impl Serialize,
        keys: impl Serialize,
        args: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("EVAL").arg(script).key_with_count(keys).arg(args))
    }

    /// This is a read-only variant of the [eval](ScriptingCommands::eval)]
    /// command that cannot execute commands that modify data.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval_ro/>](https://redis.io/commands/eval_ro/)
    #[must_use]
    fn eval_readonly<R: Response>(
        self,
        script: impl Serialize,
        keys: impl Serialize,
        args: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("EVAL_RO").arg(script).key_with_count(keys).arg(args),
        )
    }

    /// Evaluate a script from the server's cache by its SHA1 digest.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval/>](https://redis.io/commands/eval/)
    #[must_use]
    fn evalsha<R: Response>(
        self,
        sha1: impl Serialize,
        keys: impl Serialize,
        args: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("EVALSHA").arg(sha1).key_with_count(keys).arg(args),
        )
    }

    /// This is a read-only variant of the [evalsha](ScriptingCommands::evalsha)
    /// command that cannot execute commands that modify data.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/evalsha_ro/>](https://redis.io/commands/evalsha_ro/)
    #[must_use]
    fn evalsha_readonly<R: Response>(
        self,
        sha1: impl Serialize,
        keys: impl Serialize,
        args: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("EVALSHA_RO").arg(sha1).key_with_count(keys).arg(args),
        )
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [<https://redis.io/commands/fcall/>](https://redis.io/commands/fcall/)
    #[must_use]
    fn fcall<R: Response>(
        self,
        function: impl Serialize,
        keys: impl Serialize,
        args: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("FCALL").arg(function).key_with_count(keys).arg(args),
        )
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [<https://redis.io/commands/fcall-ro/>](https://redis.io/commands/fcall_ro/)
    #[must_use]
    fn fcall_readonly<R: Response>(
        self,
        function: impl Serialize,
        keys: impl Serialize,
        args: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("FCALL_RO").arg(function).key_with_count(keys).arg(args),
        )
    }

    /// Delete a library and all its functions.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-delete/>](https://redis.io/commands/function-delete/)
    #[must_use]
    fn function_delete(self, library_name: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FUNCTION")
                .arg("DELETE")
                .arg(library_name)
                .cluster_info(RequestPolicy::AllShards, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// Return the serialized payload of loaded libraries.
    /// You can restore the serialized payload later with the
    /// [`function_restore`](ScriptingCommands::function_restore) command.
    ///
    /// # Return
    /// The serialized payload
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-dump/>](https://redis.io/commands/function-dump/)
    #[must_use]
    fn function_dump(self) -> PreparedCommand<'a, Self, BulkString> {
        prepare_command(self, cmd("FUNCTION").arg("DUMP"))
    }

    /// Deletes all the libraries.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-flush/>](https://redis.io/commands/function-flush/)
    #[must_use]
    fn function_flush(self, flushing_mode: FlushingMode) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FUNCTION")
                .arg("FLUSH")
                .arg(flushing_mode)
                .cluster_info(RequestPolicy::AllShards, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// The command returns a helpful text describing the different FUNCTION subcommands.
    ///
    /// # Return
    /// An array of strings.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::ScriptingCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// # #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// # async fn main() -> Result<()> {
    /// #     let client = Client::connect("127.0.0.1:6379").await?;
    /// let result: Vec<String> = client.function_help().await?;
    /// assert!(result.iter().any(|e| e == "HELP"));
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-help/>](https://redis.io/commands/function-help/)
    #[must_use]
    fn function_help(self) -> PreparedCommand<'a, Self, Vec<String>> {
        prepare_command(self, cmd("FUNCTION").arg("HELP"))
    }

    /// Kill a function that is currently executing.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-kill/>](https://redis.io/commands/function-kill/)
    #[must_use]
    fn function_kill(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FUNCTION").arg("KILL").cluster_info(
                RequestPolicy::AllShards,
                ResponsePolicy::OneSucceeded,
                1,
            ),
        )
    }

    /// Return information about the functions and libraries.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-list/>](https://redis.io/commands/function-list/)
    #[must_use]
    fn function_list(
        self,
        options: FunctionListOptions,
    ) -> PreparedCommand<'a, Self, Vec<LibraryInfo>> {
        prepare_command(self, cmd("FUNCTION").arg("LIST").arg(options))
    }

    /// Load a library to Redis.
    ///
    /// # Return
    /// The library name that was loaded
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-load/>](https://redis.io/commands/function-load/)
    #[must_use]
    fn function_load<R: Response>(
        self,
        replace: bool,
        function_code: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("FUNCTION")
                .arg("LOAD")
                .arg_if(replace, "REPLACE")
                .arg(function_code)
                .cluster_info(RequestPolicy::AllShards, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// Restore libraries from the serialized payload.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-restore/>](https://redis.io/commands/function-restore/)
    #[must_use]
    fn function_restore(
        self,
        serialized_payload: &BulkString,
        policy: impl Into<Option<FunctionRestorePolicy>>,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FUNCTION")
                .arg("RESTORE")
                .arg(serialized_payload)
                .arg(policy.into())
                .cluster_info(RequestPolicy::AllShards, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// Return information about the function that's currently running and information about the available execution engines.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-stats/>](https://redis.io/commands/function-stats/)
    #[must_use]
    fn function_stats(self) -> PreparedCommand<'a, Self, FunctionStats> {
        prepare_command(
            self,
            cmd("FUNCTION").arg("STATS").cluster_info(
                RequestPolicy::AllShards,
                ResponsePolicy::Special,
                1,
            ),
        )
    }

    /// Set the debug mode for subsequent scripts executed with EVAL.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-debug/>](https://redis.io/commands/script-debug/)
    #[must_use]
    fn script_debug(self, debug_mode: ScriptDebugMode) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("SCRIPT").arg("DEBUG").arg(debug_mode))
    }

    /// Returns information about the existence of the scripts in the script cache.
    ///
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-exists/>](https://redis.io/commands/script-exists/)
    #[must_use]
    fn script_exists(self, sha1s: impl Serialize) -> PreparedCommand<'a, Self, Vec<bool>> {
        prepare_command(
            self,
            cmd("SCRIPT").arg("EXISTS").arg(sha1s).cluster_info(
                RequestPolicy::AllShards,
                ResponsePolicy::AggLogicalAnd,
                1,
            ),
        )
    }

    /// Flush the Lua scripts cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-flush/>](https://redis.io/commands/script-flush/)
    #[must_use]
    fn script_flush(self, flushing_mode: FlushingMode) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("SCRIPT").arg("FLUSH").arg(flushing_mode).cluster_info(
                RequestPolicy::AllNodes,
                ResponsePolicy::AllSucceeded,
                1,
            ),
        )
    }

    /// Kills the currently executing EVAL script,
    /// assuming no write operation was yet performed by the script.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-kill/>](https://redis.io/commands/script-kill/)
    #[must_use]
    fn script_kill(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("SCRIPT").arg("KILL").cluster_info(
                RequestPolicy::AllShards,
                ResponsePolicy::OneSucceeded,
                1,
            ),
        )
    }

    /// Load a script into the scripts cache, without executing it.
    ///
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-load/>](https://redis.io/commands/script-load/)
    #[must_use]
    fn script_load<R: Response>(self, script: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("SCRIPT").arg("LOAD").arg(script).cluster_info(
                RequestPolicy::AllNodes,
                ResponsePolicy::AllSucceeded,
                1,
            ),
        )
    }
}

/// Policy option for the [`function_restore`](ScriptingCommands::function_restore) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FunctionRestorePolicy {
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

/// Result for the [`function_list`](ScriptingCommands::function_list) command.
#[derive(Debug, Deserialize)]
pub struct LibraryInfo {
    /// the name of the library.
    pub library_name: String,
    /// the engine of the library.
    pub engine: String,
    /// the list of functions in the library.
    pub functions: Vec<FunctionInfo>,
    /// the library's source code (when given the
    /// [`with_code`](FunctionListOptions::with_code) modifier).
    pub library_code: Option<String>,
}

/// Sub-result for the [`function_list`](ScriptingCommands::function_list) command.
#[derive(Debug, Deserialize)]
pub struct FunctionInfo {
    /// the name of the function.
    pub name: String,
    /// the function's description.
    pub description: String,
    /// an array of [function flags](https://redis.io/docs/manual/programmability/functions-intro/#function-flags).
    pub flags: Vec<String>,
}

/// Result for the [`function_stats`](ScriptingCommands::function_stats) command.
#[derive(Debug, Deserialize)]
pub struct FunctionStats {
    /// information about the running script. If there's no in-flight function, the server replies with `None`.
    pub running_script: Option<RunningScript>,
    /// Each entry in the map represent a single engine.
    /// Engine map contains statistics about the engine like number of functions and number of libraries.
    pub engines: HashMap<String, EngineStats>,
}

/// Sub-result for the [`function_stats`](ScriptingCommands::function_stats) command.
#[derive(Debug, Deserialize)]
pub struct RunningScript {
    /// the name of the function.
    pub name: String,
    /// the command and arguments used for invoking the function.
    pub command: Vec<String>,
    /// the function's runtime duration in milliseconds.
    pub duration_ms: u64,
}

/// sub-result for the [`function_stats`](ScriptingCommands::function_stats) command.
#[derive(Debug, Default, Deserialize)]
pub struct EngineStats {
    /// Number of libraries of functions
    pub libraries_count: usize,
    /// Number of functions
    pub functions_count: usize,
}

/// Options for the [`script_debug`](ScriptingCommands::script_debug) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ScriptDebugMode {
    /// Enable non-blocking asynchronous debugging of Lua scripts (changes are discarded).
    Yes,
    /// Enable blocking synchronous debugging of Lua scripts (saves changes to data).
    Sync,
    /// Disables scripts debug mode.
    No,
}

/// Options for the [`function_list`](ScriptingCommands::function_list) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct FunctionListOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    libraryname: Option<&'a str>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withcode: bool,
}

impl<'a> FunctionListOptions<'a> {
    /// specifies a pattern for matching library names.
    #[must_use]
    pub fn library_name_pattern(mut self, library_name_pattern: &'a str) -> Self {
        self.libraryname = Some(library_name_pattern);
        self
    }

    /// will cause the server to include the libraries source implementation in the reply.
    #[must_use]
    pub fn with_code(mut self) -> Self {
        self.withcode = true;
        self
    }
}
