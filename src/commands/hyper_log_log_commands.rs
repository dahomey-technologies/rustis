use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, cmd},
};

/// A group of Redis commands related to [`HyperLogLog`](https://redis.io/docs/data-types/hyperloglogs/)
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hyperloglog)
pub trait HyperLogLogCommands<'a>: Sized {
    /// Adds the specified elements to the specified HyperLogLog.
    ///
    /// # Return
    /// * `true` if at least 1 HyperLogLog inFternal register was altered.
    /// * `false` otherwise.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pfadd/>](https://redis.io/commands/pfadd/)
    fn pfadd(self, key: impl Args, elements: impl Args) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("PFADD").arg(key).arg(elements))
    }

    /// Return the approximated cardinality of the set(s)
    /// observed by the HyperLogLog at key(s).
    ///
    /// # Return
    /// The approximated number of unique elements observed via PFADD.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pfcount/>](https://redis.io/commands/pfcount/)
    fn pfcount(self, keys: impl Args) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("PFCOUNT").arg(keys))
    }

    /// Merge N different HyperLogLogs into a single one.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pfmerge/>](https://redis.io/commands/pfmerge/)
    fn pfmerge(self, dest_key: impl Args, source_keys: impl Args) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("PFMERGE").arg(dest_key).arg(source_keys))
    }
}
