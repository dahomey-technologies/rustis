use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{cmd, CommandArgs, MultipleArgsCollection, SingleArg, SingleArgCollection, ToArgs},
};

/// A group of Redis commands related to [`Bitmaps`](https://redis.io/docs/data-types/bitmaps/)
/// & [`Bitfields`](https://redis.io/docs/data-types/bitfields/)
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=bitmap)
pub trait BitmapCommands<'a> {
    /// Count the number of set bits (population counting) in a string.
    ///
    /// # Return
    /// The number of bits set to 1.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::{BitRange, BitUnit},
    /// #    Result,
    /// # };
    /// # #[cfg_attr(feature = "tokio-runtime", tokio::test)]
    /// # #[cfg_attr(feature = "async-std-runtime", async_std::test)]
    /// # async fn bitcount() -> Result<()> {
    /// #    let client = get_test_client().await?;
    /// client.set("mykey", "foobar").await?;
    ///
    /// let count = client.bitcount("mykey", BitRange::default()).await?;
    /// assert_eq!(26, count);
    ///
    /// let count = client.bitcount("mykey", BitRange::range(0, 0)).await?;
    /// assert_eq!(4, count);
    ///
    /// let count = client.bitcount("mykey", BitRange::range(1, 1)).await?;
    /// assert_eq!(6, count);
    ///
    /// let count = client
    ///     .bitcount("mykey", BitRange::range(1, 1).unit(BitUnit::Byte))
    ///     .await?;
    /// assert_eq!(6, count);
    ///
    /// let count = client
    ///     .bitcount("mykey", BitRange::range(5, 30).unit(BitUnit::Bit))
    ///     .await?;
    /// assert_eq!(17, count);
    /// #    client.close().await?;
    /// #    Ok(())
    /// # }
    ///```
    /// # See Also
    /// [<https://redis.io/commands/bitcount/>](https://redis.io/commands/bitcount/)
    #[must_use]
    fn bitcount<K>(self, key: K, range: BitRange) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("BITCOUNT").arg(key).arg(range))
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
    fn bitfield<K, C, E, O>(self, key: K, sub_commands: C) -> PreparedCommand<'a, Self, Vec<u64>>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        O: SingleArg,
        C: MultipleArgsCollection<BitFieldSubCommand<E, O>>,
    {
        prepare_command(self, cmd("BITFIELD").arg(key).arg(sub_commands))
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
    fn bitfield_readonly<K, C, E, O>(
        self,
        key: K,
        get_commands: C,
    ) -> PreparedCommand<'a, Self, Vec<u64>>
    where
        Self: Sized,
        K: SingleArg,
        E: SingleArg,
        O: SingleArg,
        C: MultipleArgsCollection<BitFieldGetSubCommand<E, O>>,
    {
        prepare_command(self, cmd("BITFIELD_RO").arg(key).arg(get_commands))
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
        self,
        operation: BitOperation,
        dest_key: D,
        keys: KK,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        K: SingleArg,
        KK: SingleArgCollection<K>,
    {
        prepare_command(self, cmd("BITOP").arg(operation).arg(dest_key).arg(keys))
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
    fn bitpos<K>(self, key: K, bit: u64, range: BitRange) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("BITPOS").arg(key).arg(bit).arg(range))
    }

    /// Returns the bit value at offset in the string value stored at key.
    ///
    /// # Return
    /// The bit value stored at offset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/getbit/>](https://redis.io/commands/getbit/)
    #[must_use]
    fn getbit<K>(self, key: K, offset: u64) -> PreparedCommand<'a, Self, u64>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("GETBIT").arg(key).arg(offset))
    }

    /// Sets or clears the bit at offset in the string value stored at key.
    ///
    /// # Return
    /// The original bit value stored at offset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/setbit/>](https://redis.io/commands/setbit/)
    #[must_use]
    fn setbit<K>(self, key: K, offset: u64, value: u64) -> PreparedCommand<'a, Self, u64>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("SETBIT").arg(key).arg(offset).arg(value))
    }
}

/// Interval options for the [`bitcount`](BitmapCommands::bitcount) command
#[derive(Default)]
pub struct BitRange {
    command_args: CommandArgs,
}

impl BitRange {
    #[must_use]
    pub fn range(start: isize, end: isize) -> Self {
        Self {
            command_args: CommandArgs::default().arg(start).arg(end).build(),
        }
    }

