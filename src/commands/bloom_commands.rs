use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, deserialize_byte_buf, CommandArgs, FromValueArray, IntoArgs, SingleArg,
        SingleArgCollection,
    },
};
use serde::Deserialize;

/// A group of Redis commands related to [`Bloom filters`](https://redis.io/docs/stack/bloom/)
///
/// # See Also
/// [Bloom Filter Commands](https://redis.io/commands/?group=bf)
pub trait BloomCommands {
    /// Adds an item to a bloom filter
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to add
    ///
    /// # Return
    /// * `true` - if the item did not exist in the filter,
    /// * `false` - otherwise.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/bf.add/>](https://redis.io/commands/bf.add/)
    #[must_use]
    fn bf_add(&mut self, key: impl SingleArg, item: impl SingleArg) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.ADD").arg(key).arg(item))
    }

    /// Determines whether an item may exist in the Bloom Filter or not.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to check for
    ///
    /// # Return
    /// * `true` - means the item may exist in the filter,
    /// * `false` - means it does not exist in the filter..
    ///
    /// # See Also
    /// * [<https://redis.io/commands/bf.exists/>](https://redis.io/commands/bf.exists/)
    #[must_use]
    fn bf_exists(
        &mut self,
        key: impl SingleArg,
        item: impl SingleArg,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.EXISTS").arg(key).arg(item))
    }

    /// Return information about key filter.
    ///
    /// # Arguments
    /// * `key` - Name of the key to return information about
    ///
    /// # Return
    /// an instance of [`BfInfoResult`](BfInfoResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.info/>](https://redis.io/commands/bf.info/)
    #[must_use]
    fn bf_info_all(&mut self, key: impl SingleArg) -> PreparedCommand<Self, BfInfoResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.INFO").arg(key))
    }

    /// Return information about key filter for a specific information parameter
    ///
    /// # Arguments
    /// * `key` - Name of the key to return information about
    /// * `param` - specific information parameter to query
    ///
    /// # Return
    /// The value of the requested parameter
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.info/>](https://redis.io/commands/bf.info/)
    #[must_use]
    fn bf_info(
        &mut self,
        key: impl SingleArg,
        param: BfInfoParameter,
    ) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.INFO").arg(key).arg(param))
    }

    /// `bf_insert` is a sugarcoated combination of [`bf_reserve`](BloomCommands::bf_reserve) and [`bf_add`](BloomCommands::bf_add).
    ///
    /// It creates a new filter if the key does not exist using the relevant arguments (see [`bf_reserve`](BloomCommands::bf_reserve)).
    /// Next, all ITEMS are inserted.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `items` - One or more items to add
    /// * `options` - See [`BfInsertOptions`](BfInsertOptions)
    ///
    /// # Return
    /// A collection of booleans (integers).
    ///
    /// Each element is either true or false depending on whether the corresponding input element was newly added to the filter or may have previously existed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.insert/>](https://redis.io/commands/bf.insert/)
    #[must_use]
    fn bf_insert<I: SingleArg, R: FromValueArray<bool>>(
        &mut self,
        key: impl SingleArg,
        items: impl SingleArgCollection<I>,
        options: BfInsertOptions,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("BF.INSERT")
                .arg(key)
                .arg(options)
                .arg("ITEMS")
                .arg(items),
        )
    }

    /// Restores a filter previously saved using [`bf_scandump`](BloomCommands::bf_scandump).
    ///
    /// See the [`bf_scandump`](BloomCommands::bf_scandump) command for example usage.
    ///
    /// This command overwrites any bloom filter stored under key.
    /// Make sure that the bloom filter is not be changed between invocations.
    ///
    /// # Arguments
    /// * `key` - Name of the key to restore
    /// * `iterator` - Iterator value associated with `data` (returned by [`bf_scandump`](BloomCommands::bf_scandump))
    /// * `data` - Current data chunk (returned by [`bf_scandump`](BloomCommands::bf_scandump))
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.loadchunk/>](https://redis.io/commands/bf.loadchunk/)
    #[must_use]
    fn bf_loadchunk(
        &mut self,
        key: impl SingleArg,
        iterator: i64,
        data: impl SingleArg,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.LOADCHUNK").arg(key).arg(iterator).arg(data))
    }

    /// Adds one or more items to the Bloom Filter and creates the filter if it does not exist yet.
    ///
    /// This command operates identically to [`bf_add`](BloomCommands::bf_add) except that it allows multiple inputs and returns multiple values.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `items` - One or more items to add
    ///
    /// # Return
    /// Collection reply of boolean - for each item which is either `true` or `false` depending
    /// on whether the corresponding input element was newly added to the filter or may have previously existed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.madd/>](https://redis.io/commands/bf.madd/)
    #[must_use]
    fn bf_madd<I: SingleArg, R: FromValueArray<bool>>(
        &mut self,
        key: impl SingleArg,
        items: impl SingleArgCollection<I>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.MADD").arg(key).arg(items))
    }

    /// Determines if one or more items may exist in the filter or not.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `items` - One or more items to check
    ///
    /// # Return
    /// Collection reply of boolean - for each item where `true` value means the corresponding item
    /// may exist in the filter, and a `false` value means it does not exist in the filter.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.mexists/>](https://redis.io/commands/bf.mexists/)
    #[must_use]
    fn bf_mexists<I: SingleArg, R: FromValueArray<bool>>(
        &mut self,
        key: impl SingleArg,
        items: impl SingleArgCollection<I>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.MEXISTS").arg(key).arg(items))
    }

    /// Creates an empty Bloom Filter with a single sub-filter
    /// for the initial capacity requested and with an upper bound error_rate.
    ///
    /// By default, the filter auto-scales by creating additional sub-filters when capacity is reached.
    /// The new sub-filter is created with size of the previous sub-filter multiplied by expansion.
    ///
    /// Though the filter can scale up by creating sub-filters,
    /// it is recommended to reserve the estimated required capacity since maintaining and querying sub-filters requires additional memory
    /// (each sub-filter uses an extra bits and hash function) and consume further CPU time
    /// than an equivalent filter that had the right capacity at creation time.
    ///
    /// The number of hash functions is `log(error)/ln(2)^2`. The number of bits per item is `log(error)/ln(2)` ≈ 1.44.
    /// * 1% error rate requires 7 hash functions and 10.08 bits per item.
    /// * 0.1% error rate requires 10 hash functions and 14.4 bits per item.
    /// * 0.01% error rate requires 14 hash functions and 20.16 bits per item.
    ///
    /// # Arguments
    /// * `key` - The key under which the filter is found
    /// * `error_rate` - The desired probability for false positives.
    ///  The rate is a decimal value between 0 and 1.
    ///  For example, for a desired false positive rate of 0.1% (1 in 1000),
    ///  error_rate should be set to 0.001.
    /// * `capacity` - The number of entries intended to be added to the filter.
    ///  If your filter allows scaling, performance will begin to degrade after adding more items than this number.
    ///  The actual degradation depends on how far the limit has been exceeded.
    ///  Performance degrades linearly with the number of `sub-filters`.
    /// * `options` - See [`BfReserveOptions`](BfReserveOptions)
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.reserve/>](https://redis.io/commands/bf.reserve/)
    #[must_use]
    fn bf_reserve(
        &mut self,
        key: impl SingleArg,
        error_rate: f64,
        capacity: usize,
        options: BfReserveOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("BF.RESERVE")
                .arg(key)
                .arg(error_rate)
                .arg(capacity)
                .arg(options),
        )
    }

    /// Begins an incremental save of the bloom filter.
    /// This is useful for large bloom filters which cannot fit into the normal [`dump`](crate::commands::GenericCommands::dump)
    /// and [`restore`](crate::commands::GenericCommands::restore) model.
    ///
    /// # Arguments
    /// * `key` - Name of the filter
    /// * `iterator` - Iterator value; either 0 or the iterator from a previous invocation of this command.\
    ///  The first time this command is called, the value of `iterator` should be 0.
    ///
    /// # Return
    /// This command returns successive `(iterator, data)` pairs until `(0, vec![])` to indicate completion.
    ///
    /// # See Also
    /// [<https://redis.io/commands/bf.scandump/>](https://redis.io/commands/bf.scandump/)
    #[must_use]
    fn bf_scandump(
        &mut self,
        key: impl SingleArg,
        iterator: i64,
    ) -> PreparedCommand<Self, BfScanDumpResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("BF.SCANDUMP").arg(key).arg(iterator))
    }
}

