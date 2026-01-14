use crate::{
    client::{PreparedCommand, prepare_command},
    resp::cmd,
};
use serde::Serialize;

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
    fn bitcount(self, key: impl Serialize, range: BitRange) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("BITCOUNT").key(key).arg(range))
    }

    /// The command treats a Redis string as an array of bits,
    /// and is capable of addressing specific integer fields
    /// of varying bit widths and arbitrary non (necessary) aligned offset.
    ///
    /// # Arguments
    /// * `sub_commands` - A collection of [`BitFieldSubCommand`](BitFieldSubCommand)
    ///
    /// # Return
    /// A collection with each entry being the corresponding result of the sub command
    /// given at the same position. OVERFLOW subcommands don't count as generating a reply.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitfield/>](https://redis.io/commands/bitfield/)
    #[must_use]
    fn bitfield<'b>(
        self,
        key: impl Serialize,
        sub_commands: impl IntoIterator<Item = BitFieldSubCommand<'b>> + Serialize,
    ) -> PreparedCommand<'a, Self, Vec<u64>> {
        prepare_command(self, cmd("BITFIELD").key(key).arg(sub_commands))
    }

    /// Read-only variant of the BITFIELD command.
    /// It is like the original BITFIELD but only accepts GET subcommand
    /// and can safely be used in read-only replicas.
    ///
    /// # Arguments
    /// * `sub_commands` - A single or collection of [`BitFieldSubCommand`](BitFieldSubCommand)
    ///
    /// # Return
    /// A collection with each entry being the corresponding result of the sub command
    /// given at the same position.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bitfield_ro/>](https://redis.io/commands/bitfield_ro/)
    #[must_use]
    fn bitfield_readonly<'b>(
        self,
        key: impl Serialize,
        sub_commands: impl IntoIterator<Item = BitFieldSubCommand<'b>> + Serialize,
    ) -> PreparedCommand<'a, Self, Vec<u64>> {
        prepare_command(self, cmd("BITFIELD_RO").key(key).arg(sub_commands))
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
        dest_key: impl Serialize,
        keys: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("BITOP").arg(operation).key(dest_key).key(keys))
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
    fn bitpos(
        self,
        key: impl Serialize,
        bit: u64,
        range: BitRange,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("BITPOS").key(key).arg(bit).arg(range))
    }

    /// Returns the bit value at offset in the string value stored at key.
    ///
    /// # Return
    /// The bit value stored at offset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/getbit/>](https://redis.io/commands/getbit/)
    #[must_use]
    fn getbit(self, key: impl Serialize, offset: u64) -> PreparedCommand<'a, Self, u64> {
        prepare_command(self, cmd("GETBIT").key(key).arg(offset))
    }

    /// Sets or clears the bit at offset in the string value stored at key.
    ///
    /// # Return
    /// The original bit value stored at offset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/setbit/>](https://redis.io/commands/setbit/)
    #[must_use]
    fn setbit(
        self,
        key: impl Serialize,
        offset: u64,
        value: u64,
    ) -> PreparedCommand<'a, Self, u64> {
        prepare_command(self, cmd("SETBIT").key(key).arg(offset).arg(value))
    }
}

/// Interval options for the [`bitcount`](BitmapCommands::bitcount) command
#[derive(Default, Serialize)]
pub struct BitRange(
    #[serde(skip_serializing_if = "Option::is_none")] Option<(isize, isize)>,
    #[serde(skip_serializing_if = "Option::is_none")] Option<BitUnit>,
);

impl BitRange {
    #[must_use]
    pub fn range(start: isize, end: isize) -> Self {
        Self(Some((start, end)), None)
    }

    /// Unit of the range, bit or byte
    #[must_use]
    pub fn unit(mut self, unit: BitUnit) -> Self {
        self.1 = Some(unit);
        self
    }
}

/// Unit of a [`range`](BitRange), bit or byte
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BitUnit {
    Byte,
    Bit,
}

/// Sub-command for the [`bitfield`](BitmapCommands::bitfield) command
#[derive(Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub enum BitFieldSubCommand<'a> {
    Get((&'a str, &'a str)),
    Set((&'a str, &'a str, u64)),
    IncrBy((&'a str, &'a str, i64)),
    Overflow(BitFieldOverflow),
}

impl<'a> BitFieldSubCommand<'a> {
    /// Returns the specified bit field.
    #[must_use]
    pub fn get(encoding: &'a str, offset: &'a str) -> Self {
        Self::Get((encoding, offset))
    }

    /// Set the specified bit field and returns its old value.
    #[must_use]
    pub fn set(encoding: &'a str, offset: &'a str, value: u64) -> Self {
        Self::Set((encoding, offset, value))
    }

    ///  Increments or decrements (if a negative increment is given)
    /// the specified bit field and returns the new value.
    #[must_use]
    pub fn incr_by(encoding: &'a str, offset: &'a str, increment: i64) -> Self {
        Self::IncrBy((encoding, offset, increment))
    }

    #[must_use]
    pub fn overflow(overflow: BitFieldOverflow) -> Self {
        Self::Overflow(overflow)
    }
}

/// Option for the [`BitFieldSubCommand`](BitFieldSubCommand) sub-command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BitFieldOverflow {
    Wrap,
    Sat,
    Fail,
}

/// Bit operation for the [`bitop`](BitmapCommands::bitop) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BitOperation {
    And,
    Or,
    Xor,
    Not,
}
