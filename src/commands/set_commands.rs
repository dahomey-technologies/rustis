use crate::{
    cmd,
    resp::{BulkString, FromValue},
    Command, CommandSend, SingleArgOrCollection, Result,
};
use futures::Future;
use std::{collections::HashSet, hash::Hash, pin::Pin};

/// A group of Redis commands related to Sets
/// # See Also
/// [Redis Set Commands](https://redis.io/commands/?group=set)
pub trait SetCommands: CommandSend {
    /// Add the specified members to the set stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/sadd/](https://redis.io/commands/sadd/)
    fn sadd<K, M, C>(&self, key: K, members: C) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.send_into(cmd("SADD").arg(key).arg(members))
    }

    /// Returns the set cardinality (number of elements) of the set stored at key.
    ///
    /// # Return
    /// The cardinality (number of elements) of the set, or 0 if key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/scard/](https://redis.io/commands/scard/)
    fn scard<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("SCARD").arg(key))
    }

    /// Returns the members of the set resulting from the difference
    /// between the first set and all the successive sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sdiff/](https://redis.io/commands/sdiff/)
    fn sdiff<K, M, C>(&self, keys: C) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(cmd("SDIFF").arg(keys))
    }

    /// This command is equal to [sdiff](crate::SetCommands::sdiff), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sdiffstore/](https://redis.io/commands/sdiffstore/)
    fn sdiffstore<D, K, C>(
        &self,
        destination: D,
        keys: C,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(cmd("SDIFFSTORE").arg(destination).arg(keys))
    }

    /// Returns the members of the set resulting from the intersection of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sinter/](https://redis.io/commands/sinter/)
    fn sinter<K, M, C>(&self, keys: C) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(cmd("SINTER").arg(keys))
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
    fn sintercard<K, C>(
        &self,
        keys: C,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(
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
    fn sinterstore<D, K, C>(
        &self,
        destination: D,
        keys: C,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(cmd("SINTERSTORE").arg(destination).arg(keys))
    }

    /// Returns if member is a member of the set stored at key.
    ///
    /// # Return
    /// * `true` - if the element is a member of the set.
    /// * `false` - if the element is not a member of the set, or if key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/sismember/](https://redis.io/commands/sismember/)
    fn sismember<K, M>(&self, key: K, member: M) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("SISMEMBER").arg(key).arg(member))
    }

    /// Returns all the members of the set value stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/smembers/](https://redis.io/commands/smembers/)
    fn smembers<K, M>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
    {
        self.send_into(cmd("SMEMBERS").arg(key))
    }

    /// Returns whether each member is a member of the set stored at key.
    ///
    /// # Return
    /// list representing the membership of the given elements, in the same order as they are requested.
    ///
    /// # See Also
    /// [https://redis.io/commands/smismember/](https://redis.io/commands/smismember/)
    fn smismember<K, M, C>(
        &self,
        key: K,
        members: C,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<bool>>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.send_into(cmd("SMISMEMBER").arg(key).arg(members))
    }

    /// Move member from the set at source to the set at destination.
    ///
    /// # Return
    /// * `true` - if the element is moved.
    /// * `false` - if the element is not a member of source and no operation was performed.
    ///
    /// # See Also
    /// [https://redis.io/commands/smove/](https://redis.io/commands/smove/)
    fn smove<S, D, M>(
        &self,
        source: S,
        destination: D,
        member: M,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + '_>>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("SMOVE").arg(source).arg(destination).arg(member))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [https://redis.io/commands/spop/](https://redis.io/commands/spop/)
    fn spop<K, M>(
        &self,
        key: K,
        count: usize,
    ) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
    {
        self.send_into(cmd("SPOP").arg(key).arg(count))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [https://redis.io/commands/srandmember/](https://redis.io/commands/srandmember/)
    fn srandmember<K, M>(
        &self,
        key: K,
        count: usize,
    ) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
    {
        self.send_into(cmd("SRANDMEMBER").arg(key).arg(count))
    }

    /// Remove the specified members from the set stored at key.
    ///
    /// # Return
    /// the number of members that were removed from the set, not including non existing members.
    ///
    /// # See Also
    /// [https://redis.io/commands/srem/](https://redis.io/commands/srem/)
    fn srem<K, M, C>(&self, key: K, members: C) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>
    {
        self.send_into(cmd("SREM").arg(key).arg(members))
    }

    /// Iterates elements of Sets types.
    ///
    /// # Return
    /// a list of Set members.
    ///
    /// # See Also
    /// [https://redis.io/commands/sscan/](https://redis.io/commands/sscan/)
    fn sscan<K>(&self, key: K, cursor: u64) -> SScan<Self>
    where
        K: Into<BulkString>,
    {
        SScan {
            set_commands: self,
            cmd: cmd("SSCAN").arg(key).arg(cursor),
        }
    }

    /// Returns the members of the set resulting from the union of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sunion/](https://redis.io/commands/sunion/)
    fn sunion<K, M, C>(&self, keys: C) -> Pin<Box<dyn Future<Output = Result<HashSet<M>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue + Eq + Hash,
        C: SingleArgOrCollection<K>
    {
        self.send_into(cmd("SUNION").arg(keys))
    }

    /// This command is equal to [sunion](crate::SetCommands::sunion), but instead of returning the resulting set,
    /// it is stored in destination.
    ///
    /// # Return
    /// The number of elements in the resulting set.
    ///
    /// # See Also
    /// [https://redis.io/commands/sunionstore/](https://redis.io/commands/sunionstore/)
    fn sunionstore<D, K, C>(
        &self,
        destination: D,
        keys: C,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>
    {
        self.send_into(cmd("SUNIONSTORE").arg(destination).arg(keys))
    }
}

/// Builder for the [sscan](crate::SetCommands::sscan) command
pub struct SScan<'a, T: SetCommands + ?Sized> {
    set_commands: &'a T,
    cmd: Command,
}

impl<'a, T: SetCommands + ?Sized> SScan<'a, T> {
    pub fn execute<M>(self) -> Pin<Box<dyn Future<Output = Result<(u64, Vec<M>)>> + 'a>>
    where
        M: FromValue + Eq + Hash,
    {
        self.set_commands.send_into(self.cmd)
    }

    pub fn match_<P>(self, pattern: P) -> Self
    where
        P: Into<BulkString>,
    {
        Self {
            set_commands: self.set_commands,
            cmd: self.cmd.arg("MATCH").arg(pattern),
        }
    }

    pub fn count(self, count: usize) -> Self {
        Self {
            set_commands: self.set_commands,
            cmd: self.cmd.arg("COUNT").arg(count),
        }
    }
}
