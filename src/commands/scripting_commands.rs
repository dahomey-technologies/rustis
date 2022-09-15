use crate::{
    cmd,
    resp::{BulkString, FromValue, Value},
    CommandSend, Future, SingleArgOrCollection, IntoArgs,
};

/// A group of Redis commands related to Scripting and Functions
/// # See Also
/// [Redis Scripting and Functions Commands](https://redis.io/commands/?group=scripting)
/// [Scripting with LUA](https://redis.io/docs/manual/programmability/eval-intro/)
/// [Functions](https://redis.io/docs/manual/programmability/functions-intro/)
pub trait ScriptingCommands: CommandSend {
    /// This command copies the value stored at the source key to the destination key.
    ///
    /// # Return
    /// The return value of the script
    /// 
    /// # See Also
    /// [https://redis.io/commands/eval/](https://redis.io/commands/eval/)
    fn eval<S, K, KK, A, AA>(&self, script: S, keys: Option<KK>, args: Option<AA>) -> Future<'_, Value>
    where
        S: Into<BulkString>,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        A: Into<BulkString>,
        AA: SingleArgOrCollection<A>,
    {
        self.send_into(cmd("EVAL").arg(script).arg(keys.num_args()).arg(keys).arg(args))
    }

    /// Evaluate a script from the server's cache by its SHA1 digest.
    /// 
    /// # Return
    /// The return value of the script
    ///
    /// # See Also
    /// [https://redis.io/commands/eval/](https://redis.io/commands/eval/)
    fn evalsha<S, K, KK, A, AA>(&self, sha1: S, keys: Option<KK>, args: Option<AA>) -> Future<'_, Value>
    where
        S: Into<BulkString>,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        A: Into<BulkString>,
        AA: SingleArgOrCollection<A>,
    {
        self.send_into(cmd("EVALSHA").arg(sha1).arg(keys.num_args()).arg(keys).arg(args))
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
