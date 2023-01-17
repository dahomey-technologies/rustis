use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, deserialize_byte_buf, CommandArgs, FromValueArray, IntoArgs, SingleArg,
        SingleArgCollection, Value,
    },
};
use serde::Deserialize;
use std::collections::HashMap;

/// A group of Redis commands related to [`Cuckoo filters`](https://redis.io/docs/stack/bloom/)
///
/// # See Also
/// [Cuckoo Filter Commands](https://redis.io/commands/?group=cf)
pub trait CuckooCommands {
    /// Adds an item to the cuckoo filter, creating the filter if it does not exist.
    ///
    /// Cuckoo filters can contain the same item multiple times, and consider each insert as separate.
    /// You can use [`cf_addnx`](CuckooCommands::cf_addnx) to only add the item if it does not exist yet.
    /// Keep in mind that deleting an element inserted using [`cf_addnx`](CuckooCommands::cf_addnx) may cause false-negative errors.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to add
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.add/>](https://redis.io/commands/cf.add/)
    #[must_use]
    fn cf_add(&mut self, key: impl SingleArg, item: impl SingleArg) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.ADD").arg(key).arg(item))
    }

    /// Adds an item to a cuckoo filter if the item did not exist previously.
    ///
    /// See documentation on [`cf_add`](CuckooCommands::cf_add) for more information on this command.
    ///
    /// This command is equivalent to a [`cf_exists`](CuckooCommands::cf_exists) + [`cf_add`](CuckooCommands::cf_add) command.
    /// It does not insert an element into the filter if its fingerprint already exists in order to use the available capacity more efficiently.
    /// However, deleting elements can introduce `false negative` error rate!
    ///
    /// Note that this command is slower than [`cf_add`](CuckooCommands::cf_add) because it first checks whether the item exists.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to add
    ///
    /// # Return
    /// * `true` - if the item did not exist in the filter.
    /// * `false` - if the item already existed.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.addnx/>](https://redis.io/commands/cf.addnx/)
    #[must_use]
    fn cf_addnx(&mut self, key: impl SingleArg, item: impl SingleArg) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.ADDNX").arg(key).arg(item))
    }

    /// Returns the number of times an item may be in the filter.
    ///
    /// Because this is a probabilistic data structure, this may not necessarily be accurate.
    ///
    /// If you just want to know if an item exists in the filter,
    /// use [`cf_exists`](CuckooCommands::cf_exists) because it is more efficient for that purpose.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to count
    ///
    /// # Return
    /// the count of possible matching copies of the item in the filter.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.count/>](https://redis.io/commands/cf.count/)
    #[must_use]
    fn cf_count(
        &mut self,
        key: impl SingleArg,
        item: impl SingleArg,
    ) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.COUNT").arg(key).arg(item))
    }

    /// Deletes an item once from the filter.
    ///
    /// If the item exists only once, it will be removed from the filter.
    /// If the item was added multiple times, it will still be present.
    ///
    /// # Danger !
    /// Deleting elements that are not in the filter may delete a different item, resulting in false negatives!
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to delete from the filter
    ///
    /// # Complexity
    /// O(n), where n is the number of `sub-filters`. Both alternative locations are checked on all `sub-filters`.
    ///
    /// # Return
    /// * `true` - the item has been deleted from the filter.
    /// * `false` - if the item was not found.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.del/>](https://redis.io/commands/cf.del/)
    #[must_use]
    fn cf_del(&mut self, key: impl SingleArg, item: impl SingleArg) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.DEL").arg(key).arg(item))
    }

    /// Check if an `item` exists in a Cuckoo Filter `key`
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `item` - The item to check for
    ///
    /// # Return
    /// * `true` - the item may exist in the filter
    /// * `false` - if the item does not exist in the filter.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.exists/>](https://redis.io/commands/cf.exists/)
    #[must_use]
    fn cf_exists(
        &mut self,
        key: impl SingleArg,
        item: impl SingleArg,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.EXISTS").arg(key).arg(item))
    }

    /// Return information about `key`
    ///
    /// # Arguments
    /// * `key` - Name of the key to get info about
    ///
    /// # Return
    /// An instance of [`CfInfoResult`](CfInfoResult)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.info/>](https://redis.io/commands/cf.info/)
    #[must_use]
    fn cf_info(&mut self, key: impl SingleArg) -> PreparedCommand<Self, CfInfoResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.INFO").arg(key))
    }

    /// Adds one or more items to a cuckoo filter, allowing the filter to be created with a custom capacity if it does not exist yet.
    ///
    /// These commands offers more flexibility over the [`cf_add`](CuckooCommands::cf_add) command, at the cost of more verbosity.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `options` - see [`CfInsertOptions`](CfInsertOptions)
    /// * `items` - One or more items to add.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.insert/>](https://redis.io/commands/cf.insert/)
    #[must_use]
    fn cf_insert<I: SingleArg>(
        &mut self,
        key: impl SingleArg,
        options: CfInsertOptions,
        item: impl SingleArgCollection<I>,
    ) -> PreparedCommand<Self, Vec<usize>>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("CF.INSERT")
                .arg(key)
                .arg(options)
                .arg("ITEMS")
                .arg(item),
        )
    }

    /// Adds one or more items to a cuckoo filter, allowing the filter to be created with a custom capacity if it does not exist yet.
    ///
    /// This command is equivalent to a [`cf_exists`](CuckooCommands::cf_exists) + [`cf_add`](CuckooCommands::cf_add) command.
    /// It does not insert an element into the filter if its fingerprint already exists and therefore better utilizes the available capacity.
    /// However, if you delete elements it might introduce `false negative` error rate!
    ///
    /// These commands offers more flexibility over the [`cf_add`](CuckooCommands::cf_add) and [`cf_addnx`](CuckooCommands::cf_addnx) commands,
    /// at the cost of more verbosity.
    ///
    /// # Complexity
    /// `O(n + i)`, where n is the number of `sub-filters` and `i` is `maxIterations`.
    /// Adding items requires up to 2 memory accesses per `sub-filter`.
    /// But as the filter fills up, both locations for an item might be full.
    /// The filter attempts to `Cuckoo` swap items up to maxIterations times.
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `options` - see [`CfInsertOptions`](CfInsertOptions)
    /// * `items` - One or more items to add.
    ///
    /// # Return
    /// A collection of integers corresponding to the items specified. Possible values for each element are:
    /// * `>0` - if the item was successfully inserted
    /// * `0` - if the item already existed and [`cf_insertnx`](CuckooCommands::cf_insertnx) is used.
    /// * `<0` - if an error occurred
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cf.insert/>](https://redis.io/commands/cf.insert/)
    #[must_use]
    fn cf_insertnx<I: SingleArg, R: FromValueArray<i64>>(
        &mut self,
        key: impl SingleArg,
        options: CfInsertOptions,
        item: impl SingleArgCollection<I>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("CF.INSERTNX")
                .arg(key)
                .arg(options)
                .arg("ITEMS")
                .arg(item),
        )
    }

    /// Restores a filter previously saved using [`cf_scandump`](CuckooCommands::cf_scandump).
    ///
    /// See the [`cf_scandump`](CuckooCommands::cf_scandump) command for example usage.
    ///
    /// This command overwrites any bloom filter stored under `key`.
    /// Make sure that the bloom filter is not be changed between invocations.
    ///
    /// # Arguments
    /// * `key` - Name of the key to restore
    /// * `iterator` - Iterator value associated with `data` (returned by [`cf_scandump`](CuckooCommands::cf_scandump))
    /// * `data` - Current data chunk (returned by [`cf_scandump`](CuckooCommands::cf_scandump))
    ///
    /// # See Also
    /// [<https://redis.io/commands/cf.loadchunk/>](https://redis.io/commands/cf.loadchunk/)
    #[must_use]
    fn cf_loadchunk(
        &mut self,
        key: impl SingleArg,
        iterator: i64,
        data: impl SingleArg,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.LOADCHUNK").arg(key).arg(iterator).arg(data))
    }

    /// Check if one or more `items` exists in a Cuckoo Filter `key`
    ///
    /// # Arguments
    /// * `key` - The name of the filter
    /// * `items` - One or more items to check for
    ///
    /// # Return
    /// Collection reply of boolean - for each item where `true` value means the corresponding item
    /// may exist in the filter, and a `false` value means it does not exist in the filter.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cf.mexists/>](https://redis.io/commands/cf.mexists/)
    #[must_use]
    fn cf_mexists<I: SingleArg, R: FromValueArray<bool>>(
        &mut self,
        key: impl SingleArg,
        items: impl SingleArgCollection<I>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.MEXISTS").arg(key).arg(items))
    }

    /// Create a Cuckoo Filter as `key` with a single sub-filter for the initial amount of `capacity` for items.
    /// Because of how Cuckoo Filters work, the filter is likely to declare itself full before `capacity` is reached
    /// and therefore fill rate will likely never reach 100%.
    /// The fill rate can be improved by using a larger `bucketsize` at the cost of a higher error rate.
    /// When the filter self-declare itself `full`, it will auto-expand by generating additional sub-filters at the cost of reduced performance and increased error rate.
    /// The new sub-filter is created with size of the previous sub-filter multiplied by `expansion`.
    /// Like bucket size, additional sub-filters grow the error rate linearly.
    /// The size of the new sub-filter is the size of the last sub-filter multiplied by `expansion`.
    ///
    /// The minimal false positive error rate is 2/255 ≈ 0.78% when bucket size of 1 is used.
    /// Larger buckets increase the error rate linearly (for example, a bucket size of 3 yields a 2.35% error rate) but improve the fill rate of the filter.
    ///
    /// `maxiterations` dictates the number of attempts to find a slot for the incoming fingerprint.
    /// Once the filter gets full, high `maxIterations` value will slow down insertions.
    ///
    /// Unused capacity in prior sub-filters is automatically used when possible. The filter can grow up to 32 times.
    ///
    /// # Arguments
    /// * `key` - The key under which the filter is found
    /// * `capacity` - Estimated capacity for the filter.
    /// Capacity is rounded to the next 2^n number.
    /// The filter will likely not fill up to 100% of it's capacity.
    /// Make sure to reserve extra capacity if you want to avoid expansions.
    /// * `options` - See [`CfReserveOptions`](CfReserveOptions)
    ///
    /// # See Also
    /// [<https://redis.io/commands/cf.reserve/>](https://redis.io/commands/cf.reserve/)
    #[must_use]
    fn cf_reserve(
        &mut self,
        key: impl SingleArg,
        capacity: usize,
        options: CfReserveOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.RESERVE").arg(key).arg(capacity).arg(options))
    }

    /// Begins an incremental save of the cuckoo filter.
    /// This is useful for large cuckoo filters which cannot fit into the normal [`dump`](crate::commands::GenericCommands::dump)
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
    /// [<https://redis.io/commands/cf.scandump/>](https://redis.io/commands/cf.scandump/)
    #[must_use]
    fn cf_scandump(
        &mut self,
        key: impl SingleArg,
        iterator: i64,
    ) -> PreparedCommand<Self, CfScanDumpResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CF.SCANDUMP").arg(key).arg(iterator))
    }
}

