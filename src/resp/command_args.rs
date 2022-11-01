use crate::resp::BulkString;
use smallvec::{smallvec, SmallVec};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::BuildHasher,
    iter::{once, Once},
    ops::Deref,
};

#[derive(Debug)]
pub enum CommandArgs {
    Empty,
    Single(BulkString),
    Array2([BulkString; 2]),
    Array3([BulkString; 3]),
    Array4([BulkString; 4]),
    Array5([BulkString; 5]),
    Vec(SmallVec<[BulkString; 10]>),
}

impl CommandArgs {
    #[must_use]
    #[inline]
    pub fn arg<A>(self, args: A) -> Self
    where
        A: IntoArgs,
    {
        args.into_args(self)
    }

    #[must_use]
    #[inline]
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
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            CommandArgs::Empty => 0,
            CommandArgs::Single(_) => 1,
            CommandArgs::Array2(_) => 2,
            CommandArgs::Array3(_) => 3,
            CommandArgs::Array4(_) => 4,
            CommandArgs::Array5(_) => 5,
            CommandArgs::Vec(v) => v.len(),
        }
    }

    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for CommandArgs {
    fn default() -> Self {
        CommandArgs::Empty
    }
}

impl<'a> IntoIterator for &'a CommandArgs {
    type Item = &'a BulkString;
    type IntoIter = CommandArgsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CommandArgs::Empty => CommandArgsIterator::Empty,
            CommandArgs::Single(s) => CommandArgsIterator::Single(Some(s)),
            CommandArgs::Array2(a) => CommandArgsIterator::Iter(a.iter()),
            CommandArgs::Array3(a) => CommandArgsIterator::Iter(a.iter()),
            CommandArgs::Array4(a) => CommandArgsIterator::Iter(a.iter()),
            CommandArgs::Array5(a) => CommandArgsIterator::Iter(a.iter()),
            CommandArgs::Vec(a) => CommandArgsIterator::Iter(a.iter()),
        }
    }
}

#[derive(Clone)]
pub enum CommandArgsIterator<'a> {
    Empty,
    Single(Option<&'a BulkString>),
    Iter(std::slice::Iter<'a, BulkString>),
}

impl<'a> Iterator for CommandArgsIterator<'a> {
    type Item = &'a BulkString;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CommandArgsIterator::Empty => None,
            CommandArgsIterator::Single(s) => s.take(),
            CommandArgsIterator::Iter(i) => i.next(),
        }
    }
}

impl Deref for CommandArgs {
    type Target = [BulkString];

    fn deref(&self) -> &Self::Target {
        match self {
            CommandArgs::Empty => &[],
            CommandArgs::Single(s) => std::slice::from_ref(s),
            CommandArgs::Array2(a) => a,
            CommandArgs::Array3(a) => a,
            CommandArgs::Array4(a) => a,
            CommandArgs::Array5(a) => a,
            CommandArgs::Vec(v) => v,
        }
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
    #[inline]
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
                CommandArgs::Array5([item1, item2, item3, item4, self.into()])
            }
            CommandArgs::Array5(a) => {
                let [item1, item2, item3, item4, item5] = a;
                CommandArgs::Vec(smallvec![item1, item2, item3, item4, item5, self.into()])
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
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            Some(s) => s.into_args(args),
            None => args,
        }
    }

    #[inline]
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
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        N
    }
}

impl<T> IntoArgs for Vec<T>
where
    T: IntoArgs,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T, A> IntoArgs for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: IntoArgs,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T, S: BuildHasher> IntoArgs for HashSet<T, S>
where
    T: IntoArgs,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T> IntoArgs for BTreeSet<T>
where
    T: IntoArgs,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self {
            args = a.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<K, V, S: BuildHasher> IntoArgs for HashMap<K, V, S>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for (key, value) in self {
            args = key.into_args(args);
            args = value.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<K, V> IntoArgs for BTreeMap<K, V>
where
    K: Into<BulkString>,
    V: Into<BulkString>,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for (key, value) in self {
            args = key.into_args(args);
            args = value.into_args(args);
        }
        args
    }

    #[inline]
    fn num_args(&self) -> usize {
        self.len()
    }
}

impl<T, U> IntoArgs for (T, U)
where
    T: IntoArgs,
    U: IntoArgs,
{
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let args = self.0.into_args(args);
        self.1.into_args(args)
    }

    #[inline]
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
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let args = self.0.into_args(args);
        let args = self.1.into_args(args);
        self.2.into_args(args)
    }

    #[inline]
    fn num_args(&self) -> usize {
        3
    }
}

/// Allow to merge `CommandArgs` in another `CommandArgs`
impl IntoArgs for CommandArgs {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            CommandArgs::Empty => args,
            CommandArgs::Single(s) => args.arg(s),
            CommandArgs::Array2(a) => args.arg(a),
            CommandArgs::Array3(a) => args.arg(a),
            CommandArgs::Array4(a) => args.arg(a),
            CommandArgs::Array5(a) => args.arg(a),
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
    type IntoIter: Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter;
}

impl<T, const N: usize> SingleArgOrCollection<T> for [T; N]
where
    T: Into<BulkString>,
{
    type IntoIter = std::array::IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T> SingleArgOrCollection<T> for Vec<T>
where
    T: Into<BulkString>,
{
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<A, T> SingleArgOrCollection<T> for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: Into<BulkString>,
{
    type IntoIter = smallvec::IntoIter<A>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T, S: BuildHasher> SingleArgOrCollection<T> for HashSet<T, S>
where
    T: Into<BulkString>,
{
    type IntoIter = std::collections::hash_set::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T> SingleArgOrCollection<T> for BTreeSet<T>
where
    T: Into<BulkString>,
{
    type IntoIter = std::collections::btree_set::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T> SingleArgOrCollection<T> for T
where
    T: Into<BulkString>,
{
    type IntoIter = Once<T>;

    fn into_iter(self) -> Self::IntoIter {
        once(self)
    }
}

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
