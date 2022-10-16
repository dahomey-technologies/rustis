use crate::{
    resp::{
        cmd, ArgsOrCollection, BulkString, CommandArgs, FromValue, IntoArgs, SingleArgOrCollection,
    },
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to [`Sorted Sets`](https://redis.io/docs/data-types/sorted-sets/)
///
/// # See Also
/// [Redis Sorted Set Commands](https://redis.io/commands/?group=sorted-set)
pub trait SortedSetCommands<T>: PrepareCommand<T> {
    /// This command is the blocking variant of [`zmpop`](crate::SortedSetCommands::zmpop).
    ///
    /// # Return
    /// * `None` if no element could be popped
    /// * A tuple made up of
    ///     * The name of the key from which elements were popped
    ///     * An array of tuples with all the popped members and their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/bzmpop/>](https://redis.io/commands/bzmpop/)
    #[must_use]
    fn bzmpop<K, C, E>(
        &self,
        timeout: f64,
        keys: C,
        where_: ZWhere,
        count: usize,
    ) -> CommandResult<T, Option<ZMPopResult<E>>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue + Default,
    {
        self.prepare_command(
            cmd("BZMPOP")
                .arg(timeout)
                .arg(keys.num_args())
                .arg(keys)
                .arg(where_)
                .arg("COUNT")
                .arg(count),
        )
    }

    /// This command is the blocking variant of [`zpopmax`](crate::SortedSetCommands::zpopmax).
    ///
    /// # Return
    /// * `None` when no element could be popped and the timeout expired.
    /// * The list of tuple with 
    ///     * the first element being the name of the key where a member was popped,
    ///     * the second element is the popped member itself, 
    ///     * and the third element is the score of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bzpopmax/>](https://redis.io/commands/bzpopmax/)
    #[must_use]
    fn bzpopmax<K, KK, E, K1>(&self, keys: KK, timeout: f64) -> CommandResult<T, BZpopMinMaxResult<K1, E>>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue + Default,
        E: FromValue + Default,
    {
        self.prepare_command(cmd("BZPOPMAX").arg(keys).arg(timeout))
    }

    /// This command is the blocking variant of [`zpopmin`](crate::SortedSetCommands::zpopmin).
    ///
    /// # Return
    /// * `None` when no element could be popped and the timeout expired.
    /// * The list of tuple with 
    ///     * the first element being the name of the key where a member was popped,
    ///     * the second element is the popped member itself, 
    ///     * and the third element is the score of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bzpopmin/>](https://redis.io/commands/bzpopmin/)
    #[must_use]
    fn bzpopmin<K, KK, E, K1>(&self, keys: KK, timeout: f64) -> CommandResult<T, BZpopMinMaxResult<K1, E>>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue + Default,
        E: FromValue + Default,
    {
        self.prepare_command(cmd("BZPOPMIN").arg(keys).arg(timeout))
    }

    /// Adds all the specified members with the specified scores
    /// to the sorted set stored at key.
    ///
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the `change` option is specified, the number of elements that were changed (added or updated).
    ///
    /// # See Also
    /// [<https://redis.io/commands/zadd/>](https://redis.io/commands/zadd/)
    #[must_use]
    fn zadd<K, M, I>(&self, key: K, items: I, options: ZAddOptions) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        I: ArgsOrCollection<(f64, M)>,
    {
        self.prepare_command(cmd("ZADD").arg(key).arg(options).arg(items))
    }

    /// In this mode ZADD acts like ZINCRBY.
    /// Only one score-element pair can be specified in this mode.
    ///
    /// # Return
    /// The new score of member (a double precision floating point number),
    /// or nil if the operation was aborted (when called with either the XX or the NX option).
    ///
    /// # See Also
    /// [<https://redis.io/commands/zadd/>](https://redis.io/commands/zadd/)
    #[must_use]
    fn zadd_incr<K, M>(
        &self,
        key: K,
        condition: ZAddCondition,
        comparison: ZAddComparison,
        change: bool,
        score: f64,
        member: M,
    ) -> CommandResult<T, Option<f64>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(
            cmd("ZADD")
                .arg(key)
                .arg(condition)
                .arg(comparison)
                .arg_if(change, "CH")
                .arg(score)
                .arg(member),
        )
    }

    /// Returns the sorted set cardinality (number of elements)
    /// of the sorted set stored at key.
    ///
    /// # Return
    /// The cardinality (number of elements) of the sorted set, or 0 if key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zcard/>](https://redis.io/commands/zcard/)
    #[must_use]
    fn zcard<K>(&self, key: K) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("ZCARD").arg(key))
    }

    /// Returns the number of elements in the sorted set at key with a score between min and max.
    ///
    /// # Return
    /// The number of elements in the specified score range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zcount/>](https://redis.io/commands/zcount/)
    #[must_use]
    fn zcount<K, M1, M2>(&self, key: K, min: M1, max: M2) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M1: Into<BulkString>,
        M2: Into<BulkString>,
    {
        self.prepare_command(cmd("ZCOUNT").arg(key).arg(min).arg(max))
    }

    /// This command is similar to [zdiffstore](crate::SortedSetCommands::zdiffstore), but instead of storing the resulting sorted set,
    /// it is returned to the client.
    ///
    /// # Return
    /// The result of the difference
    ///
    /// # See Also
    /// [<https://redis.io/commands/zdiff/>](https://redis.io/commands/zdiff/)
    #[must_use]
    fn zdiff<K, C, E>(&self, keys: C) -> CommandResult<T, Vec<E>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue,
    {
        self.prepare_command(cmd("ZDIFF").arg(keys.num_args()).arg(keys))
    }

    /// This command is similar to [zdiffstore](crate::SortedSetCommands::zdiffstore), but instead of storing the resulting sorted set,
    /// it is returned to the client.
    ///
    /// # Return
    /// The result of the difference with their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zdiff/>](https://redis.io/commands/zdiff/)
    #[must_use]
    fn zdiff_with_scores<K, C, E>(&self, keys: C) -> CommandResult<T, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue + Default,
    {
        self.prepare_command(
            cmd("ZDIFF")
                .arg(keys.num_args())
                .arg(keys)
                .arg("WITHSCORES"),
        )
    }

    /// Computes the difference between the first and all successive
    /// input sorted sets and stores the result in destination.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set at destination.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zdiffstore/>](https://redis.io/commands/zdiffstore/)
    #[must_use]
    fn zdiffstore<D, K, C>(&self, destination: D, keys: C) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(
            cmd("ZDIFFSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys),
        )
    }

    /// Increments the score of member in the sorted set stored at key by increment.
    ///
    /// # Return
    /// the new score of member
    ///
    /// # See Also
    /// [<https://redis.io/commands/zincrby/>](https://redis.io/commands/zincrby/)
    #[must_use]
    fn zincrby<K, M>(&self, key: K, increment: f64, member: M) -> CommandResult<T, f64>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("ZINCRBY").arg(key).arg(increment).arg(member))
    }

    /// This command is similar to [zinterstore](crate::SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the intersection as an array of members
    ///
    /// # See Also
    /// [<https://redis.io/commands/zinter/>](https://redis.io/commands/zinter/)
    #[must_use]
    fn zinter<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> CommandResult<T, Vec<E>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue,
    {
        self.prepare_command(
            cmd("ZINTER")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }

    /// This command is similar to [zinterstore](crate::SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the intersection as an array of members with their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zinter/>](https://redis.io/commands/zinter/)
    #[must_use]
    fn zinter_with_scores<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> CommandResult<T, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue + Default,
    {
        self.prepare_command(
            cmd("ZINTER")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate)
                .arg("WITHSCORES"),
        )
    }

    /// This command is similar to [zinter](crate::SortedSetCommands::zinter),
    /// but instead of returning the result set, it returns just the cardinality of the result.
    ///
    //// limit: if the intersection cardinality reaches limit partway through the computation,
    /// the algorithm will exit and yield limit as the cardinality. 0 means unlimited
    ///
    /// # See Also
    /// [<https://redis.io/commands/zintercard/>](https://redis.io/commands/zintercard/)
    #[must_use]
    fn zintercard<K, C>(&self, keys: C, limit: usize) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(
            cmd("ZINTERCARD")
                .arg(keys.num_args())
                .arg(keys)
                .arg("LIMIT")
                .arg(limit),
        )
    }

    /// Computes the intersection of numkeys sorted sets given by the specified keys,
    /// and stores the result in destination.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set at destination.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zinterstore/>](https://redis.io/commands/zinterstore/)
    #[must_use]
    fn zinterstore<D, K, C, W>(
        &self,
        destination: D,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
    {
        self.prepare_command(
            cmd("ZINTERSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }

    /// When all the elements in a sorted set are inserted with the same score,
    /// in order to force lexicographical ordering, this command returns the number
    /// of elements in the sorted set at key with a value between min and max.
    ///
    /// # Return
    /// the number of elements in the specified score range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zlexcount/>](https://redis.io/commands/zlexcount/)
    #[must_use]
    fn zlexcount<K, M1, M2>(&self, key: K, min: M1, max: M2) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M1: Into<BulkString>,
        M2: Into<BulkString>,
    {
        self.prepare_command(cmd("ZLEXCOUNT").arg(key).arg(min).arg(max))
    }

    /// Pops one or more elements, that are member-score pairs,
    /// from the first non-empty sorted set in the provided list of key names.
    ///
    /// # Return
    /// * None if no element could be popped
    /// * A tuple made up of
    ///     * The name of the key from which elements were popped
    ///     * An array of tuples with all the popped members and their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zmpop/>](https://redis.io/commands/zmpop/)
    #[must_use]
    fn zmpop<K, C, E>(
        &self,
        keys: C,
        where_: ZWhere,
        count: usize,
    ) -> CommandResult<T, Option<ZMPopResult<E>>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue + Default,
    {
        self.prepare_command(
            cmd("ZMPOP")
                .arg(keys.num_args())
                .arg(keys)
                .arg(where_)
                .arg("COUNT")
                .arg(count),
        )
    }

    /// Returns the scores associated with the specified members in the sorted set stored at key.
    ///
    /// For every member that does not exist in the sorted set, a nil value is returned.
    ///
    /// # Return
    /// The list of scores or nil associated with the specified member value
    ///
    /// # See Also
    /// [<https://redis.io/commands/zmscore/>](https://redis.io/commands/zmscore/)
    #[must_use]
    fn zmscore<K, M, C>(&self, key: K, members: C) -> CommandResult<T, Vec<Option<f64>>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("ZMSCORE").arg(key).arg(members))
    }

    /// Removes and returns up to count members with the highest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zpopmax/>](https://redis.io/commands/zpopmax/)
    #[must_use]
    fn zpopmax<K, M>(&self, key: K, count: usize) -> CommandResult<T, Vec<(M, f64)>>
    where
        K: Into<BulkString>,
        M: FromValue + Default,
    {
        self.prepare_command(cmd("ZPOPMAX").arg(key).arg(count))
    }

    /// Removes and returns up to count members with the lowest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zpopmin/>](https://redis.io/commands/zpopmin/)
    #[must_use]
    fn zpopmin<K, M>(&self, key: K, count: usize) -> CommandResult<T, Vec<(M, f64)>>
    where
        K: Into<BulkString>,
        M: FromValue + Default,
    {
        self.prepare_command(cmd("ZPOPMIN").arg(key).arg(count))
    }

    /// Return a random element from the sorted set value stored at key.
    ///
    /// # Return
    /// The randomly selected element, or nil when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmember<K, E>(&self, key: K) -> CommandResult<T, E>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(cmd("ZRANDMEMBER").arg(key))
    }

    /// Return random elements from the sorted set value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct elements.
    /// The array's length is either count or the sorted set's cardinality (ZCARD), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed
    /// to return the same element multiple times. In this case, the number of returned elements
    /// is the absolute value of the specified count.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmembers<K, E>(&self, key: K, count: isize) -> CommandResult<T, Vec<E>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(cmd("ZRANDMEMBER").arg(key).arg(count))
    }

    /// Return random elements with their scores from the sorted set value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct elements with their scores.
    /// The array's length is either count or the sorted set's cardinality (ZCARD), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed
    /// to return the same element multiple times. In this case, the number of returned elements
    /// is the absolute value of the specified count.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmembers_with_scores<K, E>(&self, key: K, count: isize) -> CommandResult<T, Vec<E>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(cmd("ZRANDMEMBER").arg(key).arg(count).arg("WITHSCORES"))
    }

    /// Returns the specified range of elements in the sorted set stored at `key`.
    ///
    /// # Return
    /// A collection of elements in the specified range
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrange/>](https://redis.io/commands/zrange/)
    #[must_use]
    fn zrange<K, S, E>(
        &self,
        key: K,
        start: S,
        stop: S,
        options: ZRangeOptions,
    ) -> CommandResult<T, Vec<E>>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(cmd("ZRANGE").arg(key).arg(start).arg(stop).arg(options))
    }

    /// Returns the specified range of elements in the sorted set stored at `key`.
    ///
    /// # Return
    /// A collection of elements and their scores in the specified range
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrange/>](https://redis.io/commands/zrange/)
    #[must_use]
    fn zrange_with_scores<K, S, E>(
        &self,
        key: K,
        start: S,
        stop: S,
        options: ZRangeOptions,
    ) -> CommandResult<T, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
        E: FromValue + Default,
    {
        self.prepare_command(
            cmd("ZRANGE")
                .arg(key)
                .arg(start)
                .arg(stop)
                .arg(options)
                .arg("WITHSCORES"),
        )
    }

    /// This command is like [zrange](crate::SortedSetCommands::zrange),
    /// but stores the result in the `dst` destination key.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrangestore/>](https://redis.io/commands/zrangestore/)
    #[must_use]
    fn zrangestore<D, S, SS>(
        &self,
        dst: D,
        src: S,
        start: SS,
        stop: SS,
        options: ZRangeOptions,
    ) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        SS: Into<BulkString>,
    {
        self.prepare_command(
            cmd("ZRANGESTORE")
                .arg(dst)
                .arg(src)
                .arg(start)
                .arg(stop)
                .arg(options),
        )
    }

    /// Returns the rank of member in the sorted set stored at key,
    /// with the scores ordered from low to high.
    ///
    /// # Return
    /// * If member exists in the sorted set, the rank of member.
    /// * If member does not exist in the sorted set or key does not exist, None.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrank/>](https://redis.io/commands/zrank/)
    #[must_use]
    fn zrank<K, M>(&self, key: K, member: M) -> CommandResult<T, Option<usize>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("ZRANK").arg(key).arg(member))
    }

    /// Removes the specified members from the sorted set stored at key.
    ///
    /// # Return
    /// The number of members removed from the sorted set, not including non existing members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrem/>](https://redis.io/commands/zrem/)
    #[must_use]
    fn zrem<K, M, C>(&self, key: K, members: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.prepare_command(cmd("ZREM").arg(key).arg(members))
    }

    /// When all the elements in a sorted set are inserted with the same score,
    /// in order to force lexicographical ordering,
    /// this command removes all elements in the sorted set stored at key
    /// between the lexicographical range specified by min and max.
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebylex/>](https://redis.io/commands/zremrangebylex/)
    #[must_use]
    fn zremrangebylex<K, S>(&self, key: K, start: S, stop: S) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
    {
        self.prepare_command(cmd("ZREMRANGEBYLEX").arg(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with rank between start and stop.
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebyrank/>](https://redis.io/commands/zremrangebyrank/)
    #[must_use]
    fn zremrangebyrank<K>(&self, key: K, start: isize, stop: isize) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("ZREMRANGEBYRANK").arg(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with a score between min and max (inclusive).
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebyscore/>](https://redis.io/commands/zremrangebyscore/)
    #[must_use]
    fn zremrangebyscore<K, S>(&self, key: K, start: S, stop: S) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
    {
        self.prepare_command(cmd("ZREMRANGEBYSCORE").arg(key).arg(start).arg(stop))
    }

    /// Returns the rank of member in the sorted set stored at key, with the scores ordered from high to low.
    ///
    /// # Return
    /// * If member exists in the sorted set, the rank of member.
    /// * If member does not exist in the sorted set or key does not exist, None.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrevrank/>](https://redis.io/commands/zrevrank/)
    #[must_use]
    fn zrevrank<K, M>(&self, key: K, member: M) -> CommandResult<T, Option<usize>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("ZREVRANK").arg(key).arg(member))
    }

    /// Iterates elements of Sorted Set types and their associated scores.
    ///
    /// # Returns
    /// A tuple where
    /// * The first value is the cursor as an unsigned 64 bit number
    /// * The second value is a list of members and their scores in a Vec of Tuples
    ///
    /// # See Also
    /// [<https://redis.io/commands/zscan/>](https://redis.io/commands/zscan/)
    #[must_use]
    fn zscan<K, M>(
        &self,
        key: K,
        cursor: usize,
        options: ZScanOptions,
    ) -> CommandResult<T, (u64, Vec<(M, f64)>)>
    where
        K: Into<BulkString>,
        M: FromValue + Default,
    {
        self.prepare_command(cmd("ZSCAN").arg(key).arg(cursor).arg(options))
    }

    /// Returns the score of member in the sorted set at key.
    ///
    /// # Return
    /// The score of `member` or nil if `key`does not exist
    ///
    /// # See Also
    /// [<https://redis.io/commands/zscore/>](https://redis.io/commands/zscore/)
    #[must_use]
    fn zscore<K, M>(&self, key: K, member: M) -> CommandResult<T, Option<f64>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.prepare_command(cmd("ZSCORE").arg(key).arg(member))
    }

    /// This command is similar to [zunionstore](crate::SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the unionsection as an array of members
    ///
    /// # See Also
    /// [<https://redis.io/commands/zunion/>](https://redis.io/commands/zunion/)
    #[must_use]
    fn zunion<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> CommandResult<T, Vec<E>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue,
    {
        self.prepare_command(
            cmd("ZUNION")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }

    /// This command is similar to [zunionstore](crate::SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the unionsection as an array of members with their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zunion/>](https://redis.io/commands/zunion/)
    #[must_use]
    fn zunion_with_scores<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> CommandResult<T, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue + Default,
    {
        self.prepare_command(
            cmd("ZUNION")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate)
                .arg("WITHSCORES"),
        )
    }

    /// Computes the unionsection of numkeys sorted sets given by the specified keys,
    /// and stores the result in destination.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set at destination.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zunionstore/>](https://redis.io/commands/zunionstore/)
    #[must_use]
    fn zunionstore<D, K, C, W>(
        &self,
        destination: D,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
    {
        self.prepare_command(
            cmd("ZUNIONSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }
}

type BZpopMinMaxResult<K, E> = Option<Vec<(K, E, f64)>>;

/// Condition option for the [zadd](crate::SortedSetCommands::zadd) command
pub enum ZAddCondition {
    /// No condition
    None,
    /// Only update elements that already exist. Don't add new elements.
    NX,
    /// Only add new elements. Don't update already existing elements.
    XX,
}

impl Default for ZAddCondition {
    fn default() -> Self {
        ZAddCondition::None
    }
}

impl IntoArgs for ZAddCondition {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ZAddCondition::None => args,
            ZAddCondition::NX => args.arg("NX"),
            ZAddCondition::XX => args.arg("XX"),
        }
    }
}

/// Comparison option for the [zadd](crate::SortedSetCommands::zadd) command
pub enum ZAddComparison {
    /// No comparison
    None,
    /// Only update existing elements if the new score is greater than the current score.
    ///
    /// This flag doesn't prevent adding new elements.
    GT,
    /// Only update existing elements if the new score is less than the current score.
    ///
    /// This flag doesn't prevent adding new elements.
    LT,
}

impl Default for ZAddComparison {
    fn default() -> Self {
        ZAddComparison::None
    }
}

impl IntoArgs for ZAddComparison {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ZAddComparison::None => args,
            ZAddComparison::GT => args.arg("GT"),
            ZAddComparison::LT => args.arg("LT"),
        }
    }
}

/// sort by option of the [zrange](crate::SortedSetCommands::zrange) command
pub enum ZRangeSortBy {
    /// No sort by
    None,
    /// When the `ByScore` option is provided, the command behaves like `ZRANGEBYSCORE` and returns
    /// the range of elements from the sorted set having scores equal or between `start` and `stop`.
    ByScore,
    /// When the `ByLex` option is used, the command behaves like `ZRANGEBYLEX` and returns the range
    /// of elements from the sorted set between the `start` and `stop` lexicographical closed range intervals.
    ByLex,
}

impl Default for ZRangeSortBy {
    fn default() -> Self {
        ZRangeSortBy::None
    }
}

impl IntoArgs for ZRangeSortBy {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ZRangeSortBy::None => args,
            ZRangeSortBy::ByScore => args.arg("BYSCORE"),
            ZRangeSortBy::ByLex => args.arg("BYLEX"),
        }
    }
}

