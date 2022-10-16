use crate::{
    resp::{cmd, ArgsOrCollection, BulkString, CommandArgs, IntoArgs, SingleArgOrCollection},
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to [`Bitmaps`](https://redis.io/docs/data-types/bitmaps/) 
/// & [`Bitfields`](https://redis.io/docs/data-types/bitfields/)
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=bitmap)
pub trait BitmapCommands<T>: PrepareCommand<T> {
    /// Count the number of set bits (population counting) in a string.
    ///
    /// # Return
    /// The number of bits set to 1.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitcount/>](https://redis.io/commands/bitcount/)
    #[must_use]
    fn bitcount<K>(&self, key: K, range: BitRange) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("BITCOUNT").arg(key).arg(range))
    }

    /// The command treats a Redis string as an array of bits,
    /// and is capable of addressing specific integer fields
    /// of varying bit widths and arbitrary non (necessary) aligned offset.
    ///
    /// # Return
    /// A collection with each entry being the corresponding result of the sub command
    /// given at the same position. OVERFLOW subcommands don't count as generating a reply.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitfield/>](https://redis.io/commands/bitfield/)
    #[must_use]
    fn bitfield<K, C, E, O>(&self, key: K, sub_commands: C) -> CommandResult<T, Vec<u64>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        O: Into<BulkString>,
        C: ArgsOrCollection<BitFieldSubCommand<E, O>>,
    {
        self.prepare_command(cmd("BITFIELD").arg(key).arg(sub_commands))
    }

    /// Read-only variant of the BITFIELD command.
    /// It is like the original BITFIELD but only accepts GET subcommand
    /// and can safely be used in read-only replicas.
    ///
    /// # Return
    /// A collection with each entry being the corresponding result of the sub command
    /// given at the same position.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitfield_ro/>](https://redis.io/commands/bitfield_ro/)
    #[must_use]
    fn bitfield_readonly<K, C, E, O>(&self, key: K, get_commands: C) -> CommandResult<T, Vec<u64>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        O: Into<BulkString>,
        C: ArgsOrCollection<BitFieldGetSubCommand<E, O>>,
    {
        self.prepare_command(cmd("BITFIELD_RO").arg(key).arg(get_commands))
    }

    /// Perform a bitwise operation between multiple keys (containing string values)
    /// and store the result in the destination key.
    ///
    /// # Return
    /// The size of the string stored in the destination key,
    /// that is equal to the size of the longest input string.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitop/>](https://redis.io/commands/bitop/)
    #[must_use]
    fn bitop<D, K, KK>(
        &self,
        operation: BitOperation,
        dest_key: D,
        keys: KK,
    ) -> CommandResult<T, usize>
    where
        D: Into<BulkString>,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
    {
        self.prepare_command(cmd("BITOP").arg(operation).arg(dest_key).arg(keys))
    }

    /// Perform a bitwise operation between multiple keys (containing string values)
    /// and store the result in the destination key.
    ///
    /// # Return
    /// The position of the first bit set to 1 or 0 according to the request.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitpos/>](https://redis.io/commands/bitpos/)
    #[must_use]
    fn bitpos<K>(&self, key: K, bit: u64, range: BitRange) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("BITPOS").arg(key).arg(bit).arg(range))
    }

    /// Returns the bit value at offset in the string value stored at key.
    ///
    /// # Return
    /// The bit value stored at offset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/getbit/>](https://redis.io/commands/getbit/)
    #[must_use]
    fn getbit<K>(&self, key: K, offset: u64) -> CommandResult<T, u64>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("GETBIT").arg(key).arg(offset))
    }

    /// Sets or clears the bit at offset in the string value stored at key.
    ///
    /// # Return
    /// The original bit value stored at offset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/setbit/>](https://redis.io/commands/setbit/)
    #[must_use]
    fn setbit<K>(&self, key: K, offset: u64, value: u64) -> CommandResult<T, u64>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("SETBIT").arg(key).arg(offset).arg(value))
    }
}

