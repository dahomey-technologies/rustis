use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{cmd, CommandArgs, CollectionResponse, IntoArgs, SingleArg, SingleArgCollection, Value},
};
use serde::Deserialize;
use std::collections::HashMap;

/// A group of Redis commands related to [`T-Digest`](https://redis.io/docs/stack/bloom/)
///
/// # See Also
/// [T-Digest Commands](https://redis.io/commands/?group=tdigest)
pub trait TDigestCommands<'a> {
    /// Adds one or more observations to a t-digest sketch.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `values` - collection values of an observation (floating-point).
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.add/>](https://redis.io/commands/tdigest.add/)
    #[must_use]
    fn tdigest_add(
        self,
        key: impl SingleArg,
        values: impl SingleArgCollection<f64>,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.ADD").arg(key).arg(values))
    }

    /// Returns, for each input rank, an estimation of the value (floating-point) with that rank.
    ///
    /// Multiple estimations can be retrieved in a single call.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `ranks` - collection of ranks, for which the value should be retrieved.
    ///   * `0` - is the rank of the value of the smallest observation.
    ///   * `n-1` - is the rank of the value of the largest observation; `n` denotes the number of observations added to the sketch.
    ///
    /// # Return
    /// a collection of floating-points populated with value_1, value_2, ..., value_R:
    /// * Return an accurate result when rank is `0` (the value of the smallest observation)
    /// * Return an accurate result when rank is `n-1` (the value of the largest observation), \
    ///   where n denotes the number of observations added to the sketch.
    /// * Return `inf` when rank is equal to n or larger than `n`
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.byrank/>](https://redis.io/commands/tdigest.byrank/)
    #[must_use]
    fn tdigest_byrank<R: CollectionResponse<f64>>(
        self,
        key: impl SingleArg,
        ranks: impl SingleArgCollection<usize>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.BYRANK").arg(key).arg(ranks))
    }

    /// Returns, for each input reverse rank, an estimation of the value (floating-point) with that reverse rank.
    ///
    /// Multiple estimations can be retrieved in a single call.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `ranks` - collection of reverse ranks, for which the value should be retrieved.
    ///   * `0` -  is the reverse rank of the value of the largest observation.
    ///   * `n-1` - s the reverse rank of the value of the smallest observation; n denotes the number of observations added to the sketch.
    ///
    /// # Return
    /// a collection of floating-points populated with value_1, value_2, ..., value_R:
    /// * Return an accurate result when `revrank` is `0` (the value of the largest observation)
    /// * Return an accurate result when `revrank` is `n-1` (the value of the smallest observation), \
    ///   where `n` denotes the number of observations added to the sketch.
    /// * Return 'inf' when `revrank` is equal to `n` or larger than `n`
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.byrevrank/>](https://redis.io/commands/tdigest.byrevrank/)
    #[must_use]
    fn tdigest_byrevrank<R: CollectionResponse<f64>>(
        self,
        key: impl SingleArg,
        ranks: impl SingleArgCollection<usize>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.BYREVRANK").arg(key).arg(ranks))
    }

    /// Returns, for each input reverse rank, an estimation of the value (floating-point) with that reverse rank.
    ///
    /// Multiple estimations can be retrieved in a single call.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `values` - collection values for which the CDF \
    ///   ([`Cumulative Distribution Function`](https://en.wikipedia.org/wiki/Cumulative_distribution_function)) should be retrieved.
    ///
    /// # Return
    /// a collection of floating-points populated with fraction_1, fraction_2, ..., fraction_N.
    ///
    /// All values are `nan` if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.cdf/>](https://redis.io/commands/tdigest.cdf/)
    #[must_use]
    fn tdigest_cdf<V: SingleArg, R: CollectionResponse<f64>>(
        self,
        key: impl SingleArg,
        values: impl SingleArgCollection<V>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.CDF").arg(key).arg(values))
    }

    /// Allocates memory and initializes a new t-digest sketch.
    ///
    /// # Arguments
    /// * `key` - key name for this new t-digest sketch.
    /// * `compression` - controllable tradeoff between accuracy and memory consumption. \
    ///  100 is a common value for normal uses. 1000 is more accurate. \
    ///  If no value is passed by default the compression will be 100. \
    ///  For more information on scaling of accuracy versus the compression parameter,\
    ///  see [`The t-digest: Efficient estimates of distributions`](https://www.sciencedirect.com/science/article/pii/S2665963820300403).
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.create/>](https://redis.io/commands/tdigest.create/)
    #[must_use]
    fn tdigest_create(
        self,
        key: impl SingleArg,
        compression: Option<i64>,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TDIGEST.CREATE")
                .arg(key)
                .arg(compression.map(|c| ("COMPRESSION", c))),
        )
    }

    /// Returns information and statistics about a t-digest sketch
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    ///
    /// # Return
    /// An instance of [`TDigestInfoResult`](TDigestInfoResult)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.info/>](https://redis.io/commands/tdigest.info/)
    #[must_use]
    fn tdigest_info(self, key: impl SingleArg) -> PreparedCommand<'a, Self, TDigestInfoResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.INFO").arg(key))
    }

    /// Returns the maximum observation value from a t-digest sketch.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    ///
    /// # Return
    /// maximum observation value from a sketch. The result is always accurate.
    /// `nan` if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.max/>](https://redis.io/commands/tdigest.max/)
    #[must_use]
    fn tdigest_max(self, key: impl SingleArg) -> PreparedCommand<'a, Self, f64>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.MAX").arg(key))
    }

    /// Merges multiple t-digest sketches into a single sketch.
    ///
    /// # Arguments
    /// * `destination` - key name for a t-digest sketch to merge observation values to.
    ///   * If `destination` not exist, a new sketch is created.
    ///   * If `destination` is an existing sketch, its values are merged with the values of the source keys.\
    ///    To override the destination key contents use [`override`](TDigestMergeOptions::_override).
    /// * `sources` - collection of key names for t-digest sketches to merge observation values from.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.merge/>](https://redis.io/commands/tdigest.merge/)
    #[must_use]
    fn tdigest_merge<S: SingleArg>(
        self,
        destination: impl SingleArg,
        sources: impl SingleArgCollection<S>,
        options: TDigestMergeOptions,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TDIGEST.MERGE")
                .arg(destination)
                .arg(sources.num_args())
                .arg(sources)
                .arg(options),
        )
    }

    /// Returns the minimum observation value from a t-digest sketch.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    ///
    /// # Return
    /// minimum observation value from a sketch. The result is always accurate.
    /// `nan` if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.min/>](https://redis.io/commands/tdigest.min/)
    #[must_use]
    fn tdigest_min(self, key: impl SingleArg) -> PreparedCommand<'a, Self, f64>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.MIN").arg(key))
    }

    /// Returns, for each input fraction, an estimation of the value
    /// (floating point) that is smaller than the given fraction of observations.
    ///
    /// Multiple quantiles can be retrieved in a signle call.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `quantiles` - collection of quantiles which are input fractions (between 0 and 1 inclusively)
    ///
    /// # Return
    /// a collection of estimates (floating-point) populated with value_1, value_2, ..., value_N.
    /// * Return an accurate result when quantile is 0 (the value of the smallest observation)
    /// * Return an accurate result when quantile is 1 (the value of the largest observation)
    ///
    /// All values are `nan` if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.quantile/>](https://redis.io/commands/tdigest.quantile/)
    #[must_use]
    fn tdigest_quantile<Q: SingleArg, R: CollectionResponse<f64>>(
        self,
        key: impl SingleArg,
        quantiles: impl SingleArgCollection<Q>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.QUANTILE").arg(key).arg(quantiles))
    }

    /// Returns, for each input value (floating-point), the estimated rank of the value
    /// (the number of observations in the sketch that are smaller than the value + half the number of observations that are equal to the value).
    ///
    /// Multiple ranks can be retrieved in a signle call.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `values` - collection of values for which the rank should be estimated.
    ///
    /// # Return
    /// a collection of integers populated with rank_1, rank_2, ..., rank_V:
    /// * `-1` - when `value` is smaller than the value of the smallest observation.
    /// * The number of observations - when `value` is larger than the value of the largest observation.
    /// * Otherwise: an estimation of the number of (observations smaller than `value` + half the observations equal to `value`).
    ///
    /// `0` is the rank of the value of the smallest observation.
    ///
    /// `n-1` is the rank of the value of the largest observation; `n` denotes the number of observations added to the sketch.
    ///
    /// All values are `-2` if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.rank/>](https://redis.io/commands/tdigest.rank/)
    #[must_use]
    fn tdigest_rank<V: SingleArg, R: CollectionResponse<isize>>(
        self,
        key: impl SingleArg,
        values: impl SingleArgCollection<V>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.RANK").arg(key).arg(values))
    }

    /// Resets a t-digest sketch: empty the sketch and re-initializes it.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.reset/>](https://redis.io/commands/tdigest.reset/)
    #[must_use]
    fn tdigest_reset(self, key: impl SingleArg) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.RESET").arg(key))
    }

    /// Returns, for each input value (floating-point), the estimated reverse rank of the value
    /// (the number of observations in the sketch that are smaller than the value + half the number of observations that are equal to the value).
    ///
    /// Multiple reverse ranks can be retrieved in a signle call.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `values` - collection of values for which the reverse rank should be estimated.
    ///
    /// # Return
    /// a collection of integers populated with revrank_1, revrank_2, ..., revrank_V:
    /// * `-1` - when `value` is smaller than the value of the smallest observation.
    /// * The number of observations - when `value` is larger than the value of the largest observation.
    /// * Otherwise: an estimation of the number of (observations smaller than `value` + half the observations equal to `value`).
    ///
    /// `0` is the reverse rank of the value of the smallest observation.
    ///
    /// `n-1` is the reverse rank of the value of the largest observation; `n` denotes the number of observations added to the sketch.
    ///
    /// All values are `-2` if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.revrank/>](https://redis.io/commands/tdigest.revrank/)
    #[must_use]
    fn tdigest_revrank<V: SingleArg, R: CollectionResponse<isize>>(
        self,
        key: impl SingleArg,
        values: impl SingleArgCollection<V>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TDIGEST.REVRANK").arg(key).arg(values))
    }

    /// Returns an estimation of the mean value from the sketch, excluding observation values outside the low and high cutoff quantiles.
    ///
    /// # Arguments
    /// * `key` - key name for an existing t-digest sketch.
    /// * `low_cut_quantile` - Foating-point value in the range [0..1], should be lower than `high_cut_quantile` \
    ///    When equal to 0: No low cut. \
    ///    When higher than 0: Exclude observation values lower than this quantile.
    /// * `high_cut_quantile` - Floating-point value in the range [0..1], should be higher than `low_cut_quantile` \
    ///    When lower than 1: Exclude observation values higher than or equal to this quantile. \
    ///    When equal to 1: No high cut.
    ///
    /// # Return
    /// estimation of the mean value. 'nan' if the sketch is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/tdigest.trimmed_mean/>](https://redis.io/commands/tdigest.trimmed_mean/)
    #[must_use]
    fn tdigest_trimmed_mean(
        self,
        key: impl SingleArg,
        low_cut_quantile: f64,
        high_cut_quantile: f64,
    ) -> PreparedCommand<'a, Self, f64>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TDIGEST.TRIMMED_MEAN")
                .arg(key)
                .arg(low_cut_quantile)
                .arg(high_cut_quantile),
        )
    }
}