/// Option that specify how results of an union or intersection are aggregated
///
/// # See Also
/// [zinter](crate::SortedSetCommands::zinter)
/// [zinterstore](crate::SortedSetCommands::zinterstore)
/// [zunion](crate::SortedSetCommands::zunion)
/// [zunionstore](crate::SortedSetCommands::zunionstore)
pub enum ZAggregate {
    /// No aggregation
    None,
    /// The score of an element is summed across the inputs where it exists.
    Sum,
    /// The minimum score of an element across the inputs where it exists.
    Min,
    /// The maximum score of an element across the inputs where it exists.
    Max,
}

impl Default for ZAggregate {
    fn default() -> Self {
        ZAggregate::None
    }
}

impl IntoArgs for ZAggregate {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ZAggregate::None => args,
            ZAggregate::Sum => args.arg("SUM"),
            ZAggregate::Min => args.arg("MIN"),
            ZAggregate::Max => args.arg("MAX"),
        }
    }
}

/// Where option of the [zmpop](crate::SortedSetCommands::zmpop) command
pub enum ZWhere {
    /// When the MIN modifier is used, the elements popped are those
    /// with the lowest scores from the first non-empty sorted set.
    Min,
    /// The MAX modifier causes elements with the highest scores to be popped.
    Max,
}

