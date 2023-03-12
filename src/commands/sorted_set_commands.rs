use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, deserialize_vec_of_pairs, CommandArgs, MultipleArgsCollection, PrimitiveResponse,
        SingleArg, SingleArgCollection, ToArgs,
    },
};
use serde::{de::DeserializeOwned, Deserialize};

/// A group of Redis commands related to [`Sorted Sets`](https://redis.io/docs/data-types/sorted-sets/)
///
/// # See Also
/// [Redis Sorted Set Commands](https://redis.io/commands/?group=sorted-set)
pub trait SortedSetCommands<'a> {
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
    fn zadd<K, M, I>(
        self,
        key: K,
        items: I,
        options: ZAddOptions,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        I: MultipleArgsCollection<(f64, M)>,
    {
        prepare_command(self, cmd("ZADD").arg(key).arg(options).arg(items))
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
        self,
        key: K,
        condition: ZAddCondition,
        comparison: ZAddComparison,
        change: bool,
        score: f64,
        member: M,
    ) -> PreparedCommand<'a, Self, Option<f64>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
        prepare_command(
            self,
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
    fn zcard<K>(self, key: K) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("ZCARD").arg(key))
    }

    /// Returns the number of elements in the sorted set at key with a score between min and max.
    ///
    /// # Return
    /// The number of elements in the specified score range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zcount/>](https://redis.io/commands/zcount/)
    #[must_use]
    fn zcount<K, M1, M2>(self, key: K, min: M1, max: M2) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M1: SingleArg,
        M2: SingleArg,
    {
        prepare_command(self, cmd("ZCOUNT").arg(key).arg(min).arg(max))
    }

    /// This command is similar to [zdiffstore](SortedSetCommands::zdiffstore), but instead of storing the resulting sorted set,
    /// it is returned to the client.
    ///
    /// # Return
    /// The result of the difference
    ///
    /// # See Also
    /// [<https://redis.io/commands/zdiff/>](https://redis.io/commands/zdiff/)
    #[must_use]
    fn zdiff<K, C, E>(self, keys: C) -> PreparedCommand<'a, Self, Vec<E>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(self, cmd("ZDIFF").arg(keys.num_args()).arg(keys))
    }

    /// This command is similar to [zdiffstore](SortedSetCommands::zdiffstore), but instead of storing the resulting sorted set,
    /// it is returned to the client.
    ///
    /// # Return
    /// The result of the difference with their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zdiff/>](https://redis.io/commands/zdiff/)
    #[must_use]
    fn zdiff_with_scores<K, C, E>(self, keys: C) -> PreparedCommand<'a, Self, Vec<(E, f64)>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
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
    fn zdiffstore<D, K, C>(self, destination: D, keys: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        prepare_command(
            self,
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
    fn zincrby<K, M>(self, key: K, increment: f64, member: M) -> PreparedCommand<'a, Self, f64>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("ZINCRBY").arg(key).arg(increment).arg(member))
    }

    /// This command is similar to [zinterstore](SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the intersection as an array of members
    ///
    /// # See Also
    /// [<https://redis.io/commands/zinter/>](https://redis.io/commands/zinter/)
    #[must_use]
    fn zinter<K, C, W, E>(
        self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> PreparedCommand<'a, Self, Vec<E>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        W: SingleArgCollection<f64>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("ZINTER")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }

    /// This command is similar to [zinterstore](SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the intersection as an array of members with their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zinter/>](https://redis.io/commands/zinter/)
    #[must_use]
    fn zinter_with_scores<K, C, W, E>(
        self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> PreparedCommand<'a, Self, Vec<(E, f64)>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        W: SingleArgCollection<f64>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("ZINTER")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate)
                .arg("WITHSCORES"),
        )
    }

    /// This command is similar to [zinter](SortedSetCommands::zinter),
    /// but instead of returning the result set, it returns just the cardinality of the result.
    ///
    //// limit: if the intersection cardinality reaches limit partway through the computation,
    /// the algorithm will exit and yield limit as the cardinality. 0 means unlimited
    ///
    /// # See Also
    /// [<https://redis.io/commands/zintercard/>](https://redis.io/commands/zintercard/)
    #[must_use]
    fn zintercard<K, C>(self, keys: C, limit: usize) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
    {
        prepare_command(
            self,
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
        self,
        destination: D,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        C: SingleArgCollection<K>,
        W: SingleArgCollection<f64>,
    {
        prepare_command(
            self,
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
    fn zlexcount<K, M1, M2>(self, key: K, min: M1, max: M2) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M1: SingleArg,
        M2: SingleArg,
    {
        prepare_command(self, cmd("ZLEXCOUNT").arg(key).arg(min).arg(max))
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
        self,
        keys: C,
        where_: ZWhere,
        count: usize,
    ) -> PreparedCommand<'a, Self, Option<ZMPopResult<E>>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
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
    fn zmscore<K, M, C>(self, key: K, members: C) -> PreparedCommand<'a, Self, Vec<Option<f64>>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
        prepare_command(self, cmd("ZMSCORE").arg(key).arg(members))
    }

    /// Removes and returns up to count members with the highest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zpopmax/>](https://redis.io/commands/zpopmax/)
    #[must_use]
    fn zpopmax<K, M>(self, key: K, count: usize) -> PreparedCommand<'a, Self, Vec<(M, f64)>>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(self, cmd("ZPOPMAX").arg(key).arg(count))
    }

    /// Removes and returns up to count members with the lowest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zpopmin/>](https://redis.io/commands/zpopmin/)
    #[must_use]
    fn zpopmin<K, M>(self, key: K, count: usize) -> PreparedCommand<'a, Self, Vec<(M, f64)>>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(self, cmd("ZPOPMIN").arg(key).arg(count))
    }

    /// Return a random element from the sorted set value stored at key.
    ///
    /// # Return
    /// The randomly selected element, or nil when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmember<K, E>(self, key: K) -> PreparedCommand<'a, Self, E>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse,
    {
        prepare_command(self, cmd("ZRANDMEMBER").arg(key))
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
    fn zrandmembers<K, E>(self, key: K, count: isize) -> PreparedCommand<'a, Self, Vec<E>>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(self, cmd("ZRANDMEMBER").arg(key).arg(count))
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
    fn zrandmembers_with_scores<K, E>(
        self,
        key: K,
        count: isize,
    ) -> PreparedCommand<'a, Self, Vec<E>>
    where
        Self: Sized,
        K: SingleArg,
        E: DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("ZRANDMEMBER").arg(key).arg(count).arg("WITHSCORES"),
        )
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
        self,
        key: K,
        start: S,
        stop: S,
        options: ZRangeOptions,
    ) -> PreparedCommand<'a, Self, Vec<E>>
    where
        Self: Sized,
        K: SingleArg,
        S: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("ZRANGE").arg(key).arg(start).arg(stop).arg(options),
        )
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
        self,
        key: K,
        start: S,
        stop: S,
        options: ZRangeOptions,
    ) -> PreparedCommand<'a, Self, Vec<(E, f64)>>
    where
        Self: Sized,
        K: SingleArg,
        S: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("ZRANGE")
                .arg(key)
                .arg(start)
                .arg(stop)
                .arg(options)
                .arg("WITHSCORES"),
        )
    }

    /// This command is like [zrange](SortedSetCommands::zrange),
    /// but stores the result in the `dst` destination key.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrangestore/>](https://redis.io/commands/zrangestore/)
    #[must_use]
    fn zrangestore<D, S, SS>(
        self,
        dst: D,
        src: S,
        start: SS,
        stop: SS,
        options: ZRangeOptions,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        S: SingleArg,
        SS: SingleArg,
    {
        prepare_command(
            self,
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
    fn zrank<K, M>(self, key: K, member: M) -> PreparedCommand<'a, Self, Option<usize>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("ZRANK").arg(key).arg(member))
    }

    /// Removes the specified members from the sorted set stored at key.
    ///
    /// # Return
    /// The number of members removed from the sorted set, not including non existing members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrem/>](https://redis.io/commands/zrem/)
    #[must_use]
    fn zrem<K, M, C>(self, key: K, members: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
        C: SingleArgCollection<M>,
    {
        prepare_command(self, cmd("ZREM").arg(key).arg(members))
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
    fn zremrangebylex<K, S>(self, key: K, start: S, stop: S) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        S: SingleArg,
    {
        prepare_command(self, cmd("ZREMRANGEBYLEX").arg(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with rank between start and stop.
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebyrank/>](https://redis.io/commands/zremrangebyrank/)
    #[must_use]
    fn zremrangebyrank<K>(
        self,
        key: K,
        start: isize,
        stop: isize,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("ZREMRANGEBYRANK").arg(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with a score between min and max (inclusive).
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebyscore/>](https://redis.io/commands/zremrangebyscore/)
    #[must_use]
    fn zremrangebyscore<K, S>(self, key: K, start: S, stop: S) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        S: SingleArg,
    {
        prepare_command(self, cmd("ZREMRANGEBYSCORE").arg(key).arg(start).arg(stop))
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
    fn zrevrank<K, M>(self, key: K, member: M) -> PreparedCommand<'a, Self, Option<usize>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("ZREVRANK").arg(key).arg(member))
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
        self,
        key: K,
        cursor: usize,
        options: ZScanOptions,
    ) -> PreparedCommand<'a, Self, ZScanResult<M>>
    where
        Self: Sized,
        K: SingleArg,
        M: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(self, cmd("ZSCAN").arg(key).arg(cursor).arg(options))
    }

    /// Returns the score of member in the sorted set at key.
    ///
    /// # Return
    /// The score of `member` or nil if `key`does not exist
    ///
    /// # See Also
    /// [<https://redis.io/commands/zscore/>](https://redis.io/commands/zscore/)
    #[must_use]
    fn zscore<K, M>(self, key: K, member: M) -> PreparedCommand<'a, Self, Option<f64>>
    where
        Self: Sized,
        K: SingleArg,
        M: SingleArg,
    {
        prepare_command(self, cmd("ZSCORE").arg(key).arg(member))
    }

    /// This command is similar to [zunionstore](SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the unionsection as an array of members
    ///
    /// # See Also
    /// [<https://redis.io/commands/zunion/>](https://redis.io/commands/zunion/)
    #[must_use]
    fn zunion<K, C, W, E>(
        self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> PreparedCommand<'a, Self, Vec<E>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        W: SingleArgCollection<f64>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
            cmd("ZUNION")
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }

    /// This command is similar to [zunionstore](SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # Return
    /// The result of the unionsection as an array of members with their scores
    ///
    /// # See Also
    /// [<https://redis.io/commands/zunion/>](https://redis.io/commands/zunion/)
    #[must_use]
    fn zunion_with_scores<K, C, W, E>(
        self,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> PreparedCommand<'a, Self, Vec<(E, f64)>>
    where
        Self: Sized,
        K: SingleArg,
        C: SingleArgCollection<K>,
        W: SingleArgCollection<f64>,
        E: PrimitiveResponse + DeserializeOwned,
    {
        prepare_command(
            self,
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
        self,
        destination: D,
        keys: C,
        weights: Option<W>,
        aggregate: ZAggregate,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        C: SingleArgCollection<K>,
        W: SingleArgCollection<f64>,
    {
        prepare_command(
            self,
            cmd("ZUNIONSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys)
                .arg(weights.map(|w| ("WEIGHTS", w)))
                .arg(aggregate),
        )
    }
}

/// Condition option for the [`zadd`](SortedSetCommands::zadd) command
#[derive(Default)]
pub enum ZAddCondition {
    /// No condition
    #[default]
    None,
    /// Only update elements that already exist. Don't add new elements.
    NX,
    /// Only add new elements. Don't update already existing elements.
    XX,
}

impl ToArgs for ZAddCondition {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ZAddCondition::None => {}
            ZAddCondition::NX => {
                args.arg("NX");
            }
            ZAddCondition::XX => {
                args.arg("XX");
            }
        }
    }
}

/// Comparison option for the [`zadd`](SortedSetCommands::zadd) command
#[derive(Default)]
pub enum ZAddComparison {
    /// No comparison
    #[default]
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

impl ToArgs for ZAddComparison {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ZAddComparison::None => {}
            ZAddComparison::GT => {
                args.arg("GT");
            }
            ZAddComparison::LT => {
                args.arg("LT");
            }
        }
    }
}

/// sort by option of the [`zrange`](SortedSetCommands::zrange) command
#[derive(Default)]
pub enum ZRangeSortBy {
    /// No sort by
    #[default]
    None,
    /// When the `ByScore` option is provided, the command behaves like `ZRANGEBYSCORE` and returns
    /// the range of elements from the sorted set having scores equal or between `start` and `stop`.
    ByScore,
    /// When the `ByLex` option is used, the command behaves like `ZRANGEBYLEX` and returns the range
    /// of elements from the sorted set between the `start` and `stop` lexicographical closed range intervals.
    ByLex,
}

impl ToArgs for ZRangeSortBy {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ZRangeSortBy::None => {}
            ZRangeSortBy::ByScore => {
                args.arg("BYSCORE");
            }
            ZRangeSortBy::ByLex => {
                args.arg("BYLEX");
            }
        }
    }
}

/// Option that specify how results of an union or intersection are aggregated
///
/// # See Also
/// [zinter](SortedSetCommands::zinter)
/// [zinterstore](SortedSetCommands::zinterstore)
/// [zunion](SortedSetCommands::zunion)
/// [zunionstore](SortedSetCommands::zunionstore)
#[derive(Default)]
pub enum ZAggregate {
    /// No aggregation
    #[default]
    None,
    /// The score of an element is summed across the inputs where it exists.
    Sum,
    /// The minimum score of an element across the inputs where it exists.
    Min,
    /// The maximum score of an element across the inputs where it exists.
    Max,
}

impl ToArgs for ZAggregate {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ZAggregate::None => {}
            ZAggregate::Sum => {
                args.arg("SUM");
            }
            ZAggregate::Min => {
                args.arg("MIN");
            }
            ZAggregate::Max => {
                args.arg("MAX");
            }
        }
    }
}

/// Where option of the [`zmpop`](SortedSetCommands::zmpop) command
pub enum ZWhere {
    /// When the MIN modifier is used, the elements popped are those
    /// with the lowest scores from the first non-empty sorted set.
    Min,
    /// The MAX modifier causes elements with the highest scores to be popped.
    Max,
}

impl ToArgs for ZWhere {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            ZWhere::Min => args.arg("MIN"),
            ZWhere::Max => args.arg("MAX"),
        };
    }
}

/// Options for the [`zadd`](SortedSetCommands::zadd) command.
#[derive(Default)]
pub struct ZAddOptions {
    command_args: CommandArgs,
}

impl ZAddOptions {
    #[must_use]
    pub fn condition(mut self, condition: ZAddCondition) -> Self {
        Self {
            command_args: self.command_args.arg(condition).build(),
        }
    }

    #[must_use]
    pub fn comparison(mut self, comparison: ZAddComparison) -> Self {
        Self {
            command_args: self.command_args.arg(comparison).build(),
        }
    }

    #[must_use]
    pub fn change(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("CH").build(),
        }
    }
}

impl ToArgs for ZAddOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        self.command_args.write_args(args);
    }
}

