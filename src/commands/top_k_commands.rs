use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, Response, cmd, deserialize_vec_of_pairs},
};
use serde::{Deserialize, de::DeserializeOwned};

/// A group of Redis commands related to [`Top-K`](https://redis.io/docs/stack/bloom/)
///
/// # See Also
/// [Top-K Commands](https://redis.io/commands/?group=topk)
pub trait TopKCommands<'a>: Sized {
    /// Adds an item to the data structure.
    ///
    /// Multiple items can be added at once.
    /// If an item enters the Top-K list, the item which is expelled is returned.
    /// This allows dynamic heavy-hitter detection of items being entered or expelled from Top-K list.
    ///
    /// # Arguments
    /// * `key` - Name of sketch where item is added.
    /// * `items` - Item/s to be added.
    ///
    /// # Return
    /// Collection of items if an element was dropped from the TopK list, Null reply otherwise.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.add/>](https://redis.io/commands/topk.add/)
    #[must_use]
    fn topk_add<R: Response>(
        self,
        key: impl Args,
        items: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TOPK.ADD").arg(key).arg(items))
    }

    /// Increase the score of an item in the data structure by increment.
    ///
    /// Multiple items' score can be increased at once.
    /// If an item enters the Top-K list, the item which is expelled is returned.
    ///
    /// # Arguments
    /// * `key` - Name of sketch where item is added.
    /// * `items` - collection of tuples:
    ///   * `item` - Item to be added
    ///   * `increment` - increment to current item score. \
    ///     Increment must be greater or equal to 1. \
    ///     Increment is limited to 100,000 to avoid server freeze.
    ///
    /// # Return
    /// Collection of items if an element was dropped from the TopK list, Null reply otherwise.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.incrby/>](https://redis.io/commands/topk.incrby/)
    #[must_use]
    fn topk_incrby<R: Response>(
        self,
        key: impl Args,
        items: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TOPK.INCRBY").arg(key).arg(items))
    }

    /// Returns number of required items (k), width, depth and decay values.
    ///
    /// # Arguments
    /// * `key` - Name of sketch
    ///
    /// # Return
    /// An instance of [`TopKInfoResult`](TopKInfoResult)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.info/>](https://redis.io/commands/topk.info/)
    #[must_use]
    fn topk_info(self, key: impl Args) -> PreparedCommand<'a, Self, TopKInfoResult> {
        prepare_command(self, cmd("TOPK.INFO").arg(key))
    }

    /// Return full list of items in Top K list.
    ///
    /// # Arguments
    /// * `key` - Key under which the sketch is to be found.
    ///
    /// # Return
    /// a collection of k (or less) items in Top K list.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.list/>](https://redis.io/commands/topk.list/)
    #[must_use]
    fn topk_list<R: Response>(self, key: impl Args) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TOPK.LIST").arg(key))
    }

    /// Return full list of items in Top K list.
    ///
    /// # Arguments
    /// * `key` - Key under which the sketch is to be found.
    ///
    /// # Return
    /// a collection of k (or less) pairs of items in Top K list. Each pair holds:
    /// * the name of the item
    /// * the count of the item
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.list/>](https://redis.io/commands/topk.list/)
    #[must_use]
    fn topk_list_with_count<R: Response + DeserializeOwned>(
        self,
        key: impl Args,
    ) -> PreparedCommand<'a, Self, TopKListWithCountResult<R>> {
        prepare_command(self, cmd("TOPK.LIST").arg(key).arg("WITHCOUNT"))
    }

    /// Return full list of items in Top K list.
    ///
    /// # Arguments
    /// * `key` - Key under which the sketch is to be found.
    /// * `items` - Item/s to be queried.
    ///
    /// # Return
    /// a collection of k boolean:
    /// * `true` - if item is in Top-K
    /// * `false` - otherwise
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.query/>](https://redis.io/commands/topk.query/)
    #[must_use]
    fn topk_query<R: Response>(
        self,
        key: impl Args,
        items: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TOPK.QUERY").arg(key).arg(items))
    }

    /// Initializes a TopK with specified parameters.
    ///
    /// # Arguments
    /// * `key` - Key under which the sketch is to be found.
    /// * `topk` - Number of top occurring items to keep.
    /// * `width_depth_decay` - Optional paramaters:
    ///   * `width` - Number of counters kept in each array. (Default 8)
    ///   * `depth` - Number of arrays. (Default 7)
    ///   * `decay` - The probability of reducing a counter in an occupied bucket. \
    ///     It is raised to power of it's counter (decay ^ bucket\[i\].counter). \
    ///     Therefore, as the counter gets higher, the chance of a reduction is being reduced. (Default 0.9)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/topk.reserve/>](https://redis.io/commands/topk.reserve/)
    #[must_use]
    fn topk_reserve(
        self,
        key: impl Args,
        topk: usize,
        width_depth_decay: Option<(usize, usize, f64)>,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("TOPK.RESERVE")
                .arg(key)
                .arg(topk)
                .arg(width_depth_decay),
        )
    }
}

/// Result for the [`topk_info`](TopKCommands::topk_info) command.
#[derive(Debug, Deserialize)]
pub struct TopKInfoResult {
    /// The number of required items
    pub k: usize,
    /// Number of counters kept in each array.
    pub width: usize,
    /// Number of arrays. (
    pub depth: usize,
    /// The probability of reducing a counter in an occupied bucket.
    ///
    /// It is raised to power of it's counter (decay ^ bucket\[i\].counter).
    /// Therefore, as the counter gets higher, the chance of a reduction is being reduced.
    pub decay: f64,
}

#[derive(Debug)]
pub struct TopKListWithCountResult<R: Response> {
    pub items: Vec<(R, usize)>,
}

impl<'de, R: Response + DeserializeOwned> Deserialize<'de> for TopKListWithCountResult<R> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(TopKListWithCountResult {
            items: deserialize_vec_of_pairs(deserializer)?,
        })
    }
}
