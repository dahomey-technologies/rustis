use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Response, cmd},
};
use serde::{Serialize, de::DeserializeOwned};

/// A group of Redis commands related to [`Sets`](https://redis.io/docs/data-types/sets/)
/// # See Also
/// [Redis Set Commands](https://redis.io/commands/?group=set)
pub trait SetCommands<'a>: Sized {
    /// Add the specified members to the set stored at key.
    ///
    /// #Return
    /// The number of elements that were added to the set, not including all the elements already present in the set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sadd/>](https://redis.io/commands/sadd/)
    #[must_use]
    fn sadd(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SADD").key(key).arg(members))
    }

    /// Returns the set cardinality (number of elements) of the set stored at key.
    ///
    /// # Return
    /// The cardinality (number of elements) of the set, or 0 if key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/scard/>](https://redis.io/commands/scard/)
    #[must_use]
    fn scard(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SCARD").key(key))
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
    fn sdiff<R: Response>(self, keys: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SDIFF").key(keys))
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
    fn sdiffstore(
        self,
        destination: impl Serialize,
        keys: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SDIFFSTORE").arg(destination).key(keys))
    }

    /// Returns the members of the set resulting from the intersection of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sinter/>](https://redis.io/commands/sinter/)
    #[must_use]
    fn sinter<R: Response>(self, keys: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SINTER").key(keys))
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
    fn sintercard(self, keys: impl Serialize, limit: usize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("SINTERCARD")
                .key_with_count(keys)
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
    fn sinterstore(
        self,
        destination: impl Serialize,
        keys: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SINTERSTORE").arg(destination).key(keys))
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
    fn sismember(
        self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("SISMEMBER").key(key).arg(member))
    }

    /// Returns all the members of the set value stored at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/smembers/>](https://redis.io/commands/smembers/)
    #[must_use]
    fn smembers<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SMEMBERS").key(key))
    }

    /// Returns whether each member is a member of the set stored at key.
    ///
    /// # Return
    /// list representing the membership of the given elements, in the same order as they are requested.
    ///
    /// # See Also
    /// [<https://redis.io/commands/smismember/>](https://redis.io/commands/smismember/)
    #[must_use]
    fn smismember<R: Response>(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SMISMEMBER").key(key).arg(members))
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
    fn smove(
        self,
        source: impl Serialize,
        destination: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("SMOVE").key(source).key(destination).arg(member))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [<https://redis.io/commands/spop/>](https://redis.io/commands/spop/)
    #[must_use]
    fn spop<R: Response>(self, key: impl Serialize, count: usize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SPOP").key(key).arg(count))
    }

    /// Removes and returns one or more random members from the set value store at key.
    ///
    /// # Return
    /// the list of popped elements
    ///
    /// # See Also
    /// [<https://redis.io/commands/srandmember/>](https://redis.io/commands/srandmember/)
    #[must_use]
    fn srandmember<R: Response>(
        self,
        key: impl Serialize,
        count: usize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SRANDMEMBER").key(key).arg(count))
    }

    /// Remove the specified members from the set stored at key.
    ///
    /// # Return
    /// the number of members that were removed from the set, not including non existing members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/srem/>](https://redis.io/commands/srem/)
    #[must_use]
    fn srem(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SREM").key(key).arg(members))
    }

    /// Iterates elements of Sets types.
    ///
    /// # Return
    /// a list of Set members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sscan/>](https://redis.io/commands/sscan/)
    #[must_use]
    fn sscan<R: Response + DeserializeOwned>(
        self,
        key: impl Serialize,
        cursor: u64,
        options: SScanOptions,
    ) -> PreparedCommand<'a, Self, (u64, R)> {
        prepare_command(self, cmd("SSCAN").key(key).arg(cursor).arg(options))
    }

    /// Returns the members of the set resulting from the union of all the given sets.
    ///
    /// # Return
    /// A list with members of the resulting set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/sunion/>](https://redis.io/commands/sunion/)
    #[must_use]
    fn sunion<R: Response>(self, keys: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("SUNION").key(keys))
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
    fn sunionstore(
        self,
        destination: impl Serialize,
        keys: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("SUNIONSTORE").key(destination).key(keys))
    }
}

/// Options for the [`sscan`](SetCommands::sscan) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct SScanOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#match: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
}

impl<'a> SScanOptions<'a> {
    #[must_use]
    pub fn match_pattern(mut self, match_pattern: &'a str) -> Self {
        self.r#match = Some(match_pattern);
        self
    }

    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
}
