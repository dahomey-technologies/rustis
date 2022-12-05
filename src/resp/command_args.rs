use crate::resp::{CommandArg, IntoArgs};
use smallvec::SmallVec;
use std::ops::Deref;

/// Collection of arguments of [`Command`](crate::resp::Command).
/// 
/// This enum is meant to hold a collection of arguments 
/// without systematically allocate a container
#[derive(Debug, Clone)]
pub enum CommandArgs {
    Empty,
    Single(CommandArg),
    Array2([CommandArg; 2]),
    Array3([CommandArg; 3]),
    Array4([CommandArg; 4]),
    Array5([CommandArg; 5]),
    Vec(SmallVec<[CommandArg; 10]>),
}

impl CommandArgs {
    /// Builder function to add an argument to an existing command collection.
    #[must_use]
    #[inline]
    pub fn arg<A>(self, args: A) -> Self
    where
        A: IntoArgs,
    {
        args.into_args(self)
    }

    /// Builder function to add an argument to an existing command collection, 
    /// only if a condition is `true`.
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

    /// Number of arguments of the collection
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

    /// Check if the collection is empty
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
    type Item = &'a CommandArg;
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

/// [`CommandArgs`](CommandArgs) iterator
#[derive(Clone)]
pub enum CommandArgsIterator<'a> {
    Empty,
    Single(Option<&'a CommandArg>),
    Iter(std::slice::Iter<'a, CommandArg>),
}

impl<'a> Iterator for CommandArgsIterator<'a> {
    type Item = &'a CommandArg;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CommandArgsIterator::Empty => None,
            CommandArgsIterator::Single(s) => s.take(),
            CommandArgsIterator::Iter(i) => i.next(),
        }
    }
}

impl IntoIterator for CommandArgs {
    type Item = CommandArg;
    type IntoIter = CommandArgsIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CommandArgs::Empty => CommandArgsIntoIter::Empty,
            CommandArgs::Single(s) => CommandArgsIntoIter::Single(Some(s)),
            CommandArgs::Array2(a) => CommandArgsIntoIter::Array2(a.into_iter()),
            CommandArgs::Array3(a) => CommandArgsIntoIter::Array3(a.into_iter()),
            CommandArgs::Array4(a) => CommandArgsIntoIter::Array4(a.into_iter()),
            CommandArgs::Array5(a) => CommandArgsIntoIter::Array5(a.into_iter()),
            CommandArgs::Vec(a) => CommandArgsIntoIter::Vec(a.into_iter()),
        }
    }
}

/// [`CommandArgs`](CommandArgs) iterator
#[derive(Clone)]
pub enum CommandArgsIntoIter {
    Empty,
    Single(Option<CommandArg>),
    Array2(std::array::IntoIter<CommandArg, 2>),
    Array3(std::array::IntoIter<CommandArg, 3>),
    Array4(std::array::IntoIter<CommandArg, 4>),
    Array5(std::array::IntoIter<CommandArg, 5>),
    Vec(smallvec::IntoIter<[CommandArg; 10]>)
}

impl Iterator for CommandArgsIntoIter {
    type Item = CommandArg;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CommandArgsIntoIter::Empty => None,
            CommandArgsIntoIter::Single(s) => s.take(),
            CommandArgsIntoIter::Array2(i) => i.next(),
            CommandArgsIntoIter::Array3(i) => i.next(),
            CommandArgsIntoIter::Array4(i) => i.next(),
            CommandArgsIntoIter::Array5(i) => i.next(),
            CommandArgsIntoIter::Vec(i) => i.next(),
        }
    }
}

impl Deref for CommandArgs {
    type Target = [CommandArg];

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
