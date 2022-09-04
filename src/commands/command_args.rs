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

/// Types compatible with command args
pub trait IntoArgs {
    fn into_args(self, args: CommandArgs) -> CommandArgs;
    fn num_args(&self) -> usize;
}

/// Marker for collections of IntoArgs
pub trait IntoArgsCollection<T>: IntoArgs
where
    T: IntoArgs,
{
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

impl<T, const N: usize> IntoArgs for [T; N]
where
    T: IntoArgs,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        let mut args = args;
        for a in self.into_iter() {
            args = a.into_args(args)
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
        for a in self.into_iter() {
            args = a.into_args(args)
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

impl<T, const N: usize> IntoArgsCollection<T> for [T; N] where T: IntoArgs {}
impl<T> IntoArgsCollection<T> for Vec<T> where T: IntoArgs {}
impl<T> IntoArgsCollection<T> for T where T: IntoArgs {}
