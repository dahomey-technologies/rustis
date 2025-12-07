use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, CommandArgs, cmd},
};

/// A group of Redis commands related to [`Bitmaps`](https://redis.io/docs/data-types/bitmaps/)
/// & [`Bitfields`](https://redis.io/docs/data-types/bitfields/)
///
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=bitmap)
pub trait BitmapCommands<'a>: Sized {
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
    fn bitcount(self, key: impl Args, range: BitRange) -> PreparedCommand<'a, Self, usize> {
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
    fn bitfield(
        self,
        key: impl Args,
        sub_commands: impl Args,
    ) -> PreparedCommand<'a, Self, Vec<u64>> {
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
    fn bitfield_readonly(
        self,
        key: impl Args,
        get_commands: impl Args,
    ) -> PreparedCommand<'a, Self, Vec<u64>> {
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
    fn bitop(
        self,
        operation: BitOperation,
        dest_key: impl Args,
        keys: impl Args,
    ) -> PreparedCommand<'a, Self, usize> {
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
    fn bitpos(self, key: impl Args, bit: u64, range: BitRange) -> PreparedCommand<'a, Self, usize> {
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
    fn getbit(self, key: impl Args, offset: u64) -> PreparedCommand<'a, Self, u64> {
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
    fn setbit(self, key: impl Args, offset: u64, value: u64) -> PreparedCommand<'a, Self, u64> {
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

impl Args for BitRange {
    fn write_args(&self, args: &mut CommandArgs) {
        self.command_args.write_args(args);
    }
}

/// Unit of a [`range`](BitRange), bit or byte
pub enum BitUnit {
    Byte,
    Bit,
}

impl Args for BitUnit {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            BitUnit::Byte => "BYTE",
            BitUnit::Bit => "BIT",
        });
    }
}

/// Sub-command for the [`bitfield`](BitmapCommands::bitfield) command
pub struct BitFieldSubCommand {
    args: CommandArgs,
}

impl BitFieldSubCommand {
    /// Returns the specified bit field.
    #[must_use]
    pub fn get(encoding: impl Args, offset: impl Args) -> Self {
        let mut args = CommandArgs::default();
        args.arg("GET").arg(encoding).arg(offset);
        Self { args }
    }

    /// Set the specified bit field and returns its old value.
    #[must_use]
    pub fn set(encoding: impl Args, offset: impl Args, value: u64) -> Self {
        let mut args = CommandArgs::default();
        args.arg("SET").arg(encoding).arg(offset).arg(value);
        Self { args }
    }

    ///  Increments or decrements (if a negative increment is given)
    /// the specified bit field and returns the new value.
    #[must_use]
    pub fn incr_by(encoding: impl Args, offset: impl Args, increment: i64) -> Self {
        let mut args = CommandArgs::default();
        args.arg("INCRBY").arg(encoding).arg(offset).arg(increment);
        Self { args }
    }

    #[must_use]
    pub fn overflow(overflow: BitFieldOverflow) -> Self {
        let mut args = CommandArgs::default();
        args.arg("OVERFLOW").arg(overflow);
        Self { args }
    }
}

impl Args for BitFieldSubCommand {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.args);
    }
}

/// Option for the [`BitFieldSubCommand`](BitFieldSubCommand) sub-command.
pub enum BitFieldOverflow {
    Wrap,
    Sat,
    Fail,
}

impl Args for BitFieldOverflow {
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

impl Args for BitOperation {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            BitOperation::And => "AND",
            BitOperation::Or => "OR",
            BitOperation::Xor => "XOR",
            BitOperation::Not => "NOT",
        });
    }
}