/// Optional parameter for the [`bf_info`](BloomCommands::bf_info) command.
///
/// Used to query a specific parameter.
pub enum BfInfoParameter {
    Capacity,
    Size,
    NumFilters,
    NumItemsInserted,
    ExpansionRate,
}

impl IntoArgs for BfInfoParameter {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            BfInfoParameter::Capacity => args.arg("CAPACITY"),
            BfInfoParameter::Size => args.arg("SIZE"),
            BfInfoParameter::NumFilters => args.arg("FILTERS"),
            BfInfoParameter::NumItemsInserted => args.arg("ITEMS"),
            BfInfoParameter::ExpansionRate => args.arg("EXPANSION"),
        }
    }
}

/// Result for the [`bf_info`](BloomCommands::bf_info) command.
#[derive(Debug, Deserialize)]
pub struct BfInfoResult {
    #[serde(rename = "Capacity")]
    pub capacity: usize,
    #[serde(rename = "Size")]
    pub size: usize,
    #[serde(rename = "Number of filters")]
    pub num_filters: usize,
    #[serde(rename = "Number of items inserted")]
    pub num_items_inserted: usize,
    #[serde(rename = "Expansion rate")]
    pub expansion_rate: usize,
}

/// Options for the [`bf_insert`](BloomCommands::bf_insert) command.
#[derive(Default)]
pub struct BfInsertOptions {
    command_args: CommandArgs,
}

