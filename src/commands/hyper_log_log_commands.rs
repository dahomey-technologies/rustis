use crate::{cmd, resp::BulkString, CommandResult, PrepareCommand, SingleArgOrCollection};

/// A group of Redis commands related to HyperLogLog
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hyperloglog)
pub trait HyperLogLogCommands<T>: PrepareCommand<T> {
    /// Adds the specified elements to the specified HyperLogLog.
    /// 
    /// # Return
    /// * `true` if at least 1 HyperLogLog internal register was altered. 
    /// * `false` otherwise.
    ///
    /// # See Also
    /// [https://redis.io/commands/pfadd/](https://redis.io/commands/pfadd/)
    fn pfadd<K, E, EE>(&self, key: K, elements: EE) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        EE: SingleArgOrCollection<E>,
    {
        self.prepare_command(cmd("PFADD").arg(key).arg(elements))
    }

    /// Return the approximated cardinality of the set(s) 
    /// observed by the HyperLogLog at key(s).
    /// 
    /// # Return
    /// The approximated number of unique elements observed via PFADD.
    ///
    /// # See Also
    /// [https://redis.io/commands/pfcount/](https://redis.io/commands/pfcount/)
    fn pfcount<K, KK>(&self, keys: KK) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
    {
        self.prepare_command(cmd("PFCOUNT").arg(keys))
    }

    /// Merge N different HyperLogLogs into a single one.
    ///
    /// # See Also
    /// [https://redis.io/commands/pfmerge/](https://redis.io/commands/pfmerge/)
    fn pfmerge<D, S, SS>(&self, dest_key: D, source_keys: SS) -> CommandResult<T, ()>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        SS: SingleArgOrCollection<S>,
    {
        self.prepare_command(cmd("PFMERGE").arg(dest_key).arg(source_keys))
    }
}
