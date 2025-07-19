use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{SingleArg, SingleArgCollection, cmd},
};

/// A group of Redis commands related to [`HyperLogLog`](https://redis.io/docs/data-types/hyperloglogs/)
///
/// # See Also
/// [Redis Hash Commands](https://redis.io/commands/?group=hyperloglog)
pub trait HyperLogLogCommands<'a> {
    /// Adds the specified elements to the specified HyperLogLog.
    ///
    /// # Return
    /// * `true` if at least 1 HyperLogLog inFternal register was altered.
    /// * `false` otherwise.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pfadd/>](https://redis.io/commands/pfadd/)
    fn pfadd<K, E, EE>(self, key: K, elements: EE) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        EE: SingleArgCollection<E>,
    {
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
    fn pfcount<K, KK>(self, keys: KK) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        KK: SingleArgCollection<K>,
    {
        prepare_command(self, cmd("PFCOUNT").arg(keys))
    }

    /// Merge N different HyperLogLogs into a single one.
    ///
    /// # See Also
    /// [<https://redis.io/commands/pfmerge/>](https://redis.io/commands/pfmerge/)
    fn pfmerge<D, S, SS>(self, dest_key: D, source_keys: SS) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        D: SingleArg,
        S: SingleArg,
        SS: SingleArgCollection<S>,
    {
        prepare_command(self, cmd("PFMERGE").arg(dest_key).arg(source_keys))
    }
}
