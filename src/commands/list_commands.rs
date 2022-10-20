use crate::{
    resp::{
        cmd, BulkString, CommandArgs, FromSingleValueArray, FromValue, IntoArgs,
        SingleArgOrCollection,
    },
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to [`Lists`](https://redis.io/docs/data-types/lists/)
///
/// # See Also
/// [Redis List Commands](https://redis.io/commands/?group=list)
pub trait ListCommands<T>: PrepareCommand<T> {
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
    ) -> CommandResult<T, E>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(
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
    ) -> CommandResult<T, Option<(String, Vec<E>)>>
    where
        K: Into<BulkString>,
        E: FromValue,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(
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
    fn blpop<K, KK, K1, V>(&mut self, keys: KK, timeout: f64) -> CommandResult<T, Option<(K1, V)>>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue,
        V: FromValue,
    {
        self.prepare_command(cmd("BLPOP").arg(keys).arg(timeout))
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
    fn brpop<K, KK, K1, V>(&mut self, keys: KK, timeout: f64) -> CommandResult<T, Option<(K1, V)>>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        K1: FromValue,
        V: FromValue,
    {
        self.prepare_command(cmd("BRPOP").arg(keys).arg(timeout))
    }

    /// Returns the element at index index in the list stored at key.
    ///
    /// # Return
    /// The requested element, or nil when index is out of range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lindex/>](https://redis.io/commands/lindex/)
    #[must_use]
    fn lindex<K, E>(&mut self, key: K, index: isize) -> CommandResult<T, E>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(cmd("LINDEX").arg(key).arg(index))
    }

    /// Inserts element in the list stored at key either before or after the reference value pivot.
    ///
    /// # Return
    /// The length of the list after the insert operation, or -1 when the value pivot was not found.
    ///
    /// # See Also
    /// [<https://redis.io/commands/linsert/>](https://redis.io/commands/linsert/)
    #[must_use]
    fn linsert<K, E>(
        &mut self,
        key: K,
        where_: LInsertWhere,
        pivot: E,
        element: E,
    ) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.prepare_command(cmd("LINSERT").arg(key).arg(where_).arg(pivot).arg(element))
    }

    /// Inserts element in the list stored at key either before or after the reference value pivot.
    ///
    /// # Return
    /// The length of the list at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/llen/>](https://redis.io/commands/llen/)
    #[must_use]
    fn llen<K>(&mut self, key: K) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("LLEN").arg(key))
    }

    /// Atomically returns and removes the first/last element (head/tail depending on the wherefrom argument)
    /// of the list stored at source, and pushes the element at the first/last element
    /// (head/tail depending on the whereto argument) of the list stored at destination.
    ///
    /// # Return
    /// The element being popped and pushed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lmove/>](https://redis.io/commands/lmove/)
    #[must_use]
    fn lmove<S, D, E>(
        &mut self,
        source: S,
        destination: D,
        where_from: LMoveWhere,
        where_to: LMoveWhere,
    ) -> CommandResult<T, E>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
        E: FromValue,
    {
        self.prepare_command(
            cmd("LMOVE")
                .arg(source)
                .arg(destination)
                .arg(where_from)
                .arg(where_to),
        )
    }

    /// Pops one or more elements from the first non-empty list key from the list of provided key names.
    ///
    /// # Return
    /// Tuple composed by the name of the key from which elements were popped and the list of popped element
    ///
    /// # See Also
    /// [<https://redis.io/commands/lmpop/>](https://redis.io/commands/lmpop/)
    #[must_use]
    fn lmpop<K, E, C>(
        &mut self,
        keys: C,
        where_: LMoveWhere,
        count: usize,
    ) -> CommandResult<T, (String, Vec<E>)>
    where
        K: Into<BulkString>,
        E: FromValue,
        C: SingleArgOrCollection<K>,
    {
        self.prepare_command(
            cmd("LMPOP")
                .arg(keys.num_args())
                .arg(keys)
                .arg(where_)
                .arg("COUNT")
                .arg(count),
        )
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// # Return
    /// The list of popped elements, or empty collection when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lpop/>](https://redis.io/commands/lpop/)
    #[must_use]
    fn lpop<K, E, A>(&mut self, key: K, count: usize) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        E: FromValue,
        A: FromSingleValueArray<E>,
    {
        self.prepare_command(cmd("LPOP").arg(key).arg(count))
    }

    /// Returns the index of matching elements inside a Redis list.
    ///
    /// # Return
    /// The integer representing the matching element, or nil if there is no match.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lpos/>](https://redis.io/commands/lpos/)
    #[must_use]
    fn lpos<K, E>(
        &mut self,
        key: K,
        element: E,
        rank: Option<usize>,
        max_len: Option<usize>,
    ) -> CommandResult<T, Option<usize>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.prepare_command(
            cmd("LPOS")
                .arg(key)
                .arg(element)
                .arg(rank.map(|r| ("RANK", r)))
                .arg(max_len.map(|l| ("MAXLEN", l))),
        )
    }

    /// Returns the index of matching elements inside a Redis list.
    ///
    /// # Return
    /// An array of integers representing the matching elements.
    /// (empty if there are no matches).
    ///
    /// # See Also
    /// [<https://redis.io/commands/lpos/>](https://redis.io/commands/lpos/)
    #[must_use]
    fn lpos_with_count<K, E, A>(
        &mut self,
        key: K,
        element: E,
        num_matches: usize,
        rank: Option<usize>,
        max_len: Option<usize>,
    ) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        A: FromSingleValueArray<usize>,
    {
        self.prepare_command(
            cmd("LPOS")
                .arg(key)
                .arg(element)
                .arg(rank.map(|r| ("RANK", r)))
                .arg("COUNT")
                .arg(num_matches)
                .arg(max_len.map(|l| ("MAXLEN", l))),
        )
    }

    /// Insert all the specified values at the head of the list stored at key
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lpush/>](https://redis.io/commands/lpush/)
    #[must_use]
    fn lpush<K, E, C>(&mut self, key: K, elements: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        C: SingleArgOrCollection<E>,
    {
        self.prepare_command(cmd("LPUSH").arg(key).arg(elements))
    }

    /// Inserts specified values at the head of the list stored at key,
    /// only if key already exists and holds a list.
    ///
    /// # Return
    /// The length of the list after the push operation.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lpushx/>](https://redis.io/commands/lpushx/)
    #[must_use]
    fn lpushx<K, E, C>(&mut self, key: K, elements: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        C: SingleArgOrCollection<E>,
    {
        self.prepare_command(cmd("LPUSHX").arg(key).arg(elements))
    }

    /// Returns the specified elements of the list stored at key.
    ///
    /// # Return
    /// The list of elements in the specified range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lrange/>](https://redis.io/commands/lrange/)
    #[must_use]
    fn lrange<K, E, A>(&mut self, key: K, start: isize, stop: isize) -> CommandResult<T, A>
    where
        K: Into<BulkString>,
        E: FromValue,
        A: FromSingleValueArray<E>,
    {
        self.prepare_command(cmd("LRANGE").arg(key).arg(start).arg(stop))
    }

    /// Removes the first count occurrences of elements equal to element from the list stored at key.
    ///
    /// # Return
    /// The number of removed elements.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lrem/>](https://redis.io/commands/lrem/)
    #[must_use]
    fn lrem<K, E>(&mut self, key: K, count: isize, element: E) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.prepare_command(cmd("LREM").arg(key).arg(count).arg(element))
    }

    /// Sets the list element at index to element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lset/>](https://redis.io/commands/lset/)
    #[must_use]
    fn lset<K, E>(&mut self, key: K, index: isize, element: E) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.prepare_command(cmd("LSET").arg(key).arg(index).arg(element))
    }

    /// Trim an existing list so that it will contain only the specified range of elements specified.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ltrim/>](https://redis.io/commands/ltrim/)
    #[must_use]
    fn ltrim<K>(&mut self, key: K, start: isize, stop: isize) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("LTRIM").arg(key).arg(start).arg(stop))
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// # Return
    /// The list of popped elements, or empty collection when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rpop/>](https://redis.io/commands/rpop/)
    #[must_use]
    fn rpop<K, E, C>(&mut self, key: K, count: usize) -> CommandResult<T, C>
    where
        K: Into<BulkString>,
        E: FromValue,
        C: FromSingleValueArray<E>,
    {
        self.prepare_command(cmd("RPOP").arg(key).arg(count))
    }

    /// Insert all the specified values at the tail of the list stored at key
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rpush/>](https://redis.io/commands/rpush/)
    #[must_use]
    fn rpush<K, E, C>(&mut self, key: K, elements: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        C: SingleArgOrCollection<E>,
    {
        self.prepare_command(cmd("RPUSH").arg(key).arg(elements))
    }

    /// Inserts specified values at the tail of the list stored at key,
    /// only if key already exists and holds a list.
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rpushx/>](https://redis.io/commands/rpushx/)
    #[must_use]
    fn rpushx<K, E, C>(&mut self, key: K, elements: C) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        C: SingleArgOrCollection<E>,
    {
        self.prepare_command(cmd("RPUSHX").arg(key).arg(elements))
    }
}

pub enum LInsertWhere {
    Before,
    After,
}

impl IntoArgs for LInsertWhere {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            LInsertWhere::Before => BulkString::Str("BEFORE"),
            LInsertWhere::After => BulkString::Str("AFTER"),
        })
    }
}

pub enum LMoveWhere {
    Left,
    Right,
}

impl IntoArgs for LMoveWhere {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            LMoveWhere::Left => BulkString::Str("LEFT"),
            LMoveWhere::Right => BulkString::Str("RIGHT"),
        })
    }
}
