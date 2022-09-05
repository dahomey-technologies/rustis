use crate::{
    cmd,
    resp::{BulkString, FromValue, Value},
    ArgsOrCollection, Command, CommandSend, Error, Result, SingleArgOrCollection,
};
use futures::Future;
use std::pin::Pin;

/// A group of Redis commands related to Sorted Sets
///
/// # See Also
/// [Redis Sorted Set Commands](https://redis.io/commands/?group=sorted-set)
pub trait SortedSetCommands: CommandSend {
    /// Adds all the specified members with the specified scores
    /// to the sorted set stored at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/zadd/](https://redis.io/commands/zadd/)
    fn zadd<K>(&self, key: K) -> ZAdd<Self>
    where
        K: Into<BulkString>,
    {
        ZAdd {
            sorted_set_commands: &self,
            cmd: cmd("ZADD").arg(key),
        }
    }

    /// Returns the sorted set cardinality (number of elements)
    /// of the sorted set stored at key.
    ///
    /// # Return
    /// The cardinality (number of elements) of the sorted set, or 0 if key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/zcard/](https://redis.io/commands/zcard/)
    fn zcard<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zcount<K, T, U>(
        &self,
        key: K,
        min: T,
        max: U,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    /// The number of elements in the specified score range.
    ///
    /// # See Also
    /// [https://redis.io/commands/zdiff/](https://redis.io/commands/zdiff/)
    fn zdiff<K, C>(&self, keys: C) -> ZDiff<Self>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        ZDiff {
            sorted_set_commands: &self,
            cmd: cmd("ZDIFF").arg(keys.num_args()).arg(keys),
        }
    }

    /// Computes the difference between the first and all successive
    /// input sorted sets and stores the result in destination.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set at destination.
    ///
    /// # See Also
    /// [https://redis.io/commands/zdiffstore/](https://redis.io/commands/zdiffstore/)
    fn zdiffstore<D, K, C>(
        &self,
        destination: D,
        keys: C,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zincrby<K, M>(
        &self,
        key: K,
        increment: f64,
        member: M,
    ) -> Pin<Box<dyn Future<Output = Result<f64>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZINCRBY").arg(key).arg(increment).arg(member))
    }

