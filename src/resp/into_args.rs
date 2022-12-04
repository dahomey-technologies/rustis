use crate::resp::{BulkString, CommandArg, CommandArgs};
use smallvec::{smallvec, SmallVec};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::BuildHasher,
    iter::{once, Once},
};

/// Types compatible with command args
pub trait IntoArgs {
    fn into_args(self, args: CommandArgs) -> CommandArgs;
    fn num_args(&self) -> usize {
        1
    }
}

impl IntoArgs for CommandArg {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match args {
            CommandArgs::Empty => CommandArgs::Single(self),
            CommandArgs::Single(a) => CommandArgs::Array2([a, self]),
            CommandArgs::Array2(a) => {
                let [item1, item2] = a;
                CommandArgs::Array3([item1, item2, self])
            }
            CommandArgs::Array3(a) => {
                let [item1, item2, item3] = a;
                CommandArgs::Array4([item1, item2, item3, self])
            }
            CommandArgs::Array4(a) => {
                let [item1, item2, item3, item4] = a;
                CommandArgs::Array5([item1, item2, item3, item4, self])
            }
            CommandArgs::Array5(a) => {
                let [item1, item2, item3, item4, item5] = a;
                CommandArgs::Vec(smallvec![item1, item2, item3, item4, item5, self])
            }
            CommandArgs::Vec(mut vec) => {
                vec.push(self);
                CommandArgs::Vec(vec)
            }
        }
    }

    fn num_args(&self) -> usize {
        1
    }
}

impl IntoArgs for u8 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Unsigned(u64::from(self)))
    }
}

impl IntoArgs for i8 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Signed(i64::from(self)))
    }
}

impl IntoArgs for u16 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Unsigned(u64::from(self)))
    }
}

impl IntoArgs for i16 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Signed(i64::from(self)))
    }
}

impl IntoArgs for u32 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Unsigned(u64::from(self)))
    }
}

impl IntoArgs for i32 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Signed(i64::from(self)))
    }
}

impl IntoArgs for u64 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Unsigned(self))
    }
}

impl IntoArgs for i64 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Signed(self))
    }
}

impl IntoArgs for usize {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Unsigned(self as u64))
    }
}

impl IntoArgs for isize {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Signed(self as i64))
    }
}

impl IntoArgs for f32 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::F32(self))
    }
}

impl IntoArgs for f64 {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::F64(self))
    }
}

impl IntoArgs for bool {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Unsigned(u64::from(self)))
    }
}

impl IntoArgs for BulkString {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Binary(self.0))
    }
}

impl IntoArgs for &'static str {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::Str(self))
    }
}

impl IntoArgs for String {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::String(self))
    }
}

impl IntoArgs for char {
    #[inline]
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(CommandArg::String(self.to_string()))
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
            Some(t) => t.num_args(),
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
        self.iter().fold(0, |acc, t| acc + t.num_args())
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
        self.iter().fold(0, |acc, t| acc + t.num_args())
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
        self.iter().fold(0, |acc, t| acc + t.num_args())
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
        self.iter().fold(0, |acc, t| acc + t.num_args())
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
        self.iter().fold(0, |acc, t| acc + t.num_args())
    }
}

impl<K, V, S: BuildHasher> IntoArgs for HashMap<K, V, S>
where
    K: IntoArgs,
    V: IntoArgs,
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
        self.iter()
            .fold(0, |acc, (k, v)| acc + k.num_args() + v.num_args())
    }
}

impl<K, V> IntoArgs for BTreeMap<K, V>
where
    K: IntoArgs,
    V: IntoArgs,
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
        self.iter()
            .fold(0, |acc, (k, v)| acc + k.num_args() + v.num_args())
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
        self.0.num_args() + self.1.num_args()
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
        self.0.num_args() + self.1.num_args() + self.2.num_args()
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

