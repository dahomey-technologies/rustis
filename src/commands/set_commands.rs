use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CommandArgs, PrimitiveResponse, CollectionResponse, IntoArgs, SingleArg, SingleArgCollection,
    },
};
use serde::de::DeserializeOwned;
use std::hash::Hash;

/// A group of Redis commands related to [`Sets`](https://redis.io/docs/data-types/sets/)
/// # See Also
/// [Redis Set Commands](https://redis.io/commands/?group=set)
pub trait SetCommands {
    /// Add the specified members to the set stored at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sadd/>](https://redis.io/commands/sadd/)
    #[must_use]
    fn sadd<K, M, C>(&mut self, key: K, members: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
        prepare_command(self, cmd("SADD").arg(key).arg(members))
    }

    /// Returns the set cardinality (number of elements) of the set stored at key.
    ///
    /// # Return
    /// The cardinality (number of elements) of the set, or 0 if key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/scard/>](https://redis.io/commands/scard/)
    #[must_use]
    fn scard<K>(&mut self, key: K) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("SCARD").arg(key))
    }

    /// Returns the members of the set resulting from the difference
    /// between the first set and all the successive sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sdiff/>](https://redis.io/commands/sdiff/)
    #[must_use]
    fn sdiff<K, M, C, A>(&mut self, keys: C) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + Eq + Hash + DeserializeOwned,
        C: SingleArgCollection<K>,
        A: CollectionResponse<M> + DeserializeOwned,
    {
        prepare_command(self, cmd("SDIFF").arg(keys))
    }

    /// This command is equal to [sdiff](SetCommands::sdiff), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sdiffstore/>](https://redis.io/commands/sdiffstore/)
    #[must_use]
    fn sdiffstore<D, K, C>(&mut self, destination: D, keys: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        prepare_command(self, cmd("SDIFFSTORE").arg(destination).arg(keys))
    }

    /// Returns the members of the set resulting from the intersection of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sinter/>](https://redis.io/commands/sinter/)
    #[must_use]
    fn sinter<K, M, C, A>(&mut self, keys: C) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + Eq + Hash + DeserializeOwned,
        C: SingleArgCollection<K>,
        A: CollectionResponse<M> + DeserializeOwned,
    {
        prepare_command(self, cmd("SINTER").arg(keys))
    }

    /// This command is similar to [sinter](SetCommands::sinter), but instead of returning the result set,
    /// it returns just the cardinality of the result.
    ///
    /// limit: if the intersection cardinality reaches limit partway through the computation,
    /// the algorithm will exit and yield limit as the cardinality. 0 means unlimited
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sintercard/>](https://redis.io/commands/sintercard/)
    #[must_use]
    fn sintercard<K, C>(&mut self, keys: C, limit: usize) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        prepare_command(
            self,
            cmd("SINTERCARD")
                .arg(keys.num_args())
                .arg(keys)
                .arg("LIMIT")
                .arg(limit),
        )
    }

    /// This command is equal to [sinter](SetCommands::sinter), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sinterstore/>](https://redis.io/commands/sinterstore/)
    #[must_use]
    fn sinterstore<D, K, C>(&mut self, destination: D, keys: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        prepare_command(self, cmd("SINTERSTORE").arg(destination).arg(keys))
    }

    /// Returns if member is a member of the set stored at key.
    ///
    /// # Return
    /// * `true` - if the element is a member of the set.
    /// * `false` - if the element is not a member of the set, or if key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sismember/>](https://redis.io/commands/sismember/)
    #[must_use]
    fn sismember<K, M>(&mut self, key: K, member: M) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("SISMEMBER").arg(key).arg(member))
    }

    /// Returns all the members of the set value stored at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/smembers/>](https://redis.io/commands/smembers/)
    #[must_use]
    fn smembers<K, M, A>(&mut self, key: K) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + Eq + Hash + DeserializeOwned,
        A: CollectionResponse<M> + DeserializeOwned,
    {
        prepare_command(self, cmd("SMEMBERS").arg(key))
    }

    /// Returns whether each member is a member of the set stored at key.
    ///
    /// # Return
    /// list representing the membership of the given elements, in the same order as they are requested.
    ///
    /// # See Also
    /// [<https://redis.io/commands/smismember/>](https://redis.io/commands/smismember/)
    #[must_use]
    fn smismember<K, M, C>(&mut self, key: K, members: C) -> PreparedCommand<Self, Vec<bool>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
        prepare_command(self, cmd("SMISMEMBER").arg(key).arg(members))
    }

    /// Move member from the set at source to the set at destination.
    ///
    /// # Return
    /// * `true` - if the element is moved.
    /// * `false` - if the element is not a member of source and no operation was performed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/smove/>](https://redis.io/commands/smove/)
    #[must_use]
    fn smove<S, D, M>(
        &mut self,
        source: S,
        destination: D,
        member: M,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
        S: SingleArg,
        D: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("SMOVE").arg(source).arg(destination).arg(member))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [<https://redis.io/commands/spop/>](https://redis.io/commands/spop/)
    #[must_use]
    fn spop<K, M, A>(&mut self, key: K, count: usize) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + Eq + Hash + DeserializeOwned,
        A: CollectionResponse<M> + DeserializeOwned,
    {
        prepare_command(self, cmd("SPOP").arg(key).arg(count))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [<https://redis.io/commands/srandmember/>](https://redis.io/commands/srandmember/)
    #[must_use]
    fn srandmember<K, M, A>(&mut self, key: K, count: usize) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + Eq + Hash + DeserializeOwned,
        A: CollectionResponse<M> + DeserializeOwned,
    {
        prepare_command(self, cmd("SRANDMEMBER").arg(key).arg(count))
    }

    /// Remove the specified members from the set stored at key.
    ///
    /// # Return
    /// the number of members that were removed from the set, not including non existing members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/srem/>](https://redis.io/commands/srem/)
    #[must_use]
    fn srem<K, M, C>(&mut self, key: K, members: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
        prepare_command(self, cmd("SREM").arg(key).arg(members))
    }

    /// Iterates elements of Sets types.
    ///
    /// # Return
    /// a list of Set members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sscan/>](https://redis.io/commands/sscan/)
    #[must_use]
    fn sscan<K, M>(
        &mut self,
        key: K,
        cursor: u64,
        options: SScanOptions,
    ) -> PreparedCommand<Self, (u64, Vec<M>)>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(self, cmd("SSCAN").arg(key).arg(cursor).arg(options))
    }

    /// Returns the members of the set resulting from the union of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sunion/>](https://redis.io/commands/sunion/)
    #[must_use]
    fn sunion<K, M, C, A>(&mut self, keys: C) -> PreparedCommand<Self, A>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + Eq + Hash + DeserializeOwned,
        C: SingleArgCollection<K>,
        A: CollectionResponse<M> + DeserializeOwned,
    {
        prepare_command(self, cmd("SUNION").arg(keys))
    }

    /// This command is equal to [sunion](SetCommands::sunion), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sunionstore/>](https://redis.io/commands/sunionstore/)
    #[must_use]
    fn sunionstore<D, K, C>(&mut self, destination: D, keys: C) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        prepare_command(self, cmd("SUNIONSTORE").arg(destination).arg(keys))
    }
}

/// Options for the [`sscan`](SetCommands::sscan) command
#[derive(Default)]
pub struct SScanOptions {
    command_args: CommandArgs,
}

impl SScanOptions {
    #[must_use]
    pub fn match_pattern<P: SingleArg>(self, match_pattern: P) -> Self {
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
