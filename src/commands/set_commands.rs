use crate::{
    cmd,
    resp::{BulkString, FromSingleValueArray, FromValue},
    CommandArgs, CommandResult, IntoArgs, PrepareCommand, SingleArgOrCollection,
};
use std::hash::Hash;

/// A group of Redis commands related to Sets
/// # See Also
/// [Redis Set Commands](https://redis.io/commands/?group=set)
pub trait SetCommands<T>: PrepareCommand<T> {
    /// Add the specified members to the set stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/sadd/](https://redis.io/commands/sadd/)
    #[must_use]
    fn sadd<K, M, C>(&self, key: K, members: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("SADD").arg(key).arg(members))
    }

    /// Returns the set cardinality (number of elements) of the set stored at key.
    ///
    /// # Return
    /// The cardinality (number of elements) of the set, or 0 if key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/scard/](https://redis.io/commands/scard/)
    #[must_use]
    fn scard<K>(&self, key: K) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("SCARD").arg(key))
    }

    /// Returns the members of the set resulting from the difference
    /// between the first set and all the successive sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sdiff/](https://redis.io/commands/sdiff/)
    #[must_use]
    fn sdiff<K, M, C, A>(&self, keys: C) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        C: SingleArgOrCollection<K>,
        A: FromSingleValueArray<M>,
    {
        self.prepare_command(cmd("SDIFF").arg(keys))
    }

    /// This command is equal to [sdiff](crate::SetCommands::sdiff), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sdiffstore/](https://redis.io/commands/sdiffstore/)
    #[must_use]
    fn sdiffstore<D, K, C>(&self, destination: D, keys: C) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(cmd("SDIFFSTORE").arg(destination).arg(keys))
    }

    /// Returns the members of the set resulting from the intersection of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sinter/](https://redis.io/commands/sinter/)
    #[must_use]
    fn sinter<K, M, C, A>(&self, keys: C) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        C: SingleArgOrCollection<K>,
        A: FromSingleValueArray<M>,
    {
        self.prepare_command(cmd("SINTER").arg(keys))
    }

    /// This command is similar to [sinter](crate::SetCommands::sinter), but instead of returning the result set,
    /// it returns just the cardinality of the result.
    ///
    /// limit: if the intersection cardinality reaches limit partway through the computation,
    /// the algorithm will exit and yield limit as the cardinality. 0 means unlimited
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sintercard/](https://redis.io/commands/sintercard/)
    #[must_use]
    fn sintercard<K, C>(&self, keys: C, limit: usize) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(
            cmd("SINTERCARD")
                .arg(keys.num_args())
                .arg(keys)
                .arg("LIMIT")
                .arg(limit),
        )
    }

    /// This command is equal to [sinter](crate::SetCommands::sinter), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sinterstore/](https://redis.io/commands/sinterstore/)
    #[must_use]
    fn sinterstore<D, K, C>(&self, destination: D, keys: C) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(cmd("SINTERSTORE").arg(destination).arg(keys))
    }

    /// Returns if member is a member of the set stored at key.
    ///
    /// # Return
    /// * `true` - if the element is a member of the set.
    /// * `false` - if the element is not a member of the set, or if key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/sismember/](https://redis.io/commands/sismember/)
    #[must_use]
    fn sismember<K, M>(&self, key: K, member: M) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("SISMEMBER").arg(key).arg(member))
    }

    /// Returns all the members of the set value stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/smembers/](https://redis.io/commands/smembers/)
    #[must_use]
    fn smembers<K, M, A>(&self, key: K) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        A: FromSingleValueArray<M>,
    {
        self.prepare_command(cmd("SMEMBERS").arg(key))
    }

    /// Returns whether each member is a member of the set stored at key.
    ///
    /// # Return
    /// list representing the membership of the given elements, in the same order as they are requested.
    ///
    /// # See Also
    /// [https://redis.io/commands/smismember/](https://redis.io/commands/smismember/)
    #[must_use]
    fn smismember<K, M, C>(&self, key: K, members: C) -> CommandResult<T, Vec<bool>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("SMISMEMBER").arg(key).arg(members))
    }

    /// Move member from the set at source to the set at destination.
    ///
    /// # Return
    /// * `true` - if the element is moved.
    /// * `false` - if the element is not a member of source and no operation was performed.
    ///
    /// # See Also
    /// [https://redis.io/commands/smove/](https://redis.io/commands/smove/)
    #[must_use]
    fn smove<S, D, M>(&self, source: S, destination: D, member: M) -> CommandResult<T, bool>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("SMOVE").arg(source).arg(destination).arg(member))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [https://redis.io/commands/spop/](https://redis.io/commands/spop/)
    #[must_use]
    fn spop<K, M, A>(&self, key: K, count: usize) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        A: FromSingleValueArray<M>,
    {
        self.prepare_command(cmd("SPOP").arg(key).arg(count))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [https://redis.io/commands/srandmember/](https://redis.io/commands/srandmember/)
    #[must_use]
    fn srandmember<K, M, A>(&self, key: K, count: usize) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        A: FromSingleValueArray<M>,
    {
        self.prepare_command(cmd("SRANDMEMBER").arg(key).arg(count))
    }

    /// Remove the specified members from the set stored at key.
    ///
    /// # Return
    /// the number of members that were removed from the set, not including non existing members.
    ///
    /// # See Also
    /// [https://redis.io/commands/srem/](https://redis.io/commands/srem/)
    #[must_use]
    fn srem<K, M, C>(&self, key: K, members: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("SREM").arg(key).arg(members))
    }

    /// Iterates elements of Sets types.
    ///
    /// # Return
    /// a list of Set members.
    ///
    /// # See Also
    /// [https://redis.io/commands/sscan/](https://redis.io/commands/sscan/)
    #[must_use]
    fn sscan<K, M>(
        &self,
        key: K,
        cursor: u64,
        options: SScanOptions,
    ) -> CommandResult<T, (u64, Vec<M>)>
    where
        K: Into<BulkString>,
        M: FromValue,
    {
        self.prepare_command(cmd("SSCAN").arg(key).arg(cursor).arg(options))
    }

    /// Returns the members of the set resulting from the union of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sunion/](https://redis.io/commands/sunion/)
    #[must_use]
    fn sunion<K, M, C, A>(&self, keys: C) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        C: SingleArgOrCollection<K>,
        A: FromSingleValueArray<M>,
    {
        self.prepare_command(cmd("SUNION").arg(keys))
    }

    /// This command is equal to [sunion](crate::SetCommands::sunion), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sunionstore/](https://redis.io/commands/sunionstore/)
    #[must_use]
    fn sunionstore<D, K, C>(&self, destination: D, keys: C) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(cmd("SUNIONSTORE").arg(destination).arg(keys))
    }
}

/// Options for the [`sscan`](crate::SetCommands::sscan) command
#[derive(Default)]
pub struct SScanOptions {
    command_args: CommandArgs,
}

impl SScanOptions {
    #[must_use]
    pub fn match_pattern<P: Into<BulkString>>(self, match_pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("MATCH").arg(match_pattern),
        }
    }

    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }
}

impl IntoArgs for SScanOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