/// Result for [`zmpop`](SortedSetCommands::zmpop) the command.
pub type ZMPopResult<E> = (String, Vec<(E, f64)>);

/// Options for the [`zrange`](SortedSetCommands::zrange)
/// and [`zrangestore`](SortedSetCommands::zrangestore) commands
#[derive(Default)]
pub struct ZRangeOptions {
    command_args: CommandArgs,
}

impl ZRangeOptions {
    #[must_use]
    pub fn sort_by(mut self, sort_by: ZRangeSortBy) -> Self {
        Self {
            command_args: self.command_args.arg(sort_by).build(),
        }
    }

    #[must_use]
    pub fn reverse(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("REV").build(),
        }
    }

    #[must_use]
    pub fn limit(mut self, offset: usize, count: isize) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LIMIT")
                .arg(offset)
                .arg(count)
                .build(),
        }
    }
}

impl ToArgs for ZRangeOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`zscan`](SortedSetCommands::zscan) command
#[derive(Default)]
pub struct ZScanOptions {
    command_args: CommandArgs,
}

impl ZScanOptions {
    #[must_use]
    pub fn match_pattern<P: SingleArg>(mut self, match_pattern: P) -> Self {
        Self {
            command_args: self.command_args.arg("MATCH").arg(match_pattern).build(),
        }
    }

    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }
}

impl ToArgs for ZScanOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`zscan`](SortedSetCommands::zscan) command.
#[derive(Debug, Deserialize)]
pub struct ZScanResult<M>
where
    M: PrimitiveResponse + DeserializeOwned,
{
    pub cursor: u64,
    #[serde(deserialize_with = "deserialize_vec_of_pairs")]
    pub elements: Vec<(M, f64)>,
}