/// Result for the [`tdigest_info`](TDigestCommands::tdigest_info) command.
#[derive(Debug, Deserialize)]
pub struct TDigestInfoResult {
    /// The compression (controllable trade-off between accuracy and memory consumption) of the sketch
    #[serde(rename = "Compression")]
    pub compression: usize,
    /// Size of the buffer used for storing the centroids and for the incoming unmerged observations
    #[serde(rename = "Capacity")]
    pub capacity: usize,
    /// Number of merged observations
    #[serde(rename = "Merged nodes")]
    pub merged_nodes: usize,
    /// Number of buffered nodes (uncompressed observations)
    #[serde(rename = "Unmerged nodes")]
    pub unmerged_nodes: usize,
    /// Weight of values of the merged nodes
    #[serde(rename = "Merged weight")]
    pub merged_weight: usize,
    /// Weight of values of the unmerged nodes (uncompressed observations)
    #[serde(rename = "Unmerged weight")]
    pub unmerged_weight: usize,
    /// Number of observations added to the sketch
    #[serde(rename = "Observations")]
    pub observations: usize,
    /// Number of times this sketch compressed data together
    #[serde(rename = "Total compressions")]
    pub total_compressions: usize,
    /// Number of bytes allocated for the sketch
    #[serde(rename = "Memory usage")]
    pub memory_usage: usize,
    /// Additional information
    #[serde(flatten)]
    pub additional_info: HashMap<String, Value>,
}