    /// This command is similar to [zinterstore](crate::SortedSetCommands::zinterstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # See Also
    /// [https://redis.io/commands/zinter/](https://redis.io/commands/zinter/)
    fn zinter<K, C>(&self, keys: C) -> ZInterUnion<Self>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        ZInterUnion {
            sorted_set_commands: &self,
            cmd: cmd("ZINTER").arg(keys.num_args()).arg(keys),
        }
    }

    /// This command is similar to [zinter](crate::SortedSetCommands::zinter),
    /// but instead of returning the result set, it returns just the cardinality of the result.
    ///
    //// limit: if the intersection cardinality reaches limit partway through the computation,
    /// the algorithm will exit and yield limit as the cardinality. 0 means unlimited
    ///
    /// # See Also
    /// [https://redis.io/commands/zintercard/](https://redis.io/commands/zintercard/)
    fn zintercard<K, C>(
        &self,
        keys: C,
        limit: usize,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zinterstore<D, K, C>(&self, destination: D, keys: C) -> ZInterUnionStore<Self>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        ZInterUnionStore {
            sorted_set_commands: &self,
            cmd: cmd("ZINTERSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys),
        }
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
    fn zlexcount<K, M1, M2>(
        &self,
        key: K,
        min: M1,
        max: M2,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    /// the number of elements in the specified score range.
    ///
    /// # See Also
    /// [https://redis.io/commands/zmpop/](https://redis.io/commands/zmpop/)
    fn zmpop<K, C, E>(
        &self,
        keys: C,
        where_: ZWhere,
        count: usize,
    ) -> Pin<Box<dyn Future<Output = Result<(String, Vec<(E, f64)>)>> + '_>>
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
    fn zmscore<K, M, C>(
        &self,
        key: K,
        members: C,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Option<f64>>>> + '_>>
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
    fn zpopmax<K, M>(
        &self,
        key: K,
        count: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(M, f64)>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue,
    {
        self.send_into_tuple_vec(cmd("ZPOPMAX").arg(key).arg(count))
    }

    /// Removes and returns up to count members with the lowest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [https://redis.io/commands/zpopmin/](https://redis.io/commands/zpopmin/)
    fn zpopmin<K, M>(
        &self,
        key: K,
        count: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<(M, f64)>>> + '_>>
    where
        K: Into<BulkString>,
        M: FromValue,
    {
        self.send_into_tuple_vec(cmd("ZPOPMIN").arg(key).arg(count))
    }

    /// Removes and returns up to count members with the lowest scores in the sorted set stored at key.
    ///
    /// # Return
    /// The list of popped elements and scores.
    ///
    /// # See Also
    /// [https://redis.io/commands/zrandmember/](https://redis.io/commands/zrandmember/)
    fn zrandmember<K>(&self, key: K) -> ZRandMember<Self>
    where
        K: Into<BulkString>,
    {
        ZRandMember {
            sorted_set_commands: &self,
            cmd: cmd("ZRANDMEMBER").arg(key),
        }
    }

    /// Returns the specified range of elements in the sorted set stored at `key`.
    ///
    /// # See Also
    /// [https://redis.io/commands/zrange/](https://redis.io/commands/zrange/)
    fn zrange<K, S>(&self, key: K, start: S, stop: S) -> ZRange<Self>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
    {
        ZRange {
            sorted_set_commands: &self,
            cmd: cmd("ZRANGE").arg(key).arg(start).arg(stop),
        }
    }

    /// This command is like [zrange](crate::SortedSetCommands::zrange),
    /// but stores the result in the `dst` destination key.
    ///
    /// # See Also
    /// [https://redis.io/commands/zrangestore/](https://redis.io/commands/zrangestore/)
    fn zrangestore<D, S, T>(&self, dst: D, src: S, start: T, stop: T) -> ZRangeStore<Self>
    where
        D: Into<BulkString>,
        S: Into<BulkString>,
        T: Into<BulkString>,
    {
        ZRangeStore {
            sorted_set_commands: &self,
            cmd: cmd("ZRANGESTORE").arg(dst).arg(src).arg(start).arg(stop),
        }
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
    fn zrank<K, M>(
        &self,
        key: K,
        member: M,
    ) -> Pin<Box<dyn Future<Output = Result<Option<usize>>> + '_>>
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
    fn zrem<K, M, C>(&self, key: K, members: C) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zremrangebylex<K, S>(
        &self,
        key: K,
        start: S,
        stop: S,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zremrangebyrank<K>(
        &self,
        key: K,
        start: isize,
        stop: isize,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zremrangebyscore<K, S>(
        &self,
        key: K,
        start: S,
        stop: S,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
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
    fn zrevrank<K, M>(
        &self,
        key: K,
        member: M,
    ) -> Pin<Box<dyn Future<Output = Result<Option<usize>>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZREVRANK").arg(key).arg(member))
    }

    /// Iterates elements of Sorted Set types and their associated scores.
    ///
    /// # Return
    /// The list of members and their associated scores.
    ///
    /// # See Also
    /// [https://redis.io/commands/zscan/](https://redis.io/commands/zscan/)
    fn zscan<K>(&self, key: K, cursor: usize) -> ZScan<Self>
    where
        K: Into<BulkString>,
    {
        ZScan {
            sorted_set_commands: self,
            cmd: cmd("ZSCAN").arg(key).arg(cursor),
        }
    }

    /// Returns the score of member in the sorted set at key.
    ///
    /// # Return
    /// The score of `member` or nil if `key`does not exist
    ///
    /// # See Also
    /// [https://redis.io/commands/zscore/](https://redis.io/commands/zscore/)
    fn zscore<K, M>(
        &self,
        key: K,
        member: M,
    ) -> Pin<Box<dyn Future<Output = Result<Option<f64>>> + '_>>
    where
        K: Into<BulkString>,
        M: Into<BulkString>,
    {
        self.send_into(cmd("ZSCORE").arg(key).arg(member))
    }

    /// This command is similar to [zunionstore](crate::SortedSetCommands::zunionstore),
    /// but instead of storing the resulting sorted set, it is returned to the client.
    ///
    /// # See Also
    /// [https://redis.io/commands/zunion/](https://redis.io/commands/zunion/)
    fn zunion<K, C>(&self, keys: C) -> ZInterUnion<Self>
    where
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        ZInterUnion {
            sorted_set_commands: &self,
            cmd: cmd("ZUNION").arg(keys.num_args()).arg(keys),
        }
    }

    /// Computes the union  of numkeys sorted sets given by the specified keys,
    /// and stores the result in destination.
    ///
    /// # Return
    /// The number of elements in the resulting sorted set at destination.
    ///
    /// # See Also
    /// [https://redis.io/commands/zunionstore/](https://redis.io/commands/zunionstore/)
    fn zunionstore<D, K, C>(&self, destination: D, keys: C) -> ZInterUnionStore<Self>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
    {
        ZInterUnionStore {
            sorted_set_commands: &self,
            cmd: cmd("ZUNIONSTORE")
                .arg(destination)
                .arg(keys.num_args())
                .arg(keys),
        }
    }
}

pub struct ZAdd<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZAdd<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// # Return
    /// * When used without optional arguments, the number of elements added to the sorted set (excluding score updates).
    /// * If the CH option is specified, the number of elements that were changed (added or updated).
    pub fn execute<M, I>(self, items: I) -> Pin<Box<dyn Future<Output = Result<usize>> + 'a>>
    where
        M: Into<BulkString>,
        I: ArgsOrCollection<(f64, M)>,
    {
        self.sorted_set_commands.send_into(self.cmd.arg(items))
    }

    /// Only update elements that already exist. Don't add new elements.
    pub fn nx(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("NX"),
        }
    }

    /// Only add new elements. Don't update already existing elements.
    pub fn xx(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("XX"),
        }
    }

    /// Only update existing elements if the new score is less than the current score.
    ///
    /// This flag doesn't prevent adding new elements.
    pub fn lt(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("LT"),
        }
    }

    /// Only update existing elements if the new score is greater than the current score.
    ///
    /// This flag doesn't prevent adding new elements.
    pub fn gt(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("GT"),
        }
    }

    /// Modify the return value from the number of new elements added,
    /// to the total number of elements changed (CH is an abbreviation of changed).
    pub fn ch(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("CH"),
        }
    }

    /// When this option is specified ZADD acts like ZINCRBY.
    /// Only one score-element pair can be specified in this mode.
    ///
    /// # Return
    /// The new score of member (a double precision floating point number),
    /// or nil if the operation was aborted (when called with either the XX or the NX option).
    pub fn incr<M>(
        self,
        score: f64,
        member: M,
    ) -> Pin<Box<dyn Future<Output = Result<Option<f64>>> + 'a>>
    where
        M: Into<BulkString>,
    {
        self.sorted_set_commands
            .send_into(self.cmd.arg("INCR").arg(score).arg(member))
    }
}

/// Builder for the [zrange](crate::SortedSetCommands::zrange) command
pub struct ZRange<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZRange<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// When the `ByScore` option is provided, the command behaves like `ZRANGEBYSCORE` and returns
    /// the range of elements from the sorted set having scores equal or between `start` and `stop`.
    ///
    /// When the `ByLex` option is used, the command behaves like `ZRANGEBYLEX` and returns the range
    /// of elements from the sorted set between the `start` and `stop` lexicographical closed range intervals.
    pub fn sort_by(self, sort_by: ZRangeSortBy) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg(sort_by),
        }
    }

    /// Using the REV option reverses the sorted set, with index 0 as the element with the highest score.
    pub fn rev(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("REV"),
        }
    }

    /// The optional LIMIT argument can be used to obtain a sub-range from the matching elements
    /// (similar to SELECT LIMIT offset, count in SQL).
    ///
    /// A negative `count` returns all elements from the `offset`.
    ///
    /// Keep in mind that if `offset` is large, the sorted set needs to be traversed for `offset`
    /// elements before getting to the elements to return, which can add up to O(N) time complexity.
    pub fn limit(self, offset: usize, count: isize) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("LIMIT").arg(offset).arg(count),
        }
    }

    /// # Return
    /// list of elements in the specified range
    pub fn execute<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands.send_into(self.cmd)
    }

    /// The optional `WITHSCORES` argument supplements the command's reply with the scores of elements returned.
    /// The returned list contains value1,score1,...,valueN,scoreN instead of value1,...,valueN.
    ///
    /// # Return
    /// list of elements and their scores in the specified range
    pub fn with_scores<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<(E, f64)>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands
            .send_into_tuple_vec(self.cmd.arg("WITHSCORES"))
    }
}

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