    /// Unit of the range, bit or byte
    #[must_use]
    pub fn unit(mut self, unit: BitUnit) -> Self {
        Self {
            command_args: self.command_args.arg(unit).build(),
        }
    }
}

impl ToArgs for BitRange {
    fn write_args(&self, args: &mut CommandArgs) {
        self.command_args.write_args(args);
    }
}

/// Unit of a [`range`](BitRange), bit or byte
pub enum BitUnit {
    Byte,
    Bit,
}

impl ToArgs for BitUnit {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            BitUnit::Byte => "BYTE",
            BitUnit::Bit => "BIT",
        });
    }
}

/// Sub-command for the [`bitfield`](BitmapCommands::bitfield) command
pub enum BitFieldSubCommand<E = &'static str, O = &'static str>
where
    E: SingleArg,
    O: SingleArg,
{
    Get(BitFieldGetSubCommand<E, O>),
    Set(E, O, u64),
    IncrBy(E, O, i64),
    Overflow(BitFieldOverflow),
}

impl<E, O> BitFieldSubCommand<E, O>
where
    E: SingleArg,
    O: SingleArg,
{
    /// Returns the specified bit field.
    #[must_use]
    pub fn get(encoding: E, offset: O) -> Self {
        Self::Get(BitFieldGetSubCommand::new(encoding, offset))
    }

    /// Set the specified bit field and returns its old value.
    #[must_use]
    pub fn set(encoding: E, offset: O, value: u64) -> Self {
        Self::Set(encoding, offset, value)
    }

    ///  Increments or decrements (if a negative increment is given)
    /// the specified bit field and returns the new value.
    #[must_use]
    pub fn incr_by(encoding: E, offset: O, increment: i64) -> Self {
        Self::IncrBy(encoding, offset, increment)
    }

    #[must_use]
    pub fn overflow(overflow: BitFieldOverflow) -> Self {
        Self::Overflow(overflow)
    }
}

impl<E, O> ToArgs for BitFieldSubCommand<E, O>
where
    E: SingleArg,
    O: SingleArg,
{
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            BitFieldSubCommand::Get(g) => args.arg_ref(g),
            BitFieldSubCommand::Set(encoding, offset, value) => args
                .arg("SET")
                .arg_ref(encoding)
                .arg_ref(offset)
                .arg(*value),
            BitFieldSubCommand::IncrBy(encoding, offset, increment) => args
                .arg("INCRBY")
                .arg_ref(encoding)
                .arg_ref(offset)
                .arg(*increment),
            BitFieldSubCommand::Overflow(overflow) => args.arg("OVERFLOW").arg_ref(overflow),
        };
    }
}

/// Sub-command for the [`bitfield`](BitmapCommands::bitfield) command
pub struct BitFieldGetSubCommand<E = &'static str, O = &'static str>
where
    E: SingleArg,
    O: SingleArg,
{
    encoding: E,
    offset: O,
}

impl<E, O> BitFieldGetSubCommand<E, O>
where
    E: SingleArg,
    O: SingleArg,
{
    #[must_use]
    pub fn new(encoding: E, offset: O) -> Self {
        Self { encoding, offset }
    }
}

impl<E, O> ToArgs for BitFieldGetSubCommand<E, O>
where
    E: SingleArg,
    O: SingleArg,
{
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg("GET")
            .arg_ref(&self.encoding)
            .arg_ref(&self.offset);
    }
}

/// Option for the [`BitFieldSubCommand`](BitFieldSubCommand) sub-command.
pub enum BitFieldOverflow {
    Wrap,
    Sat,
    Fail,
}

impl ToArgs for BitFieldOverflow {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            BitFieldOverflow::Wrap => "WRAP",
            BitFieldOverflow::Sat => "SAT",
            BitFieldOverflow::Fail => "FAIL",
        });
    }
}

/// Bit operation for the [`bitop`](BitmapCommands::bitop) command.
pub enum BitOperation {
    And,
    Or,
    Xor,
    Not,
}

impl ToArgs for BitOperation {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            BitOperation::And => "AND",
            BitOperation::Or => "OR",
            BitOperation::Xor => "XOR",
            BitOperation::Not => "NOT",
        });
    }
}
