use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CollectionResponse, CommandArgs, PrimitiveResponse, SingleArg, SingleArgCollection,
        ToArgs,
    },
};
use serde::de::DeserializeOwned;

/// A group of Redis commands related to [`Lists`](https://redis.io/docs/data-types/lists/)
///
/// # See Also
/// [Redis List Commands](https://redis.io/commands/?group=list)
pub trait ListCommands<'a> {
    /// Returns the element at index index in the list stored at key.
    ///
    /// # Return
    /// The requested element, or nil when index is out of range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lindex/>](https://redis.io/commands/lindex/)
    #[must_use]
    fn lindex<K, E>(self, key: K, index: isize) -> PreparedCommand<'a, Self, E>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse,
    {
        prepare_command(self, cmd("LINDEX").arg(key).arg(index))
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
        self,
        key: K,
        where_: LInsertWhere,
        pivot: E,
        element: E,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
    {
        prepare_command(
            self,
            cmd("LINSERT").arg(key).arg(where_).arg(pivot).arg(element),
        )
    }

    /// Inserts element in the list stored at key either before or after the reference value pivot.
    ///
    /// # Return
    /// The length of the list at key.
    ///
    /// # See Also
    /// [<https://redis.io/commands/llen/>](https://redis.io/commands/llen/)
    #[must_use]
    fn llen<K>(self, key: K) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("LLEN").arg(key))
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
        self,
        source: S,
        destination: D,
        where_from: LMoveWhere,
        where_to: LMoveWhere,
    ) -> PreparedCommand<'a, Self, E>
    where
        Self: Sized,
        S: SingleArg,
        D: SingleArg,
        E: PrimitiveResponse,
    {
        prepare_command(
            self,
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
        self,
        keys: C,
        where_: LMoveWhere,
        count: usize,
    ) -> PreparedCommand<'a, Self, (String, Vec<E>)>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
        C: SingleArgCollection<K>,
    {
        prepare_command(
            self,
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
    fn lpop<K, E, A>(self, key: K, count: usize) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
        A: CollectionResponse<E> + DeserializeOwned,
    {
        prepare_command(self, cmd("LPOP").arg(key).arg(count))
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
        self,
        key: K,
        element: E,
        rank: Option<usize>,
        max_len: Option<usize>,
    ) -> PreparedCommand<'a, Self, Option<usize>>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
    {
        prepare_command(
            self,
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
        self,
        key: K,
        element: E,
        num_matches: usize,
        rank: Option<usize>,
        max_len: Option<usize>,
    ) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        A: CollectionResponse<usize>,
    {
        prepare_command(
            self,
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
    fn lpush<K, E, C>(self, key: K, elements: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        C: SingleArgCollection<E>,
    {
        prepare_command(self, cmd("LPUSH").arg(key).arg(elements))
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
    fn lpushx<K, E, C>(self, key: K, elements: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        C: SingleArgCollection<E>,
    {
        prepare_command(self, cmd("LPUSHX").arg(key).arg(elements))
    }

    /// Returns the specified elements of the list stored at key.
    ///
    /// # Return
    /// The list of elements in the specified range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lrange/>](https://redis.io/commands/lrange/)
    #[must_use]
    fn lrange<K, E, A>(self, key: K, start: isize, stop: isize) -> PreparedCommand<'a, Self, A>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
        A: CollectionResponse<E> + DeserializeOwned,
    {
        prepare_command(self, cmd("LRANGE").arg(key).arg(start).arg(stop))
    }

    /// Removes the first count occurrences of elements equal to element from the list stored at key.
    ///
    /// # Return
    /// The number of removed elements.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lrem/>](https://redis.io/commands/lrem/)
    #[must_use]
    fn lrem<K, E>(self, key: K, count: isize, element: E) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
    {
        prepare_command(self, cmd("LREM").arg(key).arg(count).arg(element))
    }

    /// Sets the list element at index to element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lset/>](https://redis.io/commands/lset/)
    #[must_use]
    fn lset<K, E>(self, key: K, index: isize, element: E) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
    {
        prepare_command(self, cmd("LSET").arg(key).arg(index).arg(element))
    }

    /// Trim an existing list so that it will contain only the specified range of elements specified.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ltrim/>](https://redis.io/commands/ltrim/)
    #[must_use]
    fn ltrim<K>(self, key: K, start: isize, stop: isize) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("LTRIM").arg(key).arg(start).arg(stop))
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// # Return
    /// The list of popped elements, or empty collection when key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rpop/>](https://redis.io/commands/rpop/)
    #[must_use]
    fn rpop<K, E, C>(self, key: K, count: usize) -> PreparedCommand<'a, Self, C>
    where
        Self: Sized,
        K: SingleArg,
        E: PrimitiveResponse + DeserializeOwned,
        C: CollectionResponse<E> + DeserializeOwned,
    {
        prepare_command(self, cmd("RPOP").arg(key).arg(count))
    }

    /// Insert all the specified values at the tail of the list stored at key
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [<https://redis.io/commands/rpush/>](https://redis.io/commands/rpush/)
    #[must_use]
    fn rpush<K, E, C>(self, key: K, elements: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        C: SingleArgCollection<E>,
    {
        prepare_command(self, cmd("RPUSH").arg(key).arg(elements))
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
    fn rpushx<K, E, C>(self, key: K, elements: C) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        C: SingleArgCollection<E>,
    {
        prepare_command(self, cmd("RPUSHX").arg(key).arg(elements))
    }
}

/// Where option for the [`linsert`](ListCommands::linsert) command.
pub enum LInsertWhere {
    Before,
    After,
}

impl ToArgs for LInsertWhere {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            LInsertWhere::Before => "BEFORE",
            LInsertWhere::After => "AFTER",
        });
    }
}

/// Where option for the [`lmove`](ListCommands::lmove) command.
pub enum LMoveWhere {
    Left,
    Right,
}

impl ToArgs for LMoveWhere {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            LMoveWhere::Left => "LEFT",
            LMoveWhere::Right => "RIGHT",
        });
    }
}
