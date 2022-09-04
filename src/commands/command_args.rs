use std::iter::once;

use crate::resp::BulkString;

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
}

impl CommandArgs {
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
}

impl<T> From<T> for CommandArgs
where
    T: Into<BulkString>,
{
    fn from(arg: T) -> Self {
        CommandArgs::Single(arg.into())
    }
}

impl From<[BulkString; 2]> for CommandArgs {
    fn from(args: [BulkString; 2]) -> Self {
        CommandArgs::Array2(args)
    }
}

impl From<[BulkString; 3]> for CommandArgs {
    fn from(args: [BulkString; 3]) -> Self {
        CommandArgs::Array3(args)
    }
}

impl From<[BulkString; 4]> for CommandArgs {
    fn from(args: [BulkString; 4]) -> Self {
        CommandArgs::Array4(args)
    }
}

impl From<Vec<BulkString>> for CommandArgs {
    fn from(args: Vec<BulkString>) -> Self {
        CommandArgs::Vec(args)
    }
}

/// Types compatible with command args
pub trait IntoArgs {
    fn into_args(self, args: CommandArgs) -> CommandArgs;
    fn num_args(&self) -> usize;
}

impl<'a, T> IntoArgs for T
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

impl<'a, T> IntoArgs for [T; 0]
where
    T: Into<BulkString> + Clone,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args
    }

    fn num_args(&self) -> usize {
        0
    }
}

impl<T> IntoArgs for [T; 2]
where
    T: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut it = self.into_iter();

        match args {
            CommandArgs::Empty => {
                CommandArgs::Array2([it.next().unwrap().into(), it.next().unwrap().into()])
            }
            CommandArgs::Single(a) => {
                CommandArgs::Array3([a, it.next().unwrap().into(), it.next().unwrap().into()])
            }
            CommandArgs::Array2(a) => {
                let mut it_old = a.into_iter();
                CommandArgs::Array4([
                    it_old.next().unwrap().into(),
                    it_old.next().unwrap().into(),
                    it.next().unwrap().into(),
                    it.next().unwrap().into(),
                ])
            }
            CommandArgs::Array3(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Array4(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Vec(mut vec) => {
                vec.reserve(2);
                for arg in it {
                    vec.push(arg.into());
                }
                CommandArgs::Vec(vec)
            }
        }
    }

    fn num_args(&self) -> usize {
        2
    }
}

impl<T> IntoArgs for [T; 3]
where
    T: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut it = self.into_iter();

        match args {
            CommandArgs::Empty => CommandArgs::Array3([
                it.next().unwrap().into(),
                it.next().unwrap().into(),
                it.next().unwrap().into(),
            ]),
            CommandArgs::Single(a) => CommandArgs::Array4([
                a,
                it.next().unwrap().into(),
                it.next().unwrap().into(),
                it.next().unwrap().into(),
            ]),
            CommandArgs::Array2(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Array3(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Array4(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Vec(mut vec) => {
                vec.reserve(3);
                for arg in it {
                    vec.push(arg.into());
                }
                CommandArgs::Vec(vec)
            }
        }
    }

    fn num_args(&self) -> usize {
        3
    }
}

impl<T> IntoArgs for [T; 4]
where
    T: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut it = self.into_iter();

        match args {
            CommandArgs::Empty => CommandArgs::Array4([
                it.next().unwrap().into(),
                it.next().unwrap().into(),
                it.next().unwrap().into(),
                it.next().unwrap().into(),
            ]),
            CommandArgs::Single(a) => {
                CommandArgs::Vec(once(a).chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Array2(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Array3(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Array4(a) => {
                CommandArgs::Vec(a.into_iter().chain(it.map(|e| e.into())).collect())
            }
            CommandArgs::Vec(mut vec) => {
                vec.reserve(3);
                for arg in it {
                    vec.push(arg.into());
                }
                CommandArgs::Vec(vec)
            }
        }
    }

    fn num_args(&self) -> usize {
        4
    }
}

impl<T> IntoArgs for Vec<T>
where
    T: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match args {
            CommandArgs::Empty => CommandArgs::Vec(self.into_iter().map(|e| e.into()).collect()),
            CommandArgs::Single(a) => {
                CommandArgs::Vec(once(a).chain(self.into_iter().map(|e| e.into())).collect())
            }
            CommandArgs::Array2(a) => CommandArgs::Vec(
                a.into_iter()
                    .chain(self.into_iter().map(|e| e.into()))
                    .collect(),
            ),
            CommandArgs::Array3(a) => CommandArgs::Vec(
                a.into_iter()
                    .chain(self.into_iter().map(|e| e.into()))
                    .collect(),
            ),
            CommandArgs::Array4(a) => CommandArgs::Vec(
                a.into_iter()
                    .chain(self.into_iter().map(|e| e.into()))
                    .collect(),
            ),
            CommandArgs::Vec(mut vec) => {
                vec.reserve(self.len());
                for arg in self.into_iter() {
                    vec.push(arg.into());
                }
                CommandArgs::Vec(vec)
            }
        }
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}
