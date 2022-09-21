use crate::resp::BulkString;
use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet}, hash::BuildHasher};

pub const NONE_ARG: Option<BulkString> = None;

#[derive(Debug)]
pub enum CommandArgs {
    Empty,
    Single(BulkString),
    Array2([BulkString; 2]),
    Array3([BulkString; 3]),
    Array4([BulkString; 4]),
    Vec(Vec<BulkString>),
}

impl CommandArgs {
    /// Return the first command
    pub(crate) fn first(&self) -> &BulkString {
        match self {
            CommandArgs::Empty => {
                unimplemented!("Cannot get first argument because arguments are empty")
            }
            CommandArgs::Single(s) => s,
            CommandArgs::Array2(a) => &a[0],
            CommandArgs::Array3(a) => &a[0],
            CommandArgs::Array4(a) => &a[0],
            CommandArgs::Vec(v) => &v[0],
        }
    }

    #[must_use]
    pub fn arg<A>(self, args: A) -> Self
    where
        A: IntoArgs,
    {
        args.into_args(self)
    }

    #[must_use]
    pub fn arg_if<A>(self, condition: bool, arg: A) -> Self
    where
        A: IntoArgs,
    {
        if condition {
            arg.into_args(self)
        } else {
            self
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            CommandArgs::Empty => 0,
            CommandArgs::Single(_) => 1,
            CommandArgs::Array2(_) => 2,
            CommandArgs::Array3(_) => 3,
            CommandArgs::Array4(_) => 4,
            CommandArgs::Vec(v) => v.len(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
       self.len() == 0
    }
}

impl Default for CommandArgs {
    fn default() -> Self {
        CommandArgs::Empty
    }
}

/// Types compatible with command args
pub trait IntoArgs {
    fn into_args(self, args: CommandArgs) -> CommandArgs;
    fn num_args(&self) -> usize {
        unimplemented!()
    }
}

impl<T> IntoArgs for T
where
    T: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match args {
            CommandArgs::Empty => CommandArgs::Single(self.into()),
            CommandArgs::Single(a) => CommandArgs::Array2([a, self.into()]),
            CommandArgs::Array2(a) => {
                let [item1, item2] = a;
                CommandArgs::Array3([item1, item2, self.into()])
            }
            CommandArgs::Array3(a) => {
                let [item1, item2, item3] = a;
                CommandArgs::Array4([item1, item2, item3, self.into()])
            }
            CommandArgs::Array4(a) => {
                let [item1, item2, item3, item4] = a;
                CommandArgs::Vec(vec![item1, item2, item3, item4, self.into()])
            }
            CommandArgs::Vec(mut vec) => {
                vec.push(self.into());
                CommandArgs::Vec(vec)
            }
        }
    }

    fn num_args(&self) -> usize {
        1
    }
}

impl<T> IntoArgs for Option<T>
where
    T: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            Some(s) => s.into_args(args),
            None => args,
        }
    }

    fn num_args(&self) -> usize {
        match self {
            Some(_) => 1,
            None => 0,
        }
    }
}

impl<T, const N: usize> IntoArgs for [T; N]
where
    T: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    fn num_args(&self) -> usize {
        N
    }
}

impl<T> IntoArgs for Vec<T>
where
    T: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T, S: BuildHasher> IntoArgs for HashSet<T, S>
where
    T: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T> IntoArgs for BTreeSet<T>
where
    T: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<K, V, S: BuildHasher> IntoArgs for HashMap<K, V, S>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for (key, value) in self {
            args = key.into_args(args);
            args = value.into_args(args);
        }
        args
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<K, V> IntoArgs for BTreeMap<K, V>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for (key, value) in self {
            args = key.into_args(args);
            args = value.into_args(args);
        }
        args
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T, U> IntoArgs for (T, U)
where
    T: IntoArgs,
    U: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let args = self.0.into_args(args);
        self.1.into_args(args)
    }

    fn num_args(&self) -> usize {
        2
    }
}

impl<T, U, V> IntoArgs for (T, U, V)
where
    T: IntoArgs,
    U: IntoArgs,
    V: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let args = self.0.into_args(args);
        let args = self.1.into_args(args);
        self.2.into_args(args)
    }

    fn num_args(&self) -> usize {
        3
    }
}

/// Allow to merge `CommandArgs` in another `CommandArgs`
impl IntoArgs for CommandArgs {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            CommandArgs::Empty => args,
            CommandArgs::Single(s) => args.arg(s),
            CommandArgs::Array2(a) => args.arg(a),
            CommandArgs::Array3(a) => args.arg(a),
            CommandArgs::Array4(a) => args.arg(a),
            CommandArgs::Vec(v) => args.arg(v),
        }
    }
}

/// Generic Marker for Collections of `IntoArgs`
pub trait ArgsOrCollection<T>: IntoArgs
where
    T: IntoArgs,
{
}

impl<T, const N: usize> ArgsOrCollection<T> for [T; N] where T: IntoArgs {}
impl<T> ArgsOrCollection<T> for Vec<T> where T: IntoArgs {}
impl<T> ArgsOrCollection<T> for T where T: IntoArgs {}

/// Marker for collections of single items (directly convertible to `BulkStrings`) of `IntoArgs`
pub trait SingleArgOrCollection<T>: IntoArgs
where
    T: Into<BulkString>,
{
}

impl<T, const N: usize> SingleArgOrCollection<T> for [T; N] where T: Into<BulkString> {}
impl<T> SingleArgOrCollection<T> for Vec<T> where T: Into<BulkString> {}
impl<T, S: BuildHasher> SingleArgOrCollection<T> for HashSet<T, S> where T: Into<BulkString> {}
impl<T> SingleArgOrCollection<T> for BTreeSet<T> where T: Into<BulkString> {}
impl<T> SingleArgOrCollection<T> for T where T: Into<BulkString> {}

/// Marker for key/value collections of Args
pub trait KeyValueArgOrCollection<K, V>: IntoArgs
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
}

impl<K, V> KeyValueArgOrCollection<K, V> for Vec<(K, V)>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
}

impl<K, V, const N: usize> KeyValueArgOrCollection<K, V> for [(K, V); N]
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
}

impl<K, V> KeyValueArgOrCollection<K, V> for (K, V)
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
}

impl<K, V, S: BuildHasher> KeyValueArgOrCollection<K, V> for HashMap<K, V, S>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
}

impl<K, V> KeyValueArgOrCollection<K, V> for BTreeMap<K, V>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
}
