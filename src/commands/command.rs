use crate::{resp::BulkString, CommandArgs};
use std::iter::once;

pub fn cmd(name: &'static str) -> Command {
    Command::new(name)
}

#[derive(Debug)]
pub struct Command {
    pub name: &'static str,
    pub args: CommandArgs,
}

impl Command {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            args: CommandArgs::Empty,
        }
    }

    pub fn arg<A>(self, args: A) -> Self
    where
        A: IntoArgs,
    {
        args.into_args(self)
    }
}

/// Types compatible with command args
pub trait IntoArgs {
    fn into_args(self, command: Command) -> Command;
    fn num_args(&self) -> usize;
}

impl<'a, T> IntoArgs for T
where
    T: Into<BulkString>,
{
    fn into_args(self, command: Command) -> Command {
        let args = match command.args {
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
        };

        Command {
            name: command.name,
            args: args,
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
    fn into_args(self, command: Command) -> Command {
        command
    }

    fn num_args(&self) -> usize {
        0
    }
}

impl<T> IntoArgs for [T; 2]
where
    T: Into<BulkString>,
{
    fn into_args(self, command: Command) -> Command {
        let mut it = self.into_iter();

        let args = match command.args {
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
        };

        Command {
            name: command.name,
            args: args,
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
    fn into_args(self, command: Command) -> Command {
        let mut it = self.into_iter();

        let args = match command.args {
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
        };

        Command {
            name: command.name,
            args: args,
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
    fn into_args(self, command: Command) -> Command {
        let mut it = self.into_iter();

        let args = match command.args {
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
        };

        Command {
            name: command.name,
            args: args,
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
    fn into_args(self, command: Command) -> Command {
        let args = match command.args {
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
        };

        Command {
            name: command.name,
            args,
        }
    }

    fn num_args(&self) -> usize {
        self.len()
    }
}
