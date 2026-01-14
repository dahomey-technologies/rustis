use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Response, cmd, deserialize_vec_of_pairs, serialize_flag},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// A group of Redis commands related to [`Sorted Sets`](https://redis.io/docs/data-types/sorted-sets/)
///
/// # See Also
/// [Redis Sorted Set Commands](https://redis.io/commands/?group=sorted-set)
pub trait SortedSetCommands<'a>: Sized {
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
    fn zadd(
        self,
        key: impl Serialize,
        items: impl Serialize,
        options: ZAddOptions,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZADD").key(key).arg(options).arg(items))
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
    fn zadd_incr(
        self,
        key: impl Serialize,
        condition: impl Into<Option<ZAddCondition>>,
        comparison: impl Into<Option<ZAddComparison>>,
        change: bool,
        score: f64,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, Option<f64>> {
        prepare_command(
            self,
            cmd("ZADD")
                .key(key)
                .arg(condition.into())
                .arg(comparison.into())
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
    fn zcard(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZCARD").key(key))
    }

    /// Returns the number of elements in the sorted set at key with a score between min and max.
    ///
    /// # Return
    /// The number of elements in the specified score range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zcount/>](https://redis.io/commands/zcount/)
    #[must_use]
    fn zcount(
        self,
        key: impl Serialize,
        min: impl Serialize,
        max: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZCOUNT").key(key).arg(min).arg(max))
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
    fn zdiff<R: Response>(self, keys: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZDIFF").key_with_count(keys))
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
    fn zdiff_with_scores<R: Response>(self, keys: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZDIFF").key_with_count(keys).arg("WITHSCORES"))
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
    fn zdiffstore(
        self,
        destination: impl Serialize,
        keys: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("ZDIFFSTORE").arg(destination).key_with_count(keys),
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
    fn zincrby(
        self,
        key: impl Serialize,
        increment: f64,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, f64> {
        prepare_command(self, cmd("ZINCRBY").key(key).arg(increment).arg(member))
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
    fn zinter<R: Response>(
        self,
        keys: impl Serialize,
        weights: impl Serialize,
        aggregate: impl Into<Option<ZAggregate>>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZINTER")
                .key_with_count(keys)
                .arg_labeled("WEIGHTS", weights)
                .arg(aggregate.into()),
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
    fn zinter_with_scores<R: Response>(
        self,
        keys: impl Serialize,
        weights: impl Serialize,
        aggregate: impl Into<Option<ZAggregate>>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZINTER")
                .key_with_count(keys)
                .arg_labeled("WEIGHTS", weights)
                .arg(aggregate.into())
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
    fn zintercard(self, keys: impl Serialize, limit: usize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("ZINTERCARD")
                .key_with_count(keys)
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
    fn zinterstore(
        self,
        destination: impl Serialize,
        keys: impl Serialize,
        weights: impl Serialize,
        aggregate: impl Into<Option<ZAggregate>>,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("ZINTERSTORE")
                .arg(destination)
                .key_with_count(keys)
                .arg_labeled("WEIGHTS", weights)
                .arg(aggregate.into()),
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
    fn zlexcount(
        self,
        key: impl Serialize,
        min: impl Serialize,
        max: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZLEXCOUNT").key(key).arg(min).arg(max))
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
    fn zmpop<R: Response + DeserializeOwned>(
        self,
        keys: impl Serialize,
        where_: ZWhere,
        count: usize,
    ) -> PreparedCommand<'a, Self, Option<ZMPopResult<R>>> {
        prepare_command(
            self,
            cmd("ZMPOP")
                .key_with_count(keys)
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
    fn zmscore<R: Response>(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZMSCORE").key(key).arg(members))
    }

    /// Removes and returns up to count members with the highest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zpopmax/>](https://redis.io/commands/zpopmax/)
    #[must_use]
    fn zpopmax<R: Response>(
        self,
        key: impl Serialize,
        count: usize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZPOPMAX").key(key).arg(count))
    }

    /// Removes and returns up to count members with the lowest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zpopmin/>](https://redis.io/commands/zpopmin/)
    #[must_use]
    fn zpopmin<R: Response>(
        self,
        key: impl Serialize,
        count: usize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZPOPMIN").key(key).arg(count))
    }

    /// Return a random element from the sorted set value stored at key.
    ///
    /// # Return
    /// The randomly selected element, or nil when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmember<R: Response>(self, key: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZRANDMEMBER").key(key))
    }

    /// Return random elements from the sorted set value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct elements.
    ///   The array's length is either count or the sorted set's cardinality (ZCARD), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed
    ///   to return the same element multiple times. In this case, the number of returned elements
    ///   is the absolute value of the specified count.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmembers<R: Response>(
        self,
        key: impl Serialize,
        count: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ZRANDMEMBER").key(key).arg(count))
    }

    /// Return random elements with their scores from the sorted set value stored at key.
    ///
    /// # Return
    /// * If the provided count argument is positive, return an array of distinct elements with their scores.
    ///   The array's length is either count or the sorted set's cardinality (ZCARD), whichever is lower.
    /// * If called with a negative count, the behavior changes and the command is allowed
    ///   to return the same element multiple times. In this case, the number of returned elements
    ///   is the absolute value of the specified count.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrandmember/>](https://redis.io/commands/zrandmember/)
    #[must_use]
    fn zrandmembers_with_scores<R: Response>(
        self,
        key: impl Serialize,
        count: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZRANDMEMBER").key(key).arg(count).arg("WITHSCORES"),
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
    fn zrange<R: Response>(
        self,
        key: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
        options: ZRangeOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZRANGE").key(key).arg(start).arg(stop).arg(options),
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
    fn zrange_with_scores<R: Response>(
        self,
        key: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
        options: ZRangeOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZRANGE")
                .key(key)
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
    fn zrangestore(
        self,
        dst: impl Serialize,
        src: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
        options: ZRangeOptions,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("ZRANGESTORE")
                .key(dst)
                .key(src)
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
    fn zrank(
        self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, Option<usize>> {
        prepare_command(self, cmd("ZRANK").key(key).arg(member))
    }

    /// Returns the rank of member in the sorted set stored at key,
    /// with the scores ordered from low to high.
    ///
    /// # Return
    /// * If member exists in the sorted set, the rank of member and its score
    /// * If member does not exist in the sorted set or key does not exist, None.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrank/>](https://redis.io/commands/zrank/)
    #[must_use]
    fn zrank_with_score(
        self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, Option<(usize, f64)>> {
        prepare_command(self, cmd("ZRANK").key(key).arg(member).arg("WITHSCORE"))
    }

    /// Removes the specified members from the sorted set stored at key.
    ///
    /// # Return
    /// The number of members removed from the sorted set, not including non existing members.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrem/>](https://redis.io/commands/zrem/)
    #[must_use]
    fn zrem(
        self,
        key: impl Serialize,
        members: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZREM").key(key).arg(members))
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
    fn zremrangebylex(
        self,
        key: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZREMRANGEBYLEX").key(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with rank between start and stop.
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebyrank/>](https://redis.io/commands/zremrangebyrank/)
    #[must_use]
    fn zremrangebyrank(
        self,
        key: impl Serialize,
        start: isize,
        stop: isize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZREMRANGEBYRANK").key(key).arg(start).arg(stop))
    }

    /// Removes all elements in the sorted set stored at key with a score between min and max (inclusive).
    ///
    /// # Return
    /// the number of elements removed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zremrangebyscore/>](https://redis.io/commands/zremrangebyscore/)
    #[must_use]
    fn zremrangebyscore(
        self,
        key: impl Serialize,
        start: impl Serialize,
        stop: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ZREMRANGEBYSCORE").key(key).arg(start).arg(stop))
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
    fn zrevrank(
        self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, Option<usize>> {
        prepare_command(self, cmd("ZREVRANK").key(key).arg(member))
    }

    /// Returns the rank of member in the sorted set stored at key, with the scores ordered from high to low.
    ///
    /// # Return
    /// * If member exists in the sorted set, the rank of member and its score.
    /// * If member does not exist in the sorted set or key does not exist, None.
    ///
    /// # See Also
    /// [<https://redis.io/commands/zrevrank/>](https://redis.io/commands/zrevrank/)
    #[must_use]
    fn zrevrank_with_score(
        self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, Option<(usize, f64)>> {
        prepare_command(self, cmd("ZREVRANK").key(key).arg(member).arg("WITHSCORE"))
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
    fn zscan<R: Response + DeserializeOwned>(
        self,
        key: impl Serialize,
        cursor: usize,
        options: ZScanOptions,
    ) -> PreparedCommand<'a, Self, ZScanResult<R>> {
        prepare_command(self, cmd("ZSCAN").key(key).arg(cursor).arg(options))
    }

    /// Returns the score of member in the sorted set at key.
    ///
    /// # Return
    /// The score of `member` or nil if `key`does not exist
    ///
    /// # See Also
    /// [<https://redis.io/commands/zscore/>](https://redis.io/commands/zscore/)
    #[must_use]
    fn zscore(
        self,
        key: impl Serialize,
        member: impl Serialize,
    ) -> PreparedCommand<'a, Self, Option<f64>> {
        prepare_command(self, cmd("ZSCORE").key(key).arg(member))
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
    fn zunion<R: Response>(
        self,
        keys: impl Serialize,
        weights: impl Serialize,
        aggregate: impl Into<Option<ZAggregate>>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZUNION")
                .key_with_count(keys)
                .arg_labeled("WEIGHTS", weights)
                .arg(aggregate.into()),
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
    fn zunion_with_scores<R: Response>(
        self,
        keys: impl Serialize,
        weights: impl Serialize,
        aggregate: impl Into<Option<ZAggregate>>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ZUNION")
                .key_with_count(keys)
                .arg_labeled("WEIGHTS", weights)
                .arg(aggregate.into())
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
    fn zunionstore(
        self,
        destination: impl Serialize,
        keys: impl Serialize,
        weights: impl Serialize,
        aggregate: impl Into<Option<ZAggregate>>,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("ZUNIONSTORE")
                .arg(destination)
                .key_with_count(keys)
                .arg_labeled("WEIGHTS", weights)
                .arg(aggregate.into()),
        )
    }
}

/// Condition option for the [`zadd`](SortedSetCommands::zadd) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZAddCondition {
    /// Only update elements that already exist. Don't add new elements.
    NX,
    /// Only add new elements. Don't update already existing elements.
    XX,
}

/// Comparison option for the [`zadd`](SortedSetCommands::zadd) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
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

/// sort by option of the [`zrange`](SortedSetCommands::zrange) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZRangeSortBy {
    /// When the `ByScore` option is provided, the command behaves like `ZRANGEBYSCORE` and returns
    /// the range of elements from the sorted set having scores equal or between `start` and `stop`.
    ByScore,
    /// When the `ByLex` option is used, the command behaves like `ZRANGEBYLEX` and returns the range
    /// of elements from the sorted set between the `start` and `stop` lexicographical closed range intervals.
    ByLex,
}

/// Option that specify how results of an union or intersection are aggregated
///
/// # See Also
/// [zinter](SortedSetCommands::zinter)
/// [zinterstore](SortedSetCommands::zinterstore)
/// [zunion](SortedSetCommands::zunion)
/// [zunionstore](SortedSetCommands::zunionstore)
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZAggregate {
    /// The score of an element is summed across the inputs where it exists.
    Sum,
    /// The minimum score of an element across the inputs where it exists.
    Min,
    /// The maximum score of an element across the inputs where it exists.
    Max,
}

/// Where option of the [`zmpop`](SortedSetCommands::zmpop) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZWhere {
    /// When the MIN modifier is used, the elements popped are those
    /// with the lowest scores from the first non-empty sorted set.
    Min,
    /// The MAX modifier causes elements with the highest scores to be popped.
    Max,
}

/// Options for the [`zadd`](SortedSetCommands::zadd) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ZAddOptions {
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    condition: Option<ZAddCondition>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    comparison: Option<ZAddComparison>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    change: bool,
}

impl ZAddOptions {
    #[must_use]
    pub fn condition(mut self, condition: ZAddCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    #[must_use]
    pub fn comparison(mut self, comparison: ZAddComparison) -> Self {
        self.comparison = Some(comparison);
        self
    }

    #[must_use]
    pub fn change(mut self) -> Self {
        self.change = true;
        self
    }
}

/// Result for [`zmpop`](SortedSetCommands::zmpop) the command.
pub type ZMPopResult<E> = (String, Vec<(E, f64)>);

/// Options for the [`zrange`](SortedSetCommands::zrange)
/// and [`zrangestore`](SortedSetCommands::zrangestore) commands
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ZRangeOptions {
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    sort_by: Option<ZRangeSortBy>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    reverse: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<(u32, i32)>,
}

impl ZRangeOptions {
    #[must_use]
    pub fn sort_by(mut self, sort_by: ZRangeSortBy) -> Self {
        self.sort_by = Some(sort_by);
        self
    }

    #[must_use]
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    #[must_use]
    pub fn limit(mut self, offset: u32, count: i32) -> Self {
        self.limit = Some((offset, count));
        self
    }
}

/// Options for the [`zscan`](SortedSetCommands::zscan) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ZScanOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#match: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
}

impl<'a> ZScanOptions<'a> {
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

/// Result for the [`zscan`](SortedSetCommands::zscan) command.
#[derive(Debug, Deserialize)]
pub struct ZScanResult<R: Response + DeserializeOwned> {
    pub cursor: u64,
    #[serde(deserialize_with = "deserialize_vec_of_pairs")]
    pub elements: Vec<(R, f64)>,
}
