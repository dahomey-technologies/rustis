use crate::{
    cmd,
    resp::{Array, BulkString, FromValue, Value},
    Command, CommandSend, Error, IntoArgs, Result,
};
use futures::Future;
use std::pin::Pin;

/// A group of Redis commands related to Lists
///
/// # See Also
/// [Redis List Commands](https://redis.io/commands/?group=list)
pub trait ListCommands: CommandSend {
    /// Returns the element at index index in the list stored at key.
    ///
    /// # Return
    /// The requested element, or nil when index is out of range.
    ///
    /// # See Also
    /// [https://redis.io/commands/lindex/](https://redis.io/commands/lindex/)
    fn lindex<K, E>(&self, key: K, index: isize) -> Pin<Box<dyn Future<Output = Result<E>> + '_>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("LINDEX").arg(key).arg(index))
    }

    /// Inserts element in the list stored at key either before or after the reference value pivot.
    ///
    /// # Return
    /// The length of the list after the insert operation, or -1 when the value pivot was not found.
    ///
    /// # See Also
    /// [https://redis.io/commands/linsert/](https://redis.io/commands/linsert/)
    fn linsert<K, E>(
        &self,
        key: K,
        where_: LInsertWhere,
        pivot: E,
        element: E,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.send_into(cmd("LINSERT").arg(key).arg(where_).arg(pivot).arg(element))
    }

    /// Inserts element in the list stored at key either before or after the reference value pivot.
    ///
    /// # Return
    /// The length of the list at key.
    ///
    /// # See Also
    /// [https://redis.io/commands/llen/](https://redis.io/commands/llen/)
    fn llen<K>(&self, key: K) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("LLEN").arg(key))
    }

    /// Atomically returns and removes the first/last element (head/tail depending on the wherefrom argument)
    /// of the list stored at source, and pushes the element at the first/last element
    /// (head/tail depending on the whereto argument) of the list stored at destination.
    ///
    /// # Return
    /// The element being popped and pushed.
    ///
    /// # See Also
    /// [https://redis.io/commands/lmove/](https://redis.io/commands/lmove/)
    fn lmove<S, D, E>(
        &self,
        source: S,
        destination: D,
        where_from: LMoveWhere,
        where_to: LMoveWhere,
    ) -> Pin<Box<dyn Future<Output = Result<E>> + '_>>
    where
        S: Into<BulkString>,
        D: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(
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
    /// [https://redis.io/commands/lmpop/](https://redis.io/commands/lmpop/)
    fn lmpop<K, E>(
        &self,
        keys: K,
        where_: LMoveWhere,
        count: usize,
    ) -> Pin<Box<dyn Future<Output = Result<(String, Vec<E>)>> + '_>>
    where
        K: IntoArgs,
        E: FromValue,
    {
        self.send_into(
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
    /// [https://redis.io/commands/lpop/](https://redis.io/commands/lpop/)
    fn lpop<K, E>(&self, key: K, count: usize) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + '_>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("LPOP").arg(key).arg(count))
    }

    /// Returns the index of matching elements inside a Redis list.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpos/](https://redis.io/commands/lpos/)
    fn lpos<K, E>(&self, key: K, element: E) -> LPos<Self>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        LPos {
            list_commands: self,
            cmd: cmd("LPOS").arg(key).arg(element),
        }
    }

    /// Insert all the specified values at the head of the list stored at key
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpush/](https://redis.io/commands/lpush/)
    fn lpush<K, E>(&self, key: K, elements: E) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        E: IntoArgs,
    {
        self.send_into(cmd("LPUSH").arg(key).arg(elements))
    }

    /// Inserts specified values at the head of the list stored at key,
    /// only if key already exists and holds a list.
    ///
    /// # Return
    /// The length of the list after the push operation.
    ///
    /// # See Also
    /// [https://redis.io/commands/lpushx/](https://redis.io/commands/lpushx/)
    fn lpushx<K, E>(&self, key: K, elements: E) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        E: IntoArgs,
    {
        self.send_into(cmd("LPUSHX").arg(key).arg(elements))
    }

    /// Returns the specified elements of the list stored at key.
    ///
    /// # Return
    /// The list of elements in the specified range.
    ///
    /// # See Also
    /// [https://redis.io/commands/lrange/](https://redis.io/commands/lrange/)
    fn lrange<K, E>(
        &self,
        key: K,
        start: isize,
        stop: isize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + '_>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("LRANGE").arg(key).arg(start).arg(stop))
    }

    /// Removes the first count occurrences of elements equal to element from the list stored at key.
    ///
    /// # Return
    /// The number of removed elements.
    ///
    /// # See Also
    /// [https://redis.io/commands/lrem/](https://redis.io/commands/lrem/)
    fn lrem<K, E>(
        &self,
        key: K,
        count: isize,
        element: E,
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.send_into(cmd("LREM").arg(key).arg(count).arg(element))
    }

    /// Sets the list element at index to element.
    ///
    /// # See Also
    /// [https://redis.io/commands/lset/](https://redis.io/commands/lset/)
    fn lset<K, E>(
        &self,
        key: K,
        index: isize,
        element: E,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
    {
        self.send_into(cmd("LSET").arg(key).arg(index).arg(element))
    }

    /// Trim an existing list so that it will contain only the specified range of elements specified.
    ///
    /// # See Also
    /// [https://redis.io/commands/ltrim/](https://redis.io/commands/ltrim/)
    fn ltrim<K>(
        &self,
        key: K,
        start: isize,
        stop: isize,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>
    where
        K: Into<BulkString>,
    {
        self.send_into(cmd("LTRIM").arg(key).arg(start).arg(stop))
    }

    /// Removes and returns the first elements of the list stored at key.
    ///
    /// # Return
    /// The list of popped elements, or empty collection when key does not exist.
    ///
    /// # See Also
    /// [https://redis.io/commands/rpop/](https://redis.io/commands/rpop/)
    fn rpop<K, E>(&self, key: K, count: usize) -> Pin<Box<dyn Future<Output = Result<Vec<E>>> + '_>>
    where
        K: Into<BulkString>,
        E: FromValue,
    {
        self.send_into(cmd("RPOP").arg(key).arg(count))
    }

    /// Insert all the specified values at the tail of the list stored at key
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [https://redis.io/commands/rpush/](https://redis.io/commands/rpush/)
    fn rpush<K, E>(&self, key: K, elements: E) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        E: IntoArgs,
    {
        self.send_into(cmd("RPUSH").arg(key).arg(elements))
    }

    /// Inserts specified values at the tail of the list stored at key,
    /// only if key already exists and holds a list.
    ///
    /// # Return
    /// The length of the list after the push operations.
    ///
    /// # See Also
    /// [https://redis.io/commands/rpushx/](https://redis.io/commands/rpushx/)
    fn rpushx<K, E>(&self, key: K, elements: E) -> Pin<Box<dyn Future<Output = Result<usize>> + '_>>
    where
        K: Into<BulkString>,
        E: IntoArgs,
    {
        self.send_into(cmd("RPUSHX").arg(key).arg(elements))
    }
}

pub enum LInsertWhere {
    Before,
    After,
}

impl From<LInsertWhere> for BulkString {
    fn from(w: LInsertWhere) -> Self {
        match w {
            LInsertWhere::Before => BulkString::Str("BEFORE"),
            LInsertWhere::After => BulkString::Str("AFTER"),
        }
    }
}

pub enum LMoveWhere {
    Left,
    Right,
}

impl From<LMoveWhere> for BulkString {
    fn from(w: LMoveWhere) -> Self {
        match w {
            LMoveWhere::Left => BulkString::Str("LEFT"),
            LMoveWhere::Right => BulkString::Str("RIGHT"),
        }
    }
}

/// Builder for the [lpos](crate::ListCommands::lpos) command
pub struct LPos<'a, T: ListCommands + ?Sized> {
    list_commands: &'a T,
    cmd: Command,
}

impl<'a, T: ListCommands + ?Sized> LPos<'a, T> {
    /// Returns the integer representing the matching element, or nil if there is no match. 
    /// 
    /// However, if the COUNT option is given the command returns an array 
    /// (empty if there are no matches).
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<Option<usize>>> + 'a>>
    {
        self.list_commands.send_into(self.cmd)
    }

    /// The RANK option specifies the "rank" of the first element to return,
    /// in case there are multiple matches.
    pub fn rank(self, rank: usize) -> Self {
        LPos {
            list_commands: self.list_commands,
            cmd: self.cmd.arg("RANK").arg(rank),
        }
    }

    /// Sometimes we want to return not just the Nth matching element,
    /// but the position of all the first N matching elements.
    /// This can be achieved using the COUNT option.
    pub fn count(self, num_matches: usize) -> LPosCount<'a, T> {
        LPosCount {
            list_commands: self.list_commands,
            cmd: self.cmd.arg("COUNT").arg(num_matches),
        }
    }

    /// the MAXLEN option tells the command to compare the provided
    /// element only with a given maximum number of list items.
    pub fn max_len(self, len: usize) -> Self {
        LPos {
            list_commands: self.list_commands,
            cmd: self.cmd.arg("MAXLEN").arg(len),
        }
    }
}

/// Builder for the [lpos](crate::ListCommands::lpos) command
pub struct LPosCount<'a, T: ListCommands + ?Sized> {
    list_commands: &'a T,
    cmd: Command,
}