/// Options for the [`tdigest_merge`](TDigestCommands::tdigest_merge) command.
#[derive(Default)]
pub struct TDigestMergeOptions {
    command_args: CommandArgs,
}

impl TDigestMergeOptions {
    /// controllable tradeoff between accuracy and memory consumption.
    ///
    /// 100 is a common value for normal uses.
    /// 1000 is more accurate.
    /// If no value is passed by default the compression will be 100.
    /// For more information on scaling of accuracy versus the compression parameter
    /// see [`The t-digest: Efficient estimates of distributions`](https://www.sciencedirect.com/science/article/pii/S2665963820300403).
    ///
    /// When COMPRESSION is not specified:
    /// * If `destination` does not exist or if [`override`](TDigestMergeOptions::_override) is specified, \
    ///   the compression is set to the maximal value among all source sketches.
    /// * If `destination` already exists and [`override`](TDigestMergeOptions::_override) is not specified, \
    ///   its compression is not changed.
    #[must_use]
    pub fn compression(self, compression: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COMPRESSION").arg(compression),
        }
    }

    /// When specified, if `destination` already exists, it is overwritten.
    #[must_use]
    pub fn _override(self) -> Self {
        Self {
            command_args: self.command_args.arg("OVERRIDE"),
        }
    }
}

impl ToArgs for TDigestMergeOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(self.command_args)
    }
}
