use crate::{
    cmd,
    resp::{BulkString, FromValue},
    ArgsOrCollection, CommandSend, Future, SingleArgOrCollection,
};

/// A group of Redis commands related to Sorted Sets
///
/// # See Also
/// [Redis Sorted Set Commands](https://redis.io/commands/?group=sorted-set)
pub trait SortedSetCommands: CommandSend {
    /// Adds all the specified members with the specified scores
    /// to the sorted set stored at key.
    ///
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the `change` option is specified, the number of elements that were changed (added or updated).
    ///
    /// # See Also
    /// [https://redis.io/commands/zadd/](https://redis.io/commands/zadd/)
    fn zadd<K, M, I>(
        &self,
        key: K,
        condition: Option<ZAddCondition>,
        comparison: Option<ZAddComparison>,
        change: bool,
        items: I,
    ) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        I: ArgsOrCollection<(f64, M)>,
    {
        self.send_into(
            cmd("ZADD")
                .arg(key)
                .arg(condition)
                .arg(comparison)
                .arg_if(change, "CH")
                .arg(items),
        )
    }

    /// In this mode ZADD acts like ZINCRBY.
    /// Only one score-element pair can be specified in this mode.
    ///
    /// # Return
    /// The new score of member (a double precision floating point number),
    /// or nil if the operation was aborted (when called with either the XX or the NX option).
    ///
    /// # See Also
    /// [https://redis.io/commands/zadd/](https://redis.io/commands/zadd/)
    fn zadd_incr<K, M>(
        &self,
        key: K,
        condition: Option<ZAddCondition>,
        comparison: Option<ZAddComparison>,
        change: bool,
        score: f64,
        member: M,
    ) -> Future<'_, Option<f64>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(
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
    /// [https://redis.io/commands/zcard/](https://redis.io/commands/zcard/)
    fn zcard<K>(&self, key: K) -> Future<'_, usize>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("ZCARD").arg(key))
    }

    /// Returns the number of elements in the sorted set at key with a score between min and max.
    ///
    /// # Return
    /// The number of elements in the specified score range.
    ///
    /// # See Also
    /// [https://redis.io/commands/zcount/](https://redis.io/commands/zcount/)
    fn zcount<K, T, U>(&self, key: K, min: T, max: U) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        T: Into<BulkString>,
        U: Into<BulkString>,
    {
        self.send_into(cmd("ZCOUNT").arg(key).arg(min).arg(max))
    }

    /// This command is similar to [zdiffstore](crate::SortedSetCommands::zdiffstore), but instead of storing the resulting sorted set,
    /// it is returned to the client.
    ///
    /// # Return
    /// The result of the difference
    ///
    /// # See Also
    /// [https://redis.io/commands/zdiff/](https://redis.io/commands/zdiff/)
    fn zdiff<K, C, E>(&self, keys: C) -> Future<'_, Vec<E>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue,
    {
        self.send_into(cmd("ZDIFF").arg(keys.num_args()).arg(keys))
    }

    /// This command is similar to [zdiffstore](crate::SortedSetCommands::zdiffstore), but instead of storing the resulting sorted set,
    /// it is returned to the client.
    ///
    /// # Return
    /// The result of the difference with their scores
    ///
    /// # See Also
    /// [https://redis.io/commands/zdiff/](https://redis.io/commands/zdiff/)
    fn zdiff_with_scores<K, C, E>(&self, keys: C) -> Future<'_, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue + Default,
    {
        self.send_into(cmd("ZDIFF").arg(keys.num_args()).arg(keys).arg("WITHSCORES"))
    }

    /// Computes the difference between the first and all successive
    /// input sorted sets and stores the result in destination.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set at destination.
    ///
    /// # See Also
    /// [https://redis.io/commands/zdiffstore/](https://redis.io/commands/zdiffstore/)
    fn zdiffstore<D, K, C>(&self, destination: D, keys: C) -> Future<'_, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(
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
    /// [https://redis.io/commands/zincrby/](https://redis.io/commands/zincrby/)
    fn zincrby<K, M>(&self, key: K, increment: f64, member: M) -> Future<'_, f64>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZINCRBY").arg(key).arg(increment).arg(member))
    }

    /// This command is similar to [zinterstore](crate::SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the intersection as an array of members
    ///
    /// # See Also
    /// [https://redis.io/commands/zinter/](https://redis.io/commands/zinter/)
    fn zinter<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: Option<ZAggregate>,
    ) -> Future<'_, Vec<E>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue,
    {
        self.send_into(
            cmd("ZINTER")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate.map(|a| ("AGGREGATE", a))),
        )
    }

    /// This command is similar to [zinterstore](crate::SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the intersection as an array of members with their scores
    ///
    /// # See Also
    /// [https://redis.io/commands/zinter/](https://redis.io/commands/zinter/)
    fn zinter_with_scores<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: Option<ZAggregate>,
    ) -> Future<'_, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue + Default,
    {
        self.send_into(
            cmd("ZINTER")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate.map(|a| ("AGGREGATE", a)))
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
    /// [https://redis.io/commands/zintercard/](https://redis.io/commands/zintercard/)
    fn zintercard<K, C>(&self, keys: C, limit: usize) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        self.send_into(
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
    /// [https://redis.io/commands/zinterstore/](https://redis.io/commands/zinterstore/)
    fn zinterstore<D, K, C, W>(
        &self,
        destination: D,
        keys: C,
        weights: Option<W>,
        aggregate: Option<ZAggregate>,
    ) -> Future<'_, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
    {
        self.send_into(
            cmd("ZINTERSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate.map(|a| ("AGGREGATE", a))),
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
    /// [https://redis.io/commands/zlexcount/](https://redis.io/commands/zlexcount/)
    fn zlexcount<K, M1, M2>(&self, key: K, min: M1, max: M2) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        M1: Into<BulkString>,
        M2: Into<BulkString>,
    {
        self.send_into(cmd("ZLEXCOUNT").arg(key).arg(min).arg(max))
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
    /// [https://redis.io/commands/zmpop/](https://redis.io/commands/zmpop/)
    fn zmpop<K, C, E>(
        &self,
        keys: C,
        where_: ZWhere,
        count: usize,
    ) -> Future<'_, Option<(String, Vec<(E, f64)>)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue + Default,
    {
        self.send_into(
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
    /// [https://redis.io/commands/zmscore/](https://redis.io/commands/zmscore/)
    fn zmscore<K, M, C>(&self, key: K, members: C) -> Future<'_, Vec<Option<f64>>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.send_into(cmd("ZMSCORE").arg(key).arg(members))
    }

    /// Removes and returns up to count members with the highest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [https://redis.io/commands/zpopmax/](https://redis.io/commands/zpopmax/)
    fn zpopmax<K, M>(&self, key: K, count: usize) -> Future<'_, Vec<(M, f64)>>
    where
        K: Into<BulkString>,
        M: FromValue + Default,
    {
        self.send_into(cmd("ZPOPMAX").arg(key).arg(count))
    }

    /// Removes and returns up to count members with the lowest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [https://redis.io/commands/zpopmin/](https://redis.io/commands/zpopmin/)
    fn zpopmin<K, M>(&self, key: K, count: usize) -> Future<'_, Vec<(M, f64)>>
    where
        K: Into<BulkString>,
        M: FromValue + Default,
    {
        self.send_into(cmd("ZPOPMIN").arg(key).arg(count))
    }

    /// Return a random element from the sorted set value stored at key.
    ///
    /// # Return
    /// The randomly selected element, or nil when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/zrandmember/](https://redis.io/commands/zrandmember/)
    fn zrandmember<K, E>(&self, key: K) -> Future<'_, E>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("ZRANDMEMBER").arg(key))
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
    /// [https://redis.io/commands/zrandmember/](https://redis.io/commands/zrandmember/)
    fn zrandmembers<K, E>(&self, key: K, count: isize) -> Future<'_, Vec<E>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("ZRANDMEMBER").arg(key).arg(count))
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
    /// [https://redis.io/commands/zrandmember/](https://redis.io/commands/zrandmember/)
    fn zrandmembers_with_scores<K, E>(&self, key: K, count: isize) -> Future<'_, Vec<E>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("ZRANDMEMBER").arg(key).arg(count).arg("WITHSCORES"))
    }

    /// Returns the specified range of elements in the sorted set stored at `key`.
    ///
    /// # Return
    /// A collection of elements in the specified range
    ///
    /// # See Also
    /// [https://redis.io/commands/zrange/](https://redis.io/commands/zrange/)
    fn zrange<K, S, E>(
        &self,
        key: K,
        start: S,
        stop: S,
        sort_by: Option<ZRangeSortBy>,
        reverse: bool,
        limit: Option<(usize, isize)>,
    ) -> Future<'_, Vec<E>>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(
            cmd("ZRANGE")
                .arg(key)
                .arg(start)
                .arg(stop)
                .arg(sort_by)
                .arg_if(reverse, "REV")
                .arg(limit.map(|(offset, count)| ("LIMIT", offset, count))),
        )
    }

    /// Returns the specified range of elements in the sorted set stored at `key`.
    ///
    /// # Return
    /// A collection of elements and their scores in the specified range
    ///
    /// # See Also
    /// [https://redis.io/commands/zrange/](https://redis.io/commands/zrange/)
    fn zrange_with_scores<K, S, E>(
        &self,
        key: K,
        start: S,
        stop: S,
        sort_by: Option<ZRangeSortBy>,
        reverse: bool,
        limit: Option<(usize, isize)>,
    ) -> Future<'_, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
        E: FromValue + Default,
    {
        self.send_into(
            cmd("ZRANGE")
                .arg(key)
                .arg(start)
                .arg(stop)
                .arg(sort_by)
                .arg_if(reverse, "REV")
                .arg(limit.map(|(offset, count)| ("LIMIT", offset, count)))
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
    /// [https://redis.io/commands/zrangestore/](https://redis.io/commands/zrangestore/)
    fn zrangestore<D, S, T>(
        &self,
        dst: D,
        src: S,
        start: T,
        stop: T,
        sort_by: Option<ZRangeSortBy>,
        reverse: bool,
        limit: Option<(usize, isize)>,
    ) -> Future<'_, usize>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        T: Into<BulkString>,
    {
        self.send_into(
            cmd("ZRANGESTORE")
                .arg(dst)
                .arg(src)
                .arg(start)
                .arg(stop)
                .arg(sort_by)
                .arg_if(reverse, "REV")
                .arg(limit.map(|(offset, count)| ("LIMIT", offset, count))),
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
    /// [https://redis.io/commands/zrank/](https://redis.io/commands/zrank/)
    fn zrank<K, M>(&self, key: K, member: M) -> Future<'_, Option<usize>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZRANK").arg(key).arg(member))
    }

    /// Removes the specified members from the sorted set stored at key.
    ///
    /// # Return
    /// The number of members removed from the sorted set, not including non existing members.
    ///
    /// # See Also
    /// [https://redis.io/commands/zrem/](https://redis.io/commands/zrem/)
    fn zrem<K, M, C>(&self, key: K, members: C) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
        C: SingleArgOrCollection<M>,
    {
        self.send_into(cmd("ZREM").arg(key).arg(members))
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
    /// [https://redis.io/commands/zremrangebylex/](https://redis.io/commands/zremrangebylex/)
    fn zremrangebylex<K, S>(&self, key: K, start: S, stop: S) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
    {
        self.send_into(cmd("ZREMRANGEBYLEX").arg(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with rank between start and stop.
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [https://redis.io/commands/zremrangebyrank/](https://redis.io/commands/zremrangebyrank/)
    fn zremrangebyrank<K>(&self, key: K, start: isize, stop: isize) -> Future<'_, usize>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("ZREMRANGEBYRANK").arg(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with a score between min and max (inclusive).
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [https://redis.io/commands/zremrangebyscore/](https://redis.io/commands/zremrangebyscore/)
    fn zremrangebyscore<K, S>(&self, key: K, start: S, stop: S) -> Future<'_, usize>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
    {
        self.send_into(cmd("ZREMRANGEBYSCORE").arg(key).arg(start).arg(stop))
    }

    /// Returns the rank of member in the sorted set stored at key, with the scores ordered from high to low.
    ///
    /// # Return
    /// * If member exists in the sorted set, the rank of member.
    /// * If member does not exist in the sorted set or key does not exist, None.
    ///
    /// # See Also
    /// [https://redis.io/commands/zrevrank/](https://redis.io/commands/zrevrank/)
    fn zrevrank<K, M>(&self, key: K, member: M) -> Future<'_, Option<usize>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZREVRANK").arg(key).arg(member))
    }

    /// Iterates elements of Sorted Set types and their associated scores.
    ///
    /// # Returns
    /// A tuple where
    /// * The first value is the cursor as an unsigned 64 bit number
    /// * The second value is a list of members and their scores in a Vec of Tuples
    ///
    /// # See Also
    /// [https://redis.io/commands/zscan/](https://redis.io/commands/zscan/)
    fn zscan<K, P, M>(
        &self,
        key: K,
        cursor: usize,
        match_pattern: Option<P>,
        count: Option<usize>,
    ) -> Future<'_, (u64, Vec<(M, f64)>)>
    where
        K: Into<BulkString>,
        P: Into<BulkString>,
        M: FromValue + Default,
    {
        self.send_into(
            cmd("ZSCAN")
                .arg(key)
                .arg(cursor)
                .arg(match_pattern.map(|p| ("MATCH", p)))
                .arg(count.map(|c| ("COUNT", c))),
        )
    }

    /// Returns the score of member in the sorted set at key.
    ///
    /// # Return
    /// The score of `member` or nil if `key`does not exist
    ///
    /// # See Also
    /// [https://redis.io/commands/zscore/](https://redis.io/commands/zscore/)
    fn zscore<K, M>(&self, key: K, member: M) -> Future<'_, Option<f64>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZSCORE").arg(key).arg(member))
    }

    /// This command is similar to [zunionstore](crate::SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the unionsection as an array of members
    ///
    /// # See Also
    /// [https://redis.io/commands/zunion/](https://redis.io/commands/zunion/)
    fn zunion<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: Option<ZAggregate>,
    ) -> Future<'_, Vec<E>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue,
    {
        self.send_into(
            cmd("ZUNION")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate.map(|a| ("AGGREGATE", a))),
        )
    }

    /// This command is similar to [zunionstore](crate::SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the unionsection as an array of members with their scores
    ///
    /// # See Also
    /// [https://redis.io/commands/zunion/](https://redis.io/commands/zunion/)
    fn zunion_with_scores<K, C, W, E>(
        &self,
        keys: C,
        weights: Option<W>,
        aggregate: Option<ZAggregate>,
    ) -> Future<'_, Vec<(E, f64)>>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
        E: FromValue + Default,
    {
        self.send_into(
            cmd("ZUNION")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate.map(|a| ("AGGREGATE", a)))
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
    /// [https://redis.io/commands/zunionstore/](https://redis.io/commands/zunionstore/)
    fn zunionstore<D, K, C, W>(
        &self,
        destination: D,
        keys: C,
        weights: Option<W>,
        aggregate: Option<ZAggregate>,
    ) -> Future<'_, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        W: SingleArgOrCollection<f64>,
    {
        self.send_into(
            cmd("ZUNIONSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate.map(|a| ("AGGREGATE", a))),
        )
    }
}

/// Condition option for the [zadd](crate::SortedSetCommands::zadd) command
pub enum ZAddCondition {
    /// Only update elements that already exist. Don't add new elements.
    NX,
    /// Only add new elements. Don't update already existing elements.
    XX,
}

impl From<ZAddCondition> for BulkString {
    fn from(cond: ZAddCondition) -> Self {
        match cond {
            ZAddCondition::NX => BulkString::Str("NX"),
            ZAddCondition::XX => BulkString::Str("XX"),
        }
    }
}

/// Comparison option for the [zadd](crate::SortedSetCommands::zadd) command
pub enum ZAddComparison {
    /// Only update existing elements if the new score is greater than the current score.
    ///
    /// This flag doesn't prevent adding new elements.
    GT,
    /// Only update existing elements if the new score is less than the current score.
    ///
    /// This flag doesn't prevent adding new elements.
    LT,
}

impl From<ZAddComparison> for BulkString {
    fn from(cond: ZAddComparison) -> Self {
        match cond {
            ZAddComparison::GT => BulkString::Str("GT"),
            ZAddComparison::LT => BulkString::Str("LT"),
        }
    }
}

/// SortBy option of the [zrange](crate::SortedSetCommands::zrange) command
pub enum ZRangeSortBy {
    /// When the `ByScore` option is provided, the command behaves like `ZRANGEBYSCORE` and returns
    /// the range of elements from the sorted set having scores equal or between `start` and `stop`.
    ByScore,
    /// When the `ByLex` option is used, the command behaves like `ZRANGEBYLEX` and returns the range
    /// of elements from the sorted set between the `start` and `stop` lexicographical closed range intervals.
    ByLex,
}

impl From<ZRangeSortBy> for BulkString {
    fn from(s: ZRangeSortBy) -> Self {
        match s {
            ZRangeSortBy::ByScore => BulkString::Str("BYSCORE"),
            ZRangeSortBy::ByLex => BulkString::Str("BYLEX"),
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
    /// The score of an element is summed across the inputs where it exists.
    Sum,
    /// The minimum score of an element across the inputs where it exists.
    Min,
    /// The maximum score of an element across the inputs where it exists.
    Max,
}

impl From<ZAggregate> for BulkString {
    fn from(a: ZAggregate) -> Self {
        match a {
            ZAggregate::Sum => BulkString::Str("SUM"),
            ZAggregate::Min => BulkString::Str("MIN"),
            ZAggregate::Max => BulkString::Str("MAX"),
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

impl From<ZWhere> for BulkString {
    fn from(w: ZWhere) -> Self {
        match w {
            ZWhere::Min => BulkString::Str("MIN"),
            ZWhere::Max => BulkString::Str("MAX"),
        }
    }
}
