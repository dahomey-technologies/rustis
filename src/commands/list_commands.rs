use serde::de::DeserializeOwned;

use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, CommandArgs, Response, cmd},
};

/// A group of Redis commands related to [`Lists`](https://redis.io/docs/data-types/lists/)
///
/// # See Also
/// [Redis List Commands](https://redis.io/commands/?group=list)
pub trait ListCommands<'a>: Sized {
    /// Returns the element at index index in the list stored at key.
    ///
    /// # Return
    /// The requested element, or nil when index is out of range.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lindex/>](https://redis.io/commands/lindex/)
    #[must_use]
    fn lindex<R: Response>(self, key: impl Args, index: isize) -> PreparedCommand<'a, Self, R> {
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
    fn linsert(
        self,
        key: impl Args,
        where_: LInsertWhere,
        pivot: impl Args,
        element: impl Args,
    ) -> PreparedCommand<'a, Self, usize> {
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
    fn llen(self, key: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn lmove<R: Response>(
        self,
        source: impl Args,
        destination: impl Args,
        where_from: LMoveWhere,
        where_to: LMoveWhere,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn lmpop<R: Response + DeserializeOwned>(
        self,
        keys: impl Args,
        where_: LMoveWhere,
        count: usize,
    ) -> PreparedCommand<'a, Self, (String, Vec<R>)> {
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
    fn lpop<R: Response>(self, key: impl Args, count: usize) -> PreparedCommand<'a, Self, R> {
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
    fn lpos(
        self,
        key: impl Args,
        element: impl Args,
        rank: Option<usize>,
        max_len: Option<usize>,
    ) -> PreparedCommand<'a, Self, Option<usize>> {
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
    fn lpos_with_count<R: Response>(
        self,
        key: impl Args,
        element: impl Args,
        num_matches: usize,
        rank: Option<usize>,
        max_len: Option<usize>,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn lpush(self, key: impl Args, elements: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn lpushx(self, key: impl Args, elements: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn lrange<R: Response>(
        self,
        key: impl Args,
        start: isize,
        stop: isize,
    ) -> PreparedCommand<'a, Self, R> {
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
    fn lrem(
        self,
        key: impl Args,
        count: isize,
        element: impl Args,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("LREM").arg(key).arg(count).arg(element))
    }

    /// Sets the list element at index to element.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lset/>](https://redis.io/commands/lset/)
    #[must_use]
    fn lset(
        self,
        key: impl Args,
        index: isize,
        element: impl Args,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("LSET").arg(key).arg(index).arg(element))
    }

    /// Trim an existing list so that it will contain only the specified range of elements specified.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ltrim/>](https://redis.io/commands/ltrim/)
    #[must_use]
    fn ltrim(self, key: impl Args, start: isize, stop: isize) -> PreparedCommand<'a, Self, ()> {
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
    fn rpop<R: Response>(self, key: impl Args, count: usize) -> PreparedCommand<'a, Self, R> {
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
    fn rpush(self, key: impl Args, elements: impl Args) -> PreparedCommand<'a, Self, usize> {
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
    fn rpushx(self, key: impl Args, elements: impl Args) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("RPUSHX").arg(key).arg(elements))
    }
}

/// Where option for the [`linsert`](ListCommands::linsert) command.
pub enum LInsertWhere {
    Before,
    After,
}

impl Args for LInsertWhere {
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

impl Args for LMoveWhere {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            LMoveWhere::Left => "LEFT",
            LMoveWhere::Right => "RIGHT",
        });
    }
}