/// Result for the [`cf_info`](CuckooCommands::cf_info) command.
#[derive(Debug, Deserialize)]
pub struct CfInfoResult {
    /// Size
    #[serde(rename = "Size")]
    pub size: usize,
    /// Number of buckets
    #[serde(rename = "Number of buckets")]
    pub num_buckets: usize,
    /// Number of filters
    #[serde(rename = "Number of filters")]
    pub num_filters: usize,
    /// Number of items inserted
    #[serde(rename = "Number of items inserted")]
    pub num_items_inserted: usize,
    /// Number of items deleted
    #[serde(rename = "Number of items deleted")]
    pub num_items_deleted: usize,
    /// Bucket size
    #[serde(rename = "Bucket size")]
    pub bucket_size: usize,
    /// Expansion rate
    #[serde(rename = "Expansion rate")]
    pub expansion_rate: usize,
    /// Max iteration
    #[serde(rename = "Max iterations")]
    pub max_iteration: usize,
    /// Additional information
    #[serde(flatten)]
    pub additional_info: HashMap<String, Value>,
}

/// Options for the [`cf_insert`](CuckooCommands::cf_insert) command.
#[derive(Default)]
pub struct CfInsertOptions {
    command_args: CommandArgs,
}

impl CfInsertOptions {
    /// Specifies the desired capacity of the new filter, if this filter does not exist yet.
    ///
    /// If the filter already exists, then this parameter is ignored.
    /// If the filter does not exist yet and this parameter is not specified,
    /// then the filter is created with the module-level default capacity which is `1024`.
    /// See [`cf_reserve`](CuckooCommands::cf_reserve) for more information on cuckoo filter capacities.
    #[must_use]
    pub fn capacity(self, capacity: usize) -> Self {
        Self {
            command_args: self.command_args.arg("CAPACITY").arg(capacity),
        }
    }