impl BfInsertOptions {
    /// Specifies the desired capacity for the filter to be created.
    ///
    /// This parameter is ignored if the filter already exists.
    /// If the filter is automatically created and this parameter is absent,
    /// then the module-level capacity is used.
    /// See [`bf_reserve`](BloomCommands::bf_reserve) for more information about the impact of this value.
    #[must_use]
    pub fn capacity(self, capacity: usize) -> Self {
        Self {
            command_args: self.command_args.arg("CAPACITY").arg(capacity),
        }
    }

    /// Specifies the error ratio of the newly created filter if it does not yet exist.
    ///
    /// If the filter is automatically created and error is not specified then the module-level error rate is used.
    /// See [`bf_reserve`](BloomCommands::bf_reserve) for more information about the format of this value.
    #[must_use]
    pub fn error(self, error_rate: f64) -> Self {
        Self {
            command_args: self.command_args.arg("ERROR").arg(error_rate),
        }
    }

    /// When `capacity` is reached, an additional sub-filter is created.
    /// The size of the new sub-filter is the size of the last sub-filter multiplied by `expansion`.
    /// If the number of elements to be stored in the filter is unknown,
    /// we recommend that you use an `expansion` of 2 or more to reduce the number of sub-filters.
    /// Otherwise, we recommend that you use an `expansion` of 1 to reduce memory consumption.
    /// The default expansion value is 2.
    #[must_use]
    pub fn expansion(self, expansion: usize) -> Self {
        Self {
            command_args: self.command_args.arg("EXPANSION").arg(expansion),
        }
    }

    /// Indicates that the filter should not be created if it does not already exist.
    ///
    /// If the filter does not yet exist, an error is returned rather than creating it automatically.
    /// This may be used where a strict separation between filter creation and filter addition is desired.
    /// It is an error to specify `nocreate` together with either [`capacity`](BfInsertOptions::capacity) or [`error`](BfInsertOptions::error).
    #[must_use]
    pub fn nocreate(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOCREATE"),
        }
    }

    /// Prevents the filter from creating additional sub-filters if initial capacity is reached.
    ///
    /// Non-scaling filters require slightly less memory than their scaling counterparts.
    /// The filter returns an error when `capacity` is reached.
    #[must_use]
    pub fn nonscaling(self) -> Self {
        Self {
            command_args: self.command_args.arg("NONSCALING"),
        }
    }
}

impl IntoArgs for BfInsertOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`bf_reserve`](BloomCommands::bf_reserve) command.
#[derive(Default)]
pub struct BfReserveOptions {
    command_args: CommandArgs,
}

impl BfReserveOptions {
    /// When `capacity` is reached, an additional sub-filter is created.
    /// The size of the new sub-filter is the size of the last sub-filter multiplied by `expansion`.
    /// If the number of elements to be stored in the filter is unknown,
    /// we recommend that you use an `expansion` of 2 or more to reduce the number of sub-filters.
    /// Otherwise, we recommend that you use an `expansion` of 1 to reduce memory consumption.
    /// The default expansion value is 2.
    #[must_use]
    pub fn expansion(self, expansion: usize) -> Self {
        Self {
            command_args: self.command_args.arg("EXPANSION").arg(expansion),
        }
    }

    /// Prevents the filter from creating additional sub-filters if initial capacity is reached.
    ///
    /// Non-scaling filters require slightly less memory than their scaling counterparts.
    /// The filter returns an error when `capacity` is reached.
    #[must_use]
    pub fn nonscaling(self) -> Self {
        Self {
            command_args: self.command_args.arg("NONSCALING"),
        }
    }
}

impl IntoArgs for BfReserveOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`bf_scandump`](BloomCommands::bf_scandump) command.
#[derive(Debug, Deserialize)]
pub struct BfScanDumpResult {
    pub iterator: i64,
    #[serde(deserialize_with = "deserialize_byte_buf")]
    pub data: Vec<u8>,
}