/// Interval options for the [bitcount](crate::BitmapCommands::bitcount) command
#[derive(Default)]
pub struct BitRange {
    command_args: CommandArgs,
}

impl BitRange {
    #[must_use]
    pub fn range(start: isize, end: isize) -> Self {
        Self {
            command_args: CommandArgs::default().arg(start).arg(end),
        }
    }

    #[must_use]
    pub fn unit(self, unit: BitUnit) -> Self {
        Self {
            command_args: self.command_args.arg(unit),
        }
    }
}

impl IntoArgs for BitRange {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

pub enum BitUnit {
    Byte,
    Bit,
}

impl IntoArgs for BitUnit {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            BitUnit::Byte => "BYTE",
            BitUnit::Bit => "BIT",
        })
    }
}

/// Sub-command for the [bitfield](crate::BitmapCommands::bitfield) command
pub enum BitFieldSubCommand<E = &'static str, O = &'static str>
where
    E: Into<BulkString>,
    O: Into<BulkString>,
{
    Get(BitFieldGetSubCommand<E, O>),
    Set(E, O, u64),
    IncrBy(E, O, i64),
    Overflow(BitFieldOverflow),
}

impl<E, O> BitFieldSubCommand<E, O>
where
    E: Into<BulkString>,
    O: Into<BulkString>,
{
    #[must_use]
    pub fn get(encoding: E, offset: O) -> Self {
        Self::Get(BitFieldGetSubCommand::new(encoding, offset))
    }

    #[must_use]
    pub fn set(encoding: E, offset: O, value: u64) -> Self {
        Self::Set(encoding, offset, value)
    }

    #[must_use]
    pub fn incr_by(encoding: E, offset: O, increment: i64) -> Self {
        Self::IncrBy(encoding, offset, increment)
    }

    #[must_use]
    pub fn overflow(overflow: BitFieldOverflow) -> Self {
        Self::Overflow(overflow)
    }
}

impl<E, O> IntoArgs for BitFieldSubCommand<E, O>
where
    E: Into<BulkString>,
    O: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            BitFieldSubCommand::Get(g) => g.into_args(args),
            BitFieldSubCommand::Set(encoding, offset, value) => {
                args.arg("SET").arg(encoding).arg(offset).arg(value)
            }
            BitFieldSubCommand::IncrBy(encoding, offset, increment) => {
                args.arg("INCRBY").arg(encoding).arg(offset).arg(increment)
            }
            BitFieldSubCommand::Overflow(overflow) => args.arg("OVERFLOW").arg(overflow),
        }
    }
}

pub struct BitFieldGetSubCommand<E = &'static str, O = &'static str>
where
    E: Into<BulkString>,
    O: Into<BulkString>,
{
    encoding: E,
    offset: O,
}

impl<E, O> BitFieldGetSubCommand<E, O>
where
    E: Into<BulkString>,
    O: Into<BulkString>,
{
    #[must_use]
    pub fn new(encoding: E, offset: O) -> Self {
        Self { encoding, offset }
    }
}

impl<E, O> IntoArgs for BitFieldGetSubCommand<E, O>
where
    E: Into<BulkString>,
    O: Into<BulkString>,
{
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg("GET").arg(self.encoding).arg(self.offset)
    }
}

pub enum BitFieldOverflow {
    Wrap,
    Sat,
    Fail,
}

impl IntoArgs for BitFieldOverflow {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            BitFieldOverflow::Wrap => "WRAP",
            BitFieldOverflow::Sat => "SAT",
            BitFieldOverflow::Fail => "FAIL",
        })
    }
}

/// Bit operation for the [bitop](crate::BitmapCommands::bitop) command.
pub enum BitOperation {
    And,
    Or,
    Xor,
    Not,
}

impl IntoArgs for BitOperation {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            BitOperation::And => "AND",
            BitOperation::Or => "OR",
            BitOperation::Xor => "XOR",
            BitOperation::Not => "NOT",
        })
    }
}
