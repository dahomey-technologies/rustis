use smallvec::SmallVec;

use crate::resp::ToArgs;
use std::fmt;

/// Collection of arguments of [`Command`](crate::resp::Command).
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct CommandArgs {
    args: SmallVec<[Vec<u8>; 10]>,
}

impl CommandArgs {
    /// Builder function to add an argument to an existing command collection.
    #[inline]
    pub fn arg<A>(&mut self, args: A) -> &mut Self
    where
        A: ToArgs,
    {
        args.write_args(self);
        self
    }

    /// Builder function to add an argument by ref to an existing command collection.
    #[inline]
    pub fn arg_ref<A>(&mut self, args: &A) -> &mut Self
    where
        A: ToArgs,
    {
        args.write_args(self);
        self
    }

    /// Builder function to add an argument to an existing command collection,
    /// only if a condition is `true`.
    #[inline]
    pub fn arg_if<A>(&mut self, condition: bool, args: A) -> &mut Self
    where
        A: ToArgs,
    {
        if condition { self.arg(args) } else { self }
    }

    /// helper to build a CommandArgs in one line.
    #[inline]
    pub fn build(&mut self) -> Self {
        let mut args = CommandArgs::default();
        std::mem::swap(&mut args.args, &mut self.args);
        args
    }

    /// Number of arguments of the collection
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.args.len()
    }

    /// Check if the collection is empty
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub(crate) fn write_arg(&mut self, buf: Vec<u8>) {
        self.args.push(buf);
    }

    pub(crate) fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&[u8]) -> bool,
    {
        self.args.retain(|arg| f(arg))
    }
}

impl<'a> IntoIterator for &'a CommandArgs {
    type Item = &'a [u8];
    type IntoIter = CommandArgsIterator<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        CommandArgsIterator {
            iter: self.args.iter(),
        }
    }
}

/// [`CommandArgs`] iterator
pub struct CommandArgsIterator<'a> {
    iter: std::slice::Iter<'a, Vec<u8>>,
}

impl<'a> Iterator for CommandArgsIterator<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|v| v.as_slice())
    }
}

impl IntoIterator for CommandArgs {
    type Item = Vec<u8>;
    type IntoIter = CommandArgsIntoIterator;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        CommandArgsIntoIterator {
            iter: self.args.into_iter(),
        }
    }
}

/// [`CommandArgs`] into iterator
pub struct CommandArgsIntoIterator {
    iter: <SmallVec<[Vec<u8>; 10]> as IntoIterator>::IntoIter,
}

impl Iterator for CommandArgsIntoIterator {
    type Item = Vec<u8>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl std::ops::Deref for CommandArgs {
    type Target = [Vec<u8>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl fmt::Debug for CommandArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandArgs")
            .field(
                "args",
                &self
                    .args
                    .iter()
                    .map(|a| String::from_utf8_lossy(a.as_slice()))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}