/// Builder for the [zdiff](crate::SortedSetCommands::zdiff) command
pub struct ZDiff<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZDiff<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// The result of the difference
    pub fn execute<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands.send_into(self.cmd)
    }

    /// The result of the difference with scores
    pub fn with_scores<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<(E, f64)>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands
            .send_into_tuple_vec(self.cmd.arg("WITHSCORES"))
    }
}

pub enum ZAggregate {
    Sum,
    Min,
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

/// Builder for the [zinter](crate::SortedSetCommands::zinter) and [zunion](crate::SortedSetCommands::zunion) commands
pub struct ZInterUnion<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZInterUnion<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// Using the WEIGHTS option, it is possible to specify a multiplication factor for each input sorted set.
    ///
    /// This means that the score of every element in every input sorted set is multiplied by this factor
    /// before being passed to the aggregation function.
    /// When WEIGHTS is not given, the multiplication factors default to 1.
    pub fn weights<W>(self, weights: W) -> Self
    where
        W: SingleArgOrCollection<f64>,
    {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("WEIGHT").arg(weights),
        }
    }

    /// With the AGGREGATE option, it is possible to specify how the results of the union are aggregated.
    ///
    /// This option defaults to SUM, where the score of an element is summed across the inputs where it exists.
    /// When this option is set to either MIN or MAX, the resulting set will contain the minimum or maximum score
    /// of an element across the inputs where it exists.
    pub fn aggregate(self, aggregate: ZAggregate) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("AGGREGATE").arg(aggregate),
        }
    }

    /// The result of the intersection
    pub fn execute<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands.send_into(self.cmd)
    }

    /// The result of the intersection with scores
    pub fn with_scores<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<(E, f64)>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands
            .send_into_tuple_vec(self.cmd.arg("WITHSCORES"))
    }
}

