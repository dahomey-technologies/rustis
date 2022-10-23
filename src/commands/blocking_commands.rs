use crate::{
    prepare_command,
    resp::{cmd, BulkString, FromValue, SingleArgOrCollection},
    Future, LMoveWhere, MonitorStream, PreparedCommand, ZMPopResult, ZWhere,
};

pub type BZpopMinMaxResult<K, E> = Option<Vec<(K, E, f64)>>;

/// A group of blocking commands
pub trait BlockingCommands {
    /// This command is the blocking variant of [`lmove`](crate::ListCommands::lmove).
    ///
    /// # Return
    /// the element being popped from `source` and pushed to `destination`.
    /// If timeout is reached, a None reply is returned.
    ///
    /// # See Also
    /// [<https://redis.io/commands/blmove/>](https://redis.io/commands/blmove/)
    #[must_use]
    fn blmove<S, D, E>(
        &mut self,
        source: S,
        destination: D,
        where_from: LMoveWhere,
        where_to: LMoveWhere,
        timeout: f64,
    ) -> PreparedCommand<Self, E>
    where
        Self: Sized,
        S: Into<BulkString>,
        D: Into<BulkString>,
        E: FromValue,
    {
        prepare_command(
            self,
            cmd("BLMOVE")
                .arg(source)
                .arg(destination)
                .arg(where_from)
                .arg(where_to)
                .arg(timeout),
        )
    }

    /// This command is the blocking variant of [`lmpop`](crate::ListCommands::lmpop).
    ///
    /// # Return
    /// - None when no element could be popped, and timeout is reached.
    /// - Tuple composed by the name of the key from which elements were popped and the list of popped element
    ///
    /// # See Also
    /// [<https://redis.io/commands/blmpop/>](https://redis.io/commands/blmpop/)
    #[must_use]
    fn blmpop<K, E, C>(
        &mut self,
        timeout: f64,
        keys: C,
        where_: LMoveWhere,
        count: usize,
    ) -> PreparedCommand<Self, Option<(String, Vec<E>)>>
    where
        Self: Sized,
        K: Into<BulkString>,
        E: FromValue,
        C: SingleArgOrCollection<K>,
    {
        prepare_command(
            self,
            cmd("BLMPOP")
                .arg(timeout)
                .arg(keys.num_args())
                .arg(keys)
                .arg(where_)
                .arg("COUNT")
                .arg(count),
        )
    }

    /// This command is a blocking list pop primitive.
    ///
    /// It is the blocking version of [`lpop`](crate::ListCommands::lpop) because it
    /// blocks the connection when there are no elements to pop from any of the given lists.
    ///
    /// An element is popped from the head of the first list that is non-empty,
    /// with the given keys being checked in the order that they are given.
    ///
    /// # Return
    /// - `None` when no element could be popped and the timeout expired
    /// - a tuple with the first element being the name of the key where an element was popped
    /// and the second element being the value of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/blpop/>](https://redis.io/commands/blpop/)
    #[must_use]
    fn blpop<K, KK, K1, V>(
        &mut self,
        keys: KK,
        timeout: f64,
    ) -> PreparedCommand<Self, Option<(K1, V)>>
    where
        Self: Sized,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue,
        V: FromValue,
    {
        prepare_command(self, cmd("BLPOP").arg(keys).arg(timeout))
    }

    /// This command is a blocking list pop primitive.
    ///
    /// It is the blocking version of [`rpop`](crate::ListCommands::rpop) because it
    /// blocks the connection when there are no elements to pop from any of the given lists.
    ///
    /// An element is popped from the tail of the first list that is non-empty,
    /// with the given keys being checked in the order that they are given.
    ///
    /// # Return
    /// - `None` when no element could be popped and the timeout expired
    /// - a tuple with the first element being the name of the key where an element was popped
    /// and the second element being the value of the popped element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/brpop/>](https://redis.io/commands/brpop/)
    #[must_use]
    fn brpop<K, KK, K1, V>(
        &mut self,
        keys: KK,
        timeout: f64,
    ) -> PreparedCommand<Self, Option<(K1, V)>>
    where
        Self: Sized,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue,
        V: FromValue,
    {
        prepare_command(self, cmd("BRPOP").arg(keys).arg(timeout))
    }

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
        &mut self,
        timeout: f64,
        keys: C,
        where_: ZWhere,
        count: usize,
    ) -> PreparedCommand<Self, Option<ZMPopResult<E>>>
    where
        Self: Sized,
        K: Into<BulkString>,
        C: SingleArgOrCollection<K>,
        E: FromValue,
    {
        prepare_command(
            self,
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
    fn bzpopmax<K, KK, E, K1>(
        &mut self,
        keys: KK,
        timeout: f64,
    ) -> PreparedCommand<Self, BZpopMinMaxResult<K1, E>>
    where
        Self: Sized,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue,
        E: FromValue,
    {
        prepare_command(self, cmd("BZPOPMAX").arg(keys).arg(timeout))
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
    fn bzpopmin<K, KK, E, K1>(
        &mut self,
        keys: KK,
        timeout: f64,
    ) -> PreparedCommand<Self, BZpopMinMaxResult<K1, E>>
    where
        Self: Sized,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue,
        E: FromValue,
    {
        prepare_command(self, cmd("BZPOPMIN").arg(keys).arg(timeout))
    }

    /// Debugging command that streams back every command processed by the Redis server.
    ///
    /// # See Also
    /// [<https://redis.io/commands/monitor/>](https://redis.io/commands/monitor/)
    #[must_use]
    fn monitor(&mut self) -> Future<MonitorStream>;
}