impl<'a, T: ListCommands + ?Sized> LPosCount<'a, T> {
    /// Returns an array of integers representing the matching elements
    ///  (empty if there are no matches).
    pub fn execute(self) -> Pin<Box<dyn Future<Output = Result<Vec<usize>>> + 'a>>
    {
        self.list_commands.send_into(self.cmd)
    }

    /// The RANK option specifies the "rank" of the first element to return,
    /// in case there are multiple matches.
    pub fn rank(self, rank: usize) -> Self {
        LPosCount {
            list_commands: self.list_commands,
            cmd: self.cmd.arg("RANK").arg(rank),
        }
    }

    /// the MAXLEN option tells the command to compare the provided
    /// element only with a given maximum number of list items.
    pub fn max_len(self, len: usize) -> Self {
        LPosCount {
            list_commands: self.list_commands,
            cmd: self.cmd.arg("MAXLEN").arg(len),
        }
    }
}

impl<E> FromValue for (String, Vec<E>)
where
    E: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(mut elements)) => {
                match (elements.pop(), elements.pop(), elements.pop()) {
                    (Some(elements), Some(key), None) => Ok((key.into()?, elements.into()?)),
                    _ => Err(Error::Internal("Cannot parse LMPOP result".to_owned())),
                }
            }
            Value::Array(Array::Nil) => Ok(("".to_owned(), Vec::new())),
            _ => Err(Error::Internal("Cannot parse LMPOP result".to_owned())),
        }
    }
}