/// Builder for the [zinterstore](crate::SortedSetCommands::zinterstore) and [zunionstore](crate::SortedSetCommands::zunionstore) commands
pub struct ZInterUnionStore<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZInterUnionStore<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// Using the WEIGHTS option, it is possible to specify a multiplication factor for each input sorted set.
    ///
    /// This means that the score of every element in every input sorted set is multiplied by this factor
    /// before being passed to the aggregation function.
    /// When WEIGHTS is not given, the multiplication factors default to 1.
    pub fn weights<W>(self, weights: W) -> Self
    where
        W: SingleArgOrCollection<f64>,
    {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("WEIGHTS").arg(weights),
        }
    }

    /// With the AGGREGATE option, it is possible to specify how the results of the union are aggregated.
    ///
    /// This option defaults to SUM, where the score of an element is summed across the inputs where it exists.
    /// When this option is set to either MIN or MAX, the resulting set will contain the minimum or maximum score
    /// of an element across the inputs where it exists.
    pub fn aggregate(self, aggregate: ZAggregate) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("AGGREGATE").arg(aggregate),
        }
    }

    /// The number of elements in the resulting sorted set at destination.
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<usize>> + 'a>> {
        self.sorted_set_commands.send_into(self.cmd)
    }
}

pub enum ZWhere {
    Min,
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

/// Builder for the [zrandmember](crate::SortedSetCommands::zrandmember) command
pub struct ZRandMember<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZRandMember<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// The randomly selected element, or nil when key does not exist.
    pub fn execute<E>(self) -> Pin<Box<dyn Future<Output = Result<E>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands.send_into(self.cmd)
    }

    /// If the provided count argument is positive, return an array of distinct elements.
    /// The array's length is either count or the sorted set's cardinality (ZCARD), whichever is lower.
    ///
    /// If called with a negative count, the behavior changes and the command is allowed
    /// to return the same element multiple times. In this case, the number of returned elements
    /// is the absolute value of the specified count.
    pub fn count(self, count: isize) -> ZRandMemberCount<'a, T> {
        ZRandMemberCount {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg(count),
        }
    }
}

