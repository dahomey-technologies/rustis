use crate::{
    client::{PreparedCommand, prepare_command},
    commands::FlushingMode,
    resp::{
        CommandArgs, PrimitiveResponse, Response, SingleArg, SingleArgCollection, ToArgs, cmd,
        deserialize_byte_buf,
    },
};
use serde::Deserialize;
use std::collections::HashMap;

/// A group of Redis commands related to Scripting and Functions
/// # See Also
/// [Redis Scripting and Functions Commands](https://redis.io/commands/?group=scripting)
/// [Scripting with LUA](https://redis.io/docs/manual/programmability/eval-intro/)
/// [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
pub trait ScriptingCommands<'a> {
    /// Invoke the execution of a server-side Lua script.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval/>](https://redis.io/commands/eval/)
    #[must_use]
    fn eval<R>(self, builder: CallBuilder) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        R: Response,
    {
        prepare_command(self, cmd("EVAL").arg(builder))
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
    fn eval_readonly<R>(self, builder: CallBuilder) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        R: Response,
    {
        prepare_command(self, cmd("EVAL_RO").arg(builder))
    }

    /// Evaluate a script from the server's cache by its SHA1 digest.
    ///
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [<https://redis.io/commands/eval/>](https://redis.io/commands/eval/)
    #[must_use]
    fn evalsha<R>(self, builder: CallBuilder) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        R: Response,
    {
        prepare_command(self, cmd("EVALSHA").arg(builder))
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
    fn evalsha_readonly<R>(self, builder: CallBuilder) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        R: Response,
    {
        prepare_command(self, cmd("EVALSHA_RO").arg(builder))
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [<https://redis.io/commands/fcall/>](https://redis.io/commands/fcall/)
    #[must_use]
    fn fcall<R>(self, builder: CallBuilder) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        R: Response,
    {
        prepare_command(self, cmd("FCALL").arg(builder))
    }

    /// Invoke a function.
    ///
    /// # Return
    /// The return value of the function
    ///
    /// # See Also
    /// [<https://redis.io/commands/fcall-ro/>](https://redis.io/commands/fcall_ro/)
    #[must_use]
    fn fcall_readonly<R>(self, builder: CallBuilder) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        R: Response,
    {
        prepare_command(self, cmd("FCALL_RO").arg(builder))
    }

    /// Delete a library and all its functions.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-delete/>](https://redis.io/commands/function-delete/)
    #[must_use]
    fn function_delete<L>(self, library_name: L) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        L: SingleArg,
    {
        prepare_command(self, cmd("FUNCTION").arg("DELETE").arg(library_name))
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
    fn function_dump(self) -> PreparedCommand<'a, Self, FunctionDumpResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FUNCTION").arg("DUMP"))
    }

    /// Deletes all the libraries.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-flush/>](https://redis.io/commands/function-flush/)
    #[must_use]
    fn function_flush(self, flushing_mode: FlushingMode) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FUNCTION").arg("FLUSH").arg(flushing_mode))
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
    fn function_help(self) -> PreparedCommand<'a, Self, Vec<String>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FUNCTION").arg("HELP"))
    }

    /// Kill a function that is currently executing.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-kill/>](https://redis.io/commands/function-kill/)
    #[must_use]
    fn function_kill(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FUNCTION").arg("KILL"))
    }

    /// Return information about the functions and libraries.
    ///
    /// # See Also
    /// [<https://redis.io/commands/function-list/>](https://redis.io/commands/function-list/)
    #[must_use]
    fn function_list(
        self,
        options: FunctionListOptions,
    ) -> PreparedCommand<'a, Self, Vec<LibraryInfo>>
    where
        Self: Sized,
    {
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
    fn function_load<F, L>(self, replace: bool, function_code: F) -> PreparedCommand<'a, Self, L>
    where
        Self: Sized,
        F: SingleArg,
        L: PrimitiveResponse,
    {
        prepare_command(
            self,
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
        self,
        serialized_payload: P,
        policy: FunctionRestorePolicy,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        P: SingleArg,
    {
        prepare_command(
            self,
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
    fn function_stats(self) -> PreparedCommand<'a, Self, FunctionStats>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FUNCTION").arg("STATS"))
    }

    /// Set the debug mode for subsequent scripts executed with EVAL.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-debug/>](https://redis.io/commands/script-debug/)
    #[must_use]
    fn script_debug(self, debug_mode: ScriptDebugMode) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
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
    fn script_exists<S, C>(self, sha1s: C) -> PreparedCommand<'a, Self, Vec<bool>>
    where
        Self: Sized,
        S: SingleArg,
        C: SingleArgCollection<S>,
    {
        prepare_command(self, cmd("SCRIPT").arg("EXISTS").arg(sha1s))
    }

    /// Flush the Lua scripts cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-flush/>](https://redis.io/commands/script-flush/)
    #[must_use]
    fn script_flush(self, flushing_mode: FlushingMode) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SCRIPT").arg("FLUSH").arg(flushing_mode))
    }

    /// Kills the currently executing EVAL script,
    /// assuming no write operation was yet performed by the script.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-kill/>](https://redis.io/commands/script-kill/)
    #[must_use]
    fn script_kill(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SCRIPT").arg("KILL"))
    }

    /// Load a script into the scripts cache, without executing it.
    ///
    /// # Return
    /// The SHA1 digest of the script added into the script cache.
    ///
    /// # See Also
    /// [<https://redis.io/commands/script-load/>](https://redis.io/commands/script-load/)
    #[must_use]
    fn script_load<S, V>(self, script: S) -> PreparedCommand<'a, Self, V>
    where
        Self: Sized,
        S: SingleArg,
        V: PrimitiveResponse,
    {
        prepare_command(self, cmd("SCRIPT").arg("LOAD").arg(script))
    }
}

/// Builder for calling a script/function for the following commands:
/// * [`eval`](ScriptingCommands::eval)
/// * [`eval_readonly`](ScriptingCommands::eval_readonly)
/// * [`evalsha`](ScriptingCommands::evalsha)
/// * [`evalsha_readonly`](ScriptingCommands::evalsha_readonly)
/// * [`fcall`](ScriptingCommands::fcall)
/// * [`fcall_readonly`](ScriptingCommands::fcall_readonly)
pub struct CallBuilder {
    command_args: CommandArgs,
    keys_added: bool,
}

impl CallBuilder {
    /// Script name when used with [`eval`](ScriptingCommands::eval)
    /// and [`eval_readonly`](ScriptingCommands::eval_readonly) commands
    #[must_use]
    pub fn script<S: SingleArg>(script: S) -> Self {
        Self {
            command_args: CommandArgs::default().arg(script).build(),
            keys_added: false,
        }
    }

    /// Sha1 haxadecimal string when used with [`eval`](ScriptingCommands::evalsha)
    /// and [`evalsha_readonly`](ScriptingCommands::evalsha_readonly) commands
    #[must_use]
    pub fn sha1<S: SingleArg>(sha1: S) -> Self {
        Self {
            command_args: CommandArgs::default().arg(sha1).build(),
            keys_added: false,
        }
    }

    /// Sha1 haxadecimal string when used with [`fcall`](ScriptingCommands::fcall)
    /// and [`fcall_readonly`](ScriptingCommands::fcall_readonly) commands
    #[must_use]
    pub fn function<F: SingleArg>(function: F) -> Self {
        Self {
            command_args: CommandArgs::default().arg(function).build(),
            keys_added: false,
        }
    }

    /// All the keys accessed by the script.
    #[must_use]
    pub fn keys<K, C>(mut self, keys: C) -> Self
    where
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        Self {
            command_args: self.command_args.arg(keys.num_args()).arg(keys).build(),
            keys_added: true,
        }
    }

    /// Additional input arguments that should not represent names of keys.
    #[must_use]
    pub fn args<A, C>(mut self, args: C) -> Self
    where
        A: SingleArg,
        C: SingleArgCollection<A>,
    {
        let command_args = if self.keys_added {
            self.command_args.arg(args).build()
        } else {
            // numkeys = 0
            self.command_args.arg(0).arg(args).build()
        };

        Self {
            command_args,
            keys_added: true,
        }
    }
}

impl ToArgs for CallBuilder {
    fn write_args(&self, args: &mut CommandArgs) {
        // no keys, no args
        if self.command_args.len() == 1 {
            args.arg(&self.command_args).arg(0);
        } else {
            args.arg(&self.command_args);
        }
    }
}

/// Policy option for the [`function_restore`](ScriptingCommands::function_restore) command.
#[derive(Default)]
pub enum FunctionRestorePolicy {
    /// Append
    #[default]
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

impl ToArgs for FunctionRestorePolicy {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            FunctionRestorePolicy::Default => {}
            FunctionRestorePolicy::Append => {
                args.arg("APPEND");
            }
            FunctionRestorePolicy::Flush => {
                args.arg("FLUSH");
            }
            FunctionRestorePolicy::Replace => {
                args.arg("REPLACE");
            }
        }
    }
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
pub enum ScriptDebugMode {
    /// Enable non-blocking asynchronous debugging of Lua scripts (changes are discarded).
    Yes,
    /// Enable blocking synchronous debugging of Lua scripts (saves changes to data).
    Sync,
    /// Disables scripts debug mode.
    No,
}

impl ToArgs for ScriptDebugMode {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ScriptDebugMode::Yes => args.arg("YES"),
            ScriptDebugMode::Sync => args.arg("SYNC"),
            ScriptDebugMode::No => args.arg("NO"),
        };
    }
}

/// Options for the [`function_list`](ScriptingCommands::function_list) command
#[derive(Default)]
pub struct FunctionListOptions {
    command_args: CommandArgs,
}

impl FunctionListOptions {
    /// specifies a pattern for matching library names.
    #[must_use]
    pub fn library_name_pattern<P: SingleArg>(mut self, library_name_pattern: P) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LIBRARYNAME")
                .arg(library_name_pattern)
                .build(),
        }
    }

    /// will cause the server to include the libraries source implementation in the reply.
    #[must_use]
    pub fn with_code(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHCODE").build(),
        }
    }
}

impl ToArgs for FunctionListOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`function_dump`](ScriptingCommands::function_dump) command.
#[derive(Deserialize)]
pub struct FunctionDumpResult(#[serde(deserialize_with = "deserialize_byte_buf")] pub Vec<u8>);
