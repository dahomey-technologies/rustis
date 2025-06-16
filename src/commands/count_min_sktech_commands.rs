use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{cmd, CollectionResponse, KeyValueArgsCollection, SingleArg, SingleArgCollection},
};
use serde::Deserialize;

/// A group of Redis commands related to [`Count-min Sketch`](https://redis.io/docs/stack/bloom/)
///
/// # See Also
/// [Count-min Sketch Commands](https://redis.io/commands/?group=cms)
pub trait CountMinSketchCommands<'a> {
    /// Increases the count of item by increment.
    ///
    /// Multiple items can be increased with one call.
    ///
    /// # Arguments
    /// * `key` - The name of the sketch.
    /// * `items` - A collection of tuples of
    ///   * `item` - The item which counter is to be increased.
    ///   * `increment`- Amount by which the item counter is to be increased.
    ///
    /// # Return
    /// A collection of count of each item after increment.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cms.incrby/>](https://redis.io/commands/cms.incrby/)
    #[must_use]
    fn cms_incrby<I: SingleArg, R: CollectionResponse<usize>>(
        self,
        key: impl SingleArg,
        items: impl KeyValueArgsCollection<I, usize>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CMS.INCRBY").arg(key).arg(items))
    }

    /// Returns width, depth and total count of the sketch.
    ///
    /// # Arguments
    /// * `key` - The name of the sketch.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cms.info/>](https://redis.io/commands/cms.info/)
    #[must_use]
    fn cms_info(self, key: impl SingleArg) -> PreparedCommand<'a, Self, CmsInfoResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CMS.INFO").arg(key))
    }

    /// Initializes a Count-Min Sketch to dimensions specified by user.
    ///
    /// Multiple items can be increased with one call.
    ///
    /// # Arguments
    /// * `key` - The name of the sketch.
    /// * `width` - Number of counters in each array. Reduces the error size.
    /// * `depth` - Number of counter-arrays. Reduces the probability for an error of a certain size (percentage of total count).
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cms.initbydim/>](https://redis.io/commands/cms.initbydim/)
    #[must_use]
    fn cms_initbydim(
        self,
        key: impl SingleArg,
        width: usize,
        depth: usize,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CMS.INITBYDIM").arg(key).arg(width).arg(depth))
    }

    /// Initializes a Count-Min Sketch to accommodate requested tolerances.
    ///
    /// # Arguments
    /// * `key` - The name of the sketch.
    /// * `error` - Estimate size of error.\
    ///   The error is a percent of total counted items. This effects the width of the sketch.
    /// * `probability` - The desired probability for inflated count. \
    ///   This should be a decimal value between 0 and 1.
    ///   This effects the depth of the sketch.
    ///   For example, for a desired false positive rate of 0.1% (1 in 1000),
    ///   error_rate should be set to 0.001. The closer this number is to zero,
    ///   the greater the memory consumption per item and the more CPU usage per operation.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cms.initbyprob/>](https://redis.io/commands/cms.initbyprob/)
    #[must_use]
    fn cms_initbyprob(
        self,
        key: impl SingleArg,
        error: f64,
        probability: f64,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("CMS.INITBYPROB").arg(key).arg(error).arg(probability),
        )
    }

    /// Returns the count for one or more items in a sketch.
    ///
    /// All sketches must have identical width and depth.
    /// Weights can be used to multiply certain sketches.
    /// Default weight is 1.
    ///
    /// # Arguments
    /// * `destination` - The name of destination sketch. Must be initialized.
    /// * `sources` - Names of source sketches to be merged.
    /// * `weights` - Multiple of each sketch. Default =1.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cms.merge/>](https://redis.io/commands/cms.merge/)
    #[must_use]
    fn cms_merge<S: SingleArg, W: SingleArgCollection<usize>>(
        self,
        destination: impl SingleArg,
        sources: impl SingleArgCollection<S>,
        weights: Option<W>,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("CMS.MERGE")
                .arg(destination)
                .arg(sources.num_args())
                .arg(sources)
                .arg(weights.map(|w| ("WEIGHTS", w))),
        )
    }

    /// Merges several sketches into one sketch.
    ///
    /// All sketches must have identical width and depth.
    /// Weights can be used to multiply certain sketches.
    /// Default weight is 1.
    ///
    /// # Arguments
    /// * `key` - The name of the sketch.
    /// * `item` - One or more items for which to return the count.
    ///
    /// # Return
    /// Count of one or more items
    ///
    /// # See Also
    /// * [<https://redis.io/commands/cms.query/>](https://redis.io/commands/cms.query/)
    #[must_use]
    fn cms_query<I: SingleArg, C: CollectionResponse<usize>>(
        self,
        key: impl SingleArg,
        items: impl SingleArgCollection<I>,
    ) -> PreparedCommand<'a, Self, C>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CMS.QUERY").arg(key).arg(items))
    }
}

/// Result for the [`cms_info`](CountMinSketchCommands::cms_info) command.
#[derive(Debug, Deserialize)]
pub struct CmsInfoResult {
    /// Width of the sketch
    pub width: usize,
    /// Depth of the sketch
    pub depth: usize,
    /// Total count of the sketch
    #[serde(rename = "count")]
    pub total_count: usize,
}