impl IntoArgs for ZWhere {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            ZWhere::Min => args.arg("MIN"),
            ZWhere::Max => args.arg("MAX"),
        }
    }
}

/// Options for the command [zadd](crate::SortedSetCommands::zadd)
#[derive(Default)]
pub struct ZAddOptions {
    command_args: CommandArgs,
}

impl ZAddOptions {
    #[must_use]
    pub fn condition(self, condition: ZAddCondition) -> Self {
        Self {
            command_args: self.command_args.arg(condition),
        }
    }

    #[must_use]
    pub fn comparison(self, comparison: ZAddComparison) -> Self {
        Self {
            command_args: self.command_args.arg(comparison),
        }
    }

    #[must_use]
    pub fn change(self) -> Self {
        Self {
            command_args: self.command_args.arg("CH"),
        }
    }
}

impl IntoArgs for ZAddOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

pub type ZMPopResult<E> = (String, Vec<(E, f64)>);

/// Options for the [`zrange`](crate::SortedSetCommands::zrange)
/// and [`zrangestore`](crate::SortedSetCommands::zrangestore) commands
#[derive(Default)]
pub struct ZRangeOptions {
    command_args: CommandArgs,
}

impl ZRangeOptions {
    #[must_use]
    pub fn sort_by(self, sort_by: ZRangeSortBy) -> Self {
        Self {
            command_args: self.command_args.arg(sort_by),
        }
    }

    #[must_use]
    pub fn reverse(self) -> Self {
        Self {
            command_args: self.command_args.arg("REV"),
        }
    }

    #[must_use]
    pub fn limit(self, offset: usize, count: isize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(offset).arg(count),
        }
    }
}

impl IntoArgs for ZRangeOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`zscan`](crate::SortedSetCommands::zscan) command
#[derive(Default)]
pub struct ZScanOptions {
    command_args: CommandArgs,
}

impl ZScanOptions {
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

impl IntoArgs for ZScanOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
