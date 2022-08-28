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