    /// If specified, prevents automatic filter creation if the filter does not exist.
    ///
    /// Instead, an error is returned if the filter does not already exist.
    /// This option is mutually exclusive with [`capacity`](CfInsertOptions::capacity).
    #[must_use]
    pub fn nocreate(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOCREATE"),
        }
    }
}

impl IntoArgs for CfInsertOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`cf_reserve`](CuckooCommands::cf_reserve) command.
#[derive(Default)]
pub struct CfReserveOptions {
    command_args: CommandArgs,
}

impl CfReserveOptions {
    /// Number of items in each bucket.
    ///
    /// A higher bucket size value improves the fill rate but also causes a higher error rate and slightly slower performance.
    /// The default value is 2.
    #[must_use]
    pub fn bucketsize(self, bucketsize: usize) -> Self {
        Self {
            command_args: self.command_args.arg("BUCKETSIZE").arg(bucketsize),
        }
    }

    /// Number of attempts to swap items between buckets before declaring filter as full and creating an additional filter.
    ///
    ///  A low value is better for performance and a higher number is better for filter fill rate.
    /// The default value is 20.
    pub fn maxiterations(self, maxiterations: usize) -> Self {
        Self {
            command_args: self.command_args.arg("MAXITERATIONS").arg(maxiterations),
        }
    }

    /// When a new filter is created, its size is the size of the current filter multiplied by `expansion`.
    /// Expansion is rounded to the next `2^n` number.
    /// The default value is 1.
    #[must_use]
    pub fn expansion(self, expansion: usize) -> Self {
        Self {
            command_args: self.command_args.arg("EXPANSION").arg(expansion),
        }
    }
}

impl IntoArgs for CfReserveOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`cf_scandump`](CuckooCommands::cf_scandump) command.
#[derive(Debug, Deserialize)]
pub struct CfScanDumpResult {
    pub iterator: i64,
    #[serde(deserialize_with = "deserialize_byte_buf")]
    pub data: Vec<u8>,
}