/// Generic Marker for single arguments (no collections nor tuples)
pub trait SingleArg: IntoArgs {
    fn into_command_arg(self) -> CommandArg
    where
        Self: Sized,
    {
        let CommandArgs::Single(arg) = self.into_args(CommandArgs::Empty) else {
            panic!("Expected a single argument");
        };
        arg
    }
}

impl SingleArg for CommandArg {}
impl SingleArg for u8 {}
impl SingleArg for i8 {}
impl SingleArg for u16 {}
impl SingleArg for i16 {}
impl SingleArg for u32 {}
impl SingleArg for i32 {}
impl SingleArg for u64 {}
impl SingleArg for i64 {}
impl SingleArg for usize {}
impl SingleArg for isize {}
impl SingleArg for f32 {}
impl SingleArg for f64 {}
impl SingleArg for bool {}
impl SingleArg for char {}
impl SingleArg for &'static str {}
impl SingleArg for String {}
impl SingleArg for BulkString {}
impl<T: SingleArg> SingleArg for Option<T> {}

/// Generic Marker for Collections of `IntoArgs`
/// 
/// Each element of the collection can produce multiple args.
pub trait MultipleArgsCollection<T>: IntoArgs
where
    T: IntoArgs,
{
}

impl<T, const N: usize> MultipleArgsCollection<T> for [T; N] where T: IntoArgs {}
impl<T> MultipleArgsCollection<T> for Vec<T> where T: IntoArgs {}
impl<T> MultipleArgsCollection<T> for T where T: IntoArgs {}

/// Marker for collections of single items (directly convertible to `CommandArg`) of `IntoArgs`
/// 
/// Each element of the collection can only produce a single arg.
pub trait SingleArgCollection<T>: IntoArgs
where
    T: SingleArg,
{
    type IntoIter: Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter;
}

impl<T, const N: usize> SingleArgCollection<T> for [T; N]
where
    T: SingleArg,
{
    type IntoIter = std::array::IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T> SingleArgCollection<T> for Vec<T>
where
    T: SingleArg,
{
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<A, T> SingleArgCollection<T> for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: SingleArg,
{
    type IntoIter = smallvec::IntoIter<A>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T, S: BuildHasher> SingleArgCollection<T> for HashSet<T, S>
where
    T: SingleArg,
{
    type IntoIter = std::collections::hash_set::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T> SingleArgCollection<T> for BTreeSet<T>
where
    T: SingleArg,
{
    type IntoIter = std::collections::btree_set::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self)
    }
}

impl<T> SingleArgCollection<T> for T
where
    T: SingleArg,
{
    type IntoIter = Once<T>;

    fn into_iter(self) -> Self::IntoIter {
        once(self)
    }
}

/// Marker for key/value collections of Args
/// 
/// The key and the value can only produce a single arg each.
pub trait KeyValueArgsCollection<K, V>: IntoArgs
where
    K: SingleArg,
    V: SingleArg,
{
}

impl<K, V> KeyValueArgsCollection<K, V> for Vec<(K, V)>
where
    K: SingleArg,
    V: SingleArg,
{
}

impl<A, K, V> KeyValueArgsCollection<K, V> for SmallVec<A>
where
    A: smallvec::Array<Item = (K, V)>,
    K: SingleArg,
    V: SingleArg,
{
}

impl<K, V, const N: usize> KeyValueArgsCollection<K, V> for [(K, V); N]
where
    K: SingleArg,
    V: SingleArg,
{
}

impl<K, V> KeyValueArgsCollection<K, V> for (K, V)
where
    K: SingleArg,
    V: SingleArg,
{
}

impl<K, V, S: BuildHasher> KeyValueArgsCollection<K, V> for HashMap<K, V, S>
where
    K: SingleArg,
    V: SingleArg,
{
}

impl<K, V> KeyValueArgsCollection<K, V> for BTreeMap<K, V>
where
    K: SingleArg,
    V: SingleArg,
{
}