/// Builder for the [zrandmember](crate::SortedSetCommands::zrandmember) command
pub struct ZRandMemberCount<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZRandMemberCount<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// The result of the intersection
    pub fn execute<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands.send_into(self.cmd)
    }

    /// The result of the intersection with scores
    pub fn with_scores<E>(self) -> Pin<Box<dyn Future<Output = Result<Vec<(E, f64)>>> + 'a>>
    where
        E: FromValue,
    {
        self.sorted_set_commands
            .send_into_tuple_vec(self.cmd.arg("WITHSCORES"))
    }
}

/// Builder for the [zrangestore](crate::SortedSetCommands::zrangestore) command
pub struct ZRangeStore<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T> ZRangeStore<'a, T>
where
    T: SortedSetCommands + ?Sized,
{
    /// When the `ByScore` option is provided, the command behaves like `ZRANGEBYSCORE` and returns
    /// the range of elements from the sorted set having scores equal or between `start` and `stop`.
    ///
    /// When the `ByLex` option is used, the command behaves like `ZRANGEBYLEX` and returns the range
    /// of elements from the sorted set between the `start` and `stop` lexicographical closed range intervals.
    pub fn sort_by(self, sort_by: ZRangeSortBy) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg(sort_by),
        }
    }

    /// Using the REV option reverses the sorted set, with index 0 as the element with the highest score.
    pub fn rev(self) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("REV"),
        }
    }

    /// The optional LIMIT argument can be used to obtain a sub-range from the matching elements
    /// (similar to SELECT LIMIT offset, count in SQL).
    ///
    /// A negative `count` returns all elements from the `offset`.
    ///
    /// Keep in mind that if `offset` is large, the sorted set needs to be traversed for `offset`
    /// elements before getting to the elements to return, which can add up to O(N) time complexity.
    pub fn limit(self, offset: usize, count: isize) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("LIMIT").arg(offset).arg(count),
        }
    }

    /// # Return
    /// the number of elements in the resulting sorted set.
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<usize>> + 'a>> {
        self.sorted_set_commands.send_into(self.cmd)
    }
}

/// Builder for the [zscan](crate::SortedSetCommands::zscan) command
pub struct ZScan<'a, T: SortedSetCommands + ?Sized> {
    sorted_set_commands: &'a T,
    cmd: Command,
}

impl<'a, T: SortedSetCommands + ?Sized> ZScan<'a, T> {
    /// # Returns
    /// A tuple where
    /// * The first value is the cursor as an unsigned 64 bit number
    /// * The second value is a list of members and their scores in a Vec of Tuples
    pub fn execute<M>(self) -> Pin<Box<dyn Future<Output = Result<ZScanResult<M>>> + 'a>>
    where
        M: FromValue,
    {
        self.sorted_set_commands.send_into(self.cmd)
    }

    pub fn match_<P>(self, pattern: P) -> Self
    where
        P: Into<BulkString>,
    {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("MATCH").arg(pattern),
        }
    }

    pub fn count(self, count: usize) -> Self {
        Self {
            sorted_set_commands: self.sorted_set_commands,
            cmd: self.cmd.arg("COUNT").arg(count),
        }
    }
}

#[derive(Debug)]
pub struct ZScanResult<M>
where
    M: FromValue,
{
    pub cursor: u64,
    pub elements: Vec<(M, f64)>,
}

impl<M> FromValue for ZScanResult<M>
where
    M: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        let mut values: Vec<Value> = value.into()?;

        match (values.pop(), values.pop(), values.pop()) {
            (Some(elements), Some(cursor), None) => Ok(ZScanResult {
                cursor: cursor.into()?,
                elements: elements.into_tuple_vec::<M, f64>()?,
            }),
            _ => Err(Error::Internal("unexpected hscan result".to_owned())),
        }
    }
}
