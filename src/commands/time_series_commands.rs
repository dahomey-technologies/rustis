use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Response, Value, cmd, serialize_flag},
};
use serde::{
    Deserialize, Serialize,
    de::{self, value::SeqAccessDeserializer},
};
use smallvec::SmallVec;
use std::{collections::HashMap, fmt};

/// A group of Redis commands related to [`Time Series`](https://redis.io/docs/stack/timeseries/)
///
/// # See Also
/// [Time Series Commands](https://redis.io/commands/?group=timeseries)
pub trait TimeSeriesCommands<'a>: Sized {
    /// Append a sample to a time series
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `timestamp` - UNIX sample timestamp in milliseconds or `*` to set the timestamp according to the server clock.
    /// * `values` - numeric data value of the sample. The double number should follow [`RFC 7159`](https://tools.ietf.org/html/rfc7159)
    ///   (JSON standard). In particular, the parser rejects overly large values that do not fit in binary64. It does not accept `NaN` or `infinite` values.
    ///
    /// # Return
    /// The UNIX sample timestamp in milliseconds
    ///
    /// # Notes
    /// * When specified key does not exist, a new time series is created.
    /// * if a [`COMPACTION_POLICY`](https://redis.io/docs/stack/timeseries/configuration/#compaction_policy) configuration parameter is defined,
    ///   compacted time series would be created as well.
    /// * If timestamp is older than the retention period compared to the maximum existing timestamp, the sample is discarded and an error is returned.
    /// * When adding a sample to a time series for which compaction rules are defined:
    ///   * If all the original samples for an affected aggregated time bucket are available,
    ///     the compacted value is recalculated based on the reported sample and the original samples.
    ///   * If only a part of the original samples for an affected aggregated time bucket is available
    ///     due to trimming caused in accordance with the time series RETENTION policy, the compacted value
    ///     is recalculated based on the reported sample and the available original samples.
    ///   * If the original samples for an affected aggregated time bucket are not available due to trimming
    ///     caused in accordance with the time series RETENTION policy, the compacted value bucket is not updated.
    ///  * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add), [`ts_madd`](TimeSeriesCommands::ts_madd),
    ///    [`ts_incrby`](TimeSeriesCommands::ts_incrby), or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result
    ///    in inconsistencies between the raw and the compacted data. The compaction process may override such samples.
    ///
    /// # Complexity
    /// If a compaction rule exits on a time series, the performance of `ts_add` can be reduced.
    /// The complexity of `ts_add` is always `O(M)`, where `M` is the number of compaction rules or `O(1)` with no compaction.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.add/>](https://redis.io/commands/ts.add/)
    #[must_use]
    fn ts_add(
        self,
        key: impl Serialize,
        timestamp: TsTimestamp,
        value: f64,
        options: TsAddOptions,
    ) -> PreparedCommand<'a, Self, u64> {
        prepare_command(
            self,
            cmd("TS.ADD")
                .key(key)
                .arg(timestamp)
                .arg(value)
                .arg(options),
        )
    }

    /// Update the retention, chunk size, duplicate policy, and labels of an existing time series
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `options` - options to alter the existing time series. [`encoding`](TsCreateOptions::encoding) cannot be used on this command.
    ///
    /// # Notes
    /// This command alters only the specified element.
    /// For example, if you specify only retention and labels, the chunk size and the duplicate policy are not altered.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.alter/>](https://redis.io/commands/ts.alter/)
    #[must_use]
    fn ts_alter(
        self,
        key: impl Serialize,
        options: TsCreateOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("TS.ALTER").key(key).arg(options))
    }

    /// Create a new time series
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    ///
    /// # Notes
    /// * If a key already exists, you get a Redis error reply, TSDB: key already exists.
    ///   You can check for the existence of a key with the [`exists`](crate::commands::GenericCommands::exists) command.
    /// * Other commands that also create a new time series when called with a key that does not exist are
    ///   [`ts_add`](TimeSeriesCommands::ts_add), [`ts_incrby`](TimeSeriesCommands::ts_incrby), and [`ts_decrby`](TimeSeriesCommands::ts_decrby).
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.create/>](https://redis.io/commands/ts.create/)
    #[must_use]
    fn ts_create(
        self,
        key: impl Serialize,
        options: TsCreateOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("TS.CREATE").key(key).arg(options))
    }

    /// Create a compaction rule
    ///
    /// # Arguments
    /// * `src_key` - key name for the source time series.
    /// * `dst_key` - key name for destination (compacted) time series.
    ///   It must be created before `ts_createrule` is called.
    /// * `aggregator` - aggregates results into time buckets by taking an aggregation type
    /// * `bucket_duration` - duration of each aggregation bucket, in milliseconds.
    /// * `options` - See [`TsCreateRuleOptions`](TsCreateRuleOptions)
    ///
    /// # Notes
    /// * Only new samples that are added into the source series after the creation of the rule will be aggregated.
    /// * Calling `ts_createrule` with a nonempty `dst_key` may result in inconsistencies between the raw and the compacted data.
    /// * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add),
    ///   [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby), or [`ts_decrby`](TimeSeriesCommands::ts_decrby))
    ///   may result in inconsistencies between the raw and the compacted data. The compaction process may override such samples.
    /// * If no samples are added to the source time series during a bucket period. no compacted sample is added to the destination time series.
    /// * The timestamp of a compacted sample added to the destination time series is set to the start timestamp the appropriate compaction bucket.
    ///   For example, for a 10-minute compaction bucket with no alignment, the compacted samples timestamps are `x:00`, `x:10`, `x:20`, and so on.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.createrule/>](https://redis.io/commands/ts.createrule/)
    #[must_use]
    fn ts_createrule(
        self,
        src_key: impl Serialize,
        dst_key: impl Serialize,
        aggregator: TsAggregationType,
        bucket_duration: u64,
        options: TsCreateRuleOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("TS.CREATERULE")
                .key(src_key)
                .key(dst_key)
                .arg("AGGREGATION")
                .arg(aggregator)
                .arg(bucket_duration)
                .arg(options),
        )
    }

    /// Decrease the value of the sample with the maximum existing timestamp,
    /// or create a new sample with a value equal to the value of the sample with the maximum existing timestamp with a given decrement
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `value` - numeric data value of the sample
    /// * `options` - See [`TsIncrByDecrByOptions`](TsIncrByDecrByOptions)
    ///
    /// # Notes
    /// * When specified key does not exist, a new time series is created.
    /// * You can use this command as a counter or gauge that automatically gets history as a time series.
    /// * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add),
    ///   [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    ///   or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result in inconsistencies between the raw and the compacted data.
    ///   The compaction process may override such samples.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.decrby/>](https://redis.io/commands/ts.decrby/)
    #[must_use]
    fn ts_decrby(
        self,
        key: impl Serialize,
        value: f64,
        options: TsIncrByDecrByOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("TS.DECRBY").key(key).arg(value).arg(options))
    }

    /// Delete all samples between two timestamps for a given time series
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `from_timestamp` - start timestamp for the range deletion.
    /// * `to_timestamp` - end timestamp for the range deletion.
    ///
    /// # Return
    /// The number of samples that were removed.
    ///
    /// # Notes
    /// The given timestamp interval is closed (inclusive),
    /// meaning that samples whose timestamp equals the `from_timestamp` or `to_timestamp` are also deleted.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.del/>](https://redis.io/commands/ts.del/)
    #[must_use]
    fn ts_del(
        self,
        key: impl Serialize,
        from_timestamp: u64,
        to_timestamp: u64,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("TS.DEL").key(key).arg(from_timestamp).arg(to_timestamp),
        )
    }

    /// Delete a compaction rule
    ///
    /// # Arguments
    /// * `src_key` - key name for the source time series.
    /// * `dst_key` - key name for destination (compacted) time series.
    ///
    /// # Notes
    /// This command does not delete the compacted series.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.deleterule/>](https://redis.io/commands/ts.deleterule/)
    #[must_use]
    fn ts_deleterule(
        self,
        src_key: impl Serialize,
        dst_key: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("TS.DELETERULE").key(src_key).key(dst_key))
    }

    /// Get the last sample
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `options` - See [`TsGetOptions`](TsGetOptions)
    ///
    /// # Return
    /// An option tuple:
    /// * The last sample timestamp, and last sample value, when the time series contains data.
    /// * None, when the time series is empty.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.get/>](https://redis.io/commands/ts.get/)
    #[must_use]
    fn ts_get(
        self,
        key: impl Serialize,
        options: TsGetOptions,
    ) -> PreparedCommand<'a, Self, Option<(u64, f64)>> {
        prepare_command(self, cmd("TS.GET").key(key).arg(options))
    }

    /// Increase the value of the sample with the maximum existing timestamp,
    /// or create a new sample with a value equal to the value of the sample
    /// with the maximum existing timestamp with a given increment
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `value` - numeric data value of the sample
    /// * `options` - See [`TsIncrByDecrByOptions`](TsIncrByDecrByOptions)
    ///
    /// # Notes
    /// * When specified key does not exist, a new time series is created.
    /// * You can use this command as a counter or gauge that automatically gets history as a time series.
    /// * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add),
    ///   [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    ///   or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result in inconsistencies between the raw and the compacted data.
    ///   The compaction process may override such samples.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.incrby/>](https://redis.io/commands/ts.incrby/)
    #[must_use]
    fn ts_incrby(
        self,
        key: impl Serialize,
        value: f64,
        options: TsIncrByDecrByOptions,
    ) -> PreparedCommand<'a, Self, u64> {
        prepare_command(self, cmd("TS.INCRBY").key(key).arg(value).arg(options))
    }

    /// Return information and statistics for a time series.
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
    /// * `debug` - an optional flag to get a more detailed information about the chunks.
    ///
    /// # Return
    /// an instance of [`TsInfoResult`](TsInfoResult)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.info/>](https://redis.io/commands/ts.info/)
    #[must_use]
    fn ts_info(self, key: impl Serialize, debug: bool) -> PreparedCommand<'a, Self, TsInfoResult> {
        prepare_command(self, cmd("TS.INFO").key(key).arg_if(debug, "DEBUG"))
    }

    /// Append new samples to one or more time series
    ///
    /// # Arguments
    /// * `items` - one or more the following tuple:
    ///   * `key` - the key name for the time series.
    ///   * `timestamp` - the UNIX sample timestamp in milliseconds or * to set the timestamp according to the server clock.
    ///   * `value` - numeric data value of the sample (double). \
    ///     The double number should follow [`RFC 7159`](https://tools.ietf.org/html/rfc7159) (a JSON standard).
    ///     The parser rejects overly large values that would not fit in binary64.
    ///     It does not accept `NaN` or `infinite` values.
    ///
    /// # Return
    /// a collection of the timestamps of added samples
    ///
    /// # Notes
    /// * If timestamp is older than the retention period compared to the maximum existing timestamp,
    ///   the sample is discarded and an error is returned.
    /// * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add),
    ///   [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    ///   or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result in inconsistencies between the raw and the compacted data.
    ///   The compaction process may override such samples.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.madd/>](https://redis.io/commands/ts.madd/)
    #[must_use]
    fn ts_madd<R: Response>(self, items: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("TS.MADD")
                .key_with_step(items, 3)
                .cluster_info(None, None, 3),
        )
    }

    /// Get the last samples matching a specific filter
    ///
    /// # Arguments
    /// * `options` - See [`TsMGetOptions`](TsMGetOptions)
    /// * `filters` - filters time series based on their labels and label values, with these options:
    ///   * `label=value`, where `label` equals `value`
    ///   * `label!=value`, where `label` does not equal `value`
    ///   * `label=`, where `key` does not have label `label`
    ///   * `label!=`, where `key` has label `label`
    ///   * `label=(_value1_,_value2_,...)`, where `key` with label `label` equals one of the values in the list
    ///   * `label!=(value1,value2,...)` where `key` with label `label` does not equal any of the values in the list
    ///
    /// # Return
    /// A collection of [`TsSample`](TsSample)
    ///
    /// # Notes
    /// * When using filters, apply a minimum of one label=value filter.
    /// * Filters are conjunctive. For example, the FILTER `type=temperature room=study`
    ///   means the time series is a temperature time series of a study room.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.mget/>](https://redis.io/commands/ts.mget/)
    #[must_use]
    fn ts_mget<R: Response>(
        self,
        options: TsMGetOptions,
        filters: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TS.MGET").arg(options).arg("FILTER").arg(filters))
    }

    /// Query a range across multiple time series by filters in forward direction
    ///
    /// # Arguments
    /// * `from_timestamp` - start timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `-` to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `+` to denote the timestamp of the latest sample in the time series.
    /// * `options` - See [`TsMRangeOptions`](TsMRangeOptions)
    /// * `filters` - filters time series based on their labels and label values, with these options:
    ///   * `label=value`, where `label` equals `value`
    ///   * `label!=value`, where `label` does not equal `value`
    ///   * `label=`, where `key` does not have label `label`
    ///   * `label!=`, where `key` has label `label`
    ///   * `label=(_value1_,_value2_,...)`, where `key` with label `label` equals one of the values in the list
    ///   * `label!=(value1,value2,...)` where `key` with label `label` does not equal any of the values in the list
    /// * `groupby_options` - See [`TsGroupByOptions`](TsGroupByOptions)
    ///
    /// # Return
    /// A collection of [`TsRangeSample`](TsRangeSample)
    ///
    /// # Notes
    /// * The `ts_mrange` command cannot be part of transaction when running on a Redis cluster.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.mrange/>](https://redis.io/commands/ts.mrange/)
    #[must_use]
    fn ts_mrange<R: Response>(
        self,
        from_timestamp: impl Serialize,
        to_timestamp: impl Serialize,
        options: TsMRangeOptions,
        filters: impl Serialize,
        groupby_options: TsGroupByOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("TS.MRANGE")
                .arg(from_timestamp)
                .arg(to_timestamp)
                .arg(options)
                .arg("FILTER")
                .arg(filters)
                .arg(groupby_options),
        )
    }

    /// Query a range across multiple time series by filters in reverse direction
    ///
    /// # Arguments
    /// * `from_timestamp` - start timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `-` to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `+` to denote the timestamp of the latest sample in the time series.
    /// * `options` - See [`TsMRangeOptions`](TsMRangeOptions)
    /// * `filters` - filters time series based on their labels and label values, with these options:
    ///   * `label=value`, where `label` equals `value`
    ///   * `label!=value`, where `label` does not equal `value`
    ///   * `label=`, where `key` does not have label `label`
    ///   * `label!=`, where `key` has label `label`
    ///   * `label=(_value1_,_value2_,...)`, where `key` with label `label` equals one of the values in the list
    ///   * `label!=(value1,value2,...)` where `key` with label `label` does not equal any of the values in the list
    /// * `groupby_options` - See [`TsGroupByOptions`](TsGroupByOptions)
    ///
    /// # Return
    /// A collection of [`TsRangeSample`](TsRangeSample)
    ///
    /// # Notes
    /// * The `ts_mrevrange` command cannot be part of transaction when running on a Redis cluster.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.mrevrange/>](https://redis.io/commands/ts.mrevrange/)
    #[must_use]
    fn ts_mrevrange<R: Response>(
        self,
        from_timestamp: impl Serialize,
        to_timestamp: impl Serialize,
        options: TsMRangeOptions,
        filters: impl Serialize,
        groupby_options: TsGroupByOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("TS.MREVRANGE")
                .arg(from_timestamp)
                .arg(to_timestamp)
                .arg(options)
                .arg("FILTER")
                .arg(filters)
                .arg(groupby_options),
        )
    }

    /// Get all time series keys matching a filter list
    ///
    /// # Arguments
    /// * `filters` - filters time series based on their labels and label values, with these options:
    ///   * `label=value`, where `label` equals `value`
    ///   * `label!=value`, where `label` does not equal `value`
    ///   * `label=`, where `key` does not have label `label`
    ///   * `label!=`, where `key` has label `label`
    ///   * `label=(_value1_,_value2_,...)`, where `key` with label `label` equals one of the values in the list
    ///   * `label!=(value1,value2,...)` where `key` with label `label` does not equal any of the values in the list
    ///
    /// # Return
    /// A collection of keys
    ///
    /// # Notes
    /// * When using filters, apply a minimum of one `label=value` filter.
    /// * `ts_queryindex` cannot be part of a transaction that runs on a Redis cluster.
    /// * Filters are conjunctive. For example, the FILTER `type=temperature room=study`
    ///   means the a time series is a temperature time series of a study room.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.queryindex/>](https://redis.io/commands/ts.queryindex/)
    #[must_use]
    fn ts_queryindex<R: Response>(self, filters: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("TS.QUERYINDEX").arg(filters))
    }

    /// Query a range in forward direction
    ///
    /// # Arguments
    /// * `key` - the key name for the time series.
    /// * `from_timestamp` - start timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `-`to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `+` to denote the timestamp of the latest sample in the time series.
    /// * `options` - See [`TsRangeOptions`](TsRangeOptions)
    ///
    /// # Return
    /// A collection of keys
    ///
    /// # Notes
    /// * When the time series is a compaction,
    ///   the last compacted value may aggregate raw values with timestamp beyond `to_timestamp`.
    ///   That is because `to_timestamp` only limits the timestamp of the compacted value,
    ///   which is the start time of the raw bucket that was compacted.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.range/>](https://redis.io/commands/ts.range/)
    #[must_use]
    fn ts_range<R: Response>(
        self,
        key: impl Serialize,
        from_timestamp: impl Serialize,
        to_timestamp: impl Serialize,
        options: TsRangeOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("TS.RANGE")
                .key(key)
                .arg(from_timestamp)
                .arg(to_timestamp)
                .arg(options),
        )
    }

    /// Query a range in reverse direction
    ///
    /// # Arguments
    /// * `key` - the key name for the time series.
    /// * `from_timestamp` - start timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `-`to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///   or `+` to denote the timestamp of the latest sample in the time series.
    /// * `options` - See [`TsRangeOptions`](TsRangeOptions)
    ///
    /// # Return
    /// A collection of keys
    ///
    /// # Notes
    /// * When the time series is a compaction,
    ///   the last compacted value may aggregate raw values with timestamp beyond `to_timestamp`.
    ///   That is because `to_timestamp` only limits the timestamp of the compacted value,
    ///   which is the start time of the raw bucket that was compacted.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.revrange/>](https://redis.io/commands/ts.revrange/)
    #[must_use]
    fn ts_revrange<R: Response>(
        self,
        key: impl Serialize,
        from_timestamp: impl Serialize,
        to_timestamp: impl Serialize,
        options: TsRangeOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("TS.REVRANGE")
                .key(key)
                .arg(from_timestamp)
                .arg(to_timestamp)
                .arg(options),
        )
    }
}

/// Options for the [`ts_add`](TimeSeriesCommands::ts_add) command.
///
/// # Notes
/// * You can use this command to add data to a nonexisting time series in a single command.
///   This is why [`retention`](TsAddOptions::retention), [`encoding`](TsAddOptions::encoding),
///   [`chunk_size`](TsAddOptions::chunk_size), [`on_duplicate`](TsAddOptions::on_duplicate),
///   and [`labels`](TsAddOptions::labels) are optional arguments.
/// * Setting [`retention`](TsAddOptions::retention) and [`labels`](TsAddOptions::labels) introduces additional time complexity.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsAddOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    retention: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<TsEncoding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on_duplicate: Option<TsDuplicatePolicy>,
    #[serde(skip_serializing_if = "SmallVec::is_empty")]
    labels: SmallVec<[(&'a str, &'a str); 10]>,
}

impl<'a> TsAddOptions<'a> {
    /// maximum retention period, compared to the maximum existing timestamp, in milliseconds.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`retention`](TsCreateOptions::retention).
    #[must_use]
    pub fn retention(mut self, retention_period: u64) -> Self {
        self.retention = Some(retention_period);
        self
    }

    /// specifies the series sample's encoding format.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`encoding`](TsCreateOptions::encoding).
    #[must_use]
    pub fn encoding(mut self, encoding: TsEncoding) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// memory size, in bytes, allocated for each data chunk.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`chunk_size`](TsCreateOptions::chunk_size).
    #[must_use]
    pub fn chunk_size(mut self, chunk_size: u32) -> Self {
        self.chunk_size = Some(chunk_size);
        self
    }

    /// overwrite key and database configuration for
    /// [`DUPLICATE_POLICY`](https://redis.io/docs/stack/timeseries/configuration/#duplicate_policy)
    #[must_use]
    pub fn on_duplicate(mut self, policy: TsDuplicatePolicy) -> Self {
        self.on_duplicate = Some(policy);
        self
    }

    /// set of label-value pairs that represent metadata labels of the time series.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`labels`](TsCreateOptions::labels).
    ///
    /// The [`ts_mget`](TimeSeriesCommands::ts_mget), [`ts_mrange`](TimeSeriesCommands::ts_mrange),
    /// and [`ts_mrevrange`](TimeSeriesCommands::ts_mrevrange) commands operate on multiple time series based on their labels.
    /// The [`ts_queryindex`](TimeSeriesCommands::ts_queryindex) command returns all time series keys matching a given filter based on their labels.
    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        self.labels.extend(labels);
        self
    }
}

/// specifies the series samples encoding format.
///
/// `Compressed` is almost always the right choice.
/// Compression not only saves memory but usually improves performance due to a lower number of memory accesses.
/// It can result in about 90% memory reduction. The exception are highly irregular timestamps or values, which occur rarely.
///
/// When not specified, the option is set to `Compressed`.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TsEncoding {
    /// applies compression to the series samples.
    Compressed,
    /// keeps the raw samples in memory.
    ///
    /// Adding this flag keeps data in an uncompressed form.
    Uncompressed,
}

/// [`Policy`](https://redis.io/docs/stack/timeseries/configuration/#duplicate_policy)
/// for handling samples with identical timestamps
///
///  It is used with one of the following values:
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TsDuplicatePolicy {
    /// ignore any newly reported value and reply with an error
    Block,
    /// ignore any newly reported value
    First,
    /// override with the newly reported value
    Last,
    /// only override if the value is lower than the existing value
    Min,
    /// only override if the value is higher than the existing value
    Max,
    /// If a previous sample exists, add the new sample to it so that the updated value is equal to (previous + new).     ///
    /// If no previous sample exists, set the updated value equal to the new value.
    Sum,
}

/// Options for the [`ts_add`](TimeSeriesCommands::ts_create) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsCreateOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    retention: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<TsEncoding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate_policy: Option<TsDuplicatePolicy>,
    #[serde(skip_serializing_if = "SmallVec::is_empty")]
    labels: SmallVec<[(&'a str, &'a str); 10]>,
}

impl<'a> TsCreateOptions<'a> {
    /// maximum age for samples compared to the highest reported timestamp, in milliseconds.
    ///
    /// Samples are expired based solely on the difference between their timestamp
    /// and the timestamps passed to subsequent [`ts_add`](TimeSeriesCommands::ts_add),
    /// [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    /// and [`ts_decrby`](TimeSeriesCommands::ts_decrby) calls.
    ///
    /// When set to 0, samples never expire. When not specified, the option is set to the global
    /// [`RETENTION_POLICY`](https://redis.io/docs/stack/timeseries/configuration/#retention_policy)
    /// configuration of the database, which by default is 0.
    #[must_use]
    pub fn retention(mut self, retention_period: u64) -> Self {
        self.retention = Some(retention_period);
        self
    }

    /// specifies the series sample's encoding format.
    #[must_use]
    pub fn encoding(mut self, encoding: TsEncoding) -> Self {
        self.encoding = Some(encoding);
        self
    }

    /// initial allocation size, in bytes, for the data part of each new chunk. Actual chunks may consume more memory.
    ///
    /// Changing chunkSize (using [`ts_alter`](TimeSeriesCommands::ts_alter)) does not affect existing chunks.
    ///
    /// Must be a multiple of 8 in the range [48 .. 1048576].
    /// When not specified, it is set to 4096 bytes (a single memory page).
    ///
    /// Note: Before v1.6.10 no minimum was enforced. Between v1.6.10 and v1.6.17 and in v1.8.0 the minimum value was 128.
    /// Since v1.8.1 the minimum value is 48.
    ///
    /// The data in each key is stored in chunks. Each chunk contains header and data for a given timeframe.
    /// An index contains all chunks. Iterations occur inside each chunk. Depending on your use case, consider these tradeoffs for having smaller or larger sizes of chunks:
    /// * Insert performance: Smaller chunks result in slower inserts (more chunks need to be created).
    /// * Query performance: Queries for a small subset when the chunks are very large are slower,
    ///   as we need to iterate over the chunk to find the data.
    /// * Larger chunks may take more memory when you have a very large number of keys and very few samples per key,
    ///   or less memory when you have many samples per key.
    ///
    /// If you are unsure about your use case, select the default.
    #[must_use]
    pub fn chunk_size(mut self, chunk_size: u32) -> Self {
        self.chunk_size = Some(chunk_size);
        self
    }

    /// policy for handling insertion ([`ts_add`](TimeSeriesCommands::ts_add) and [`ts_madd`](TimeSeriesCommands::ts_madd))
    /// of multiple samples with identical timestamps
    #[must_use]
    pub fn duplicate_policy(mut self, policy: TsDuplicatePolicy) -> Self {
        self.duplicate_policy = Some(policy);
        self
    }

    /// set of label-value pairs that represent metadata labels of the time series.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`labels`](TsCreateOptions::labels).
    ///
    /// The [`ts_mget`](TimeSeriesCommands::ts_mget), [`ts_mrange`](TimeSeriesCommands::ts_mrange),
    /// and [`ts_mrevrange`](TimeSeriesCommands::ts_mrevrange) commands operate on multiple time series based on their labels.
    /// The [`ts_queryindex`](TimeSeriesCommands::ts_queryindex) command returns all time series keys matching a given filter based on their labels.
    #[must_use]
    pub fn labels(mut self, labels: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        self.labels.extend(labels);
        self
    }
}

/// Aggregation type for the [`ts_createrule`](TimeSeriesCommands::ts_createrule)
/// and [`ts_mrange`](TimeSeriesCommands::ts_mrange) commands.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TsAggregationType {
    /// Arithmetic mean of all values
    Avg,
    /// Sum of all values
    Sum,
    /// Minimum value
    Min,
    /// Maximum value
    Max,
    /// Difference between the highest and the lowest value
    Range,
    /// Number of values
    Count,
    /// Value with lowest timestamp in the bucket
    First,
    /// Value with highest timestamp in the bucket
    Last,
    /// Population standard deviation of the values
    #[serde(rename = "STD.P")]
    StdP,
    /// Sample standard deviation of the values
    #[serde(rename = "STD.S")]
    StdS,
    /// Population variance of the values
    #[serde(rename = "VAR.P")]
    VarP,
    /// Sample variance of the values
    #[serde(rename = "VAR.S")]
    VarS,
    /// Time-weighted average over the bucket's timeframe (since RedisTimeSeries v1.8)
    Twa,
}

/// Options for the [`ts_createrule`](TimeSeriesCommands::ts_createrule) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsCreateRuleOptions {
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    align_timestamp: Option<u64>,
}

impl TsCreateRuleOptions {
    /// ensures that there is a bucket that starts exactly at `align_timestamp`
    /// and aligns all other buckets accordingly. (since RedisTimeSeries v1.8)
    ///
    /// It is expressed in milliseconds.
    /// The default value is 0 aligned with the epoch.
    /// For example, if `bucket_duration` is 24 hours (`24 * 3600 * 1000`), setting `align_timestamp`
    /// to 6 hours after the epoch (`6 * 3600 * 1000`) ensures that each bucket’s timeframe is `[06:00 .. 06:00)`.
    #[must_use]
    pub fn align_timestamp(mut self, align_timestamp: u64) -> Self {
        self.align_timestamp = Some(align_timestamp);
        self
    }
}

/// Options for the [`ts_incrby`](TimeSeriesCommands::ts_incrby)
/// and [`ts_decrby`](TimeSeriesCommands::ts_decrby) commands.
///
/// # Notes
/// * You can use this command to add data to a nonexisting time series in a single command.
///   This is why `retention`, `uncompressed`, `chunk_size`, and `labels` are optional arguments.
/// * When specified and the key doesn't exist, a new time series is created.
///   Setting the `retention` and `labels` options introduces additional time complexity.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsIncrByDecrByOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<TsTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retention: Option<u64>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    uncompressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_size: Option<u32>,
    #[serde(skip_serializing_if = "SmallVec::is_empty")]
    labels: SmallVec<[(&'a str, &'a str); 10]>,
}

impl<'a> TsIncrByDecrByOptions<'a> {
    /// is (integer) UNIX sample timestamp in milliseconds or * to set the timestamp according to the server clock.
    ///
    /// timestamp must be equal to or higher than the maximum existing timestamp.
    /// When equal, the value of the sample with the maximum existing timestamp is decreased.
    /// If it is higher, a new sample with a timestamp set to timestamp is created,
    /// and its value is set to the value of the sample with the maximum existing timestamp minus value.
    ///
    /// If the time series is empty, the value is set to value.
    ///
    /// When not specified, the timestamp is set according to the server clock.
    #[must_use]
    pub fn timestamp(mut self, timestamp: TsTimestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// maximum age for samples compared to the highest reported timestamp, in milliseconds.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series
    ///
    /// See [`retention`](TsCreateOptions::retention).
    #[must_use]
    pub fn retention(mut self, retention_period: u64) -> Self {
        self.retention = Some(retention_period);
        self
    }

    /// changes data storage from compressed (default) to uncompressed.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`encoding`](TsCreateOptions::encoding).
    #[must_use]
    pub fn uncompressed(mut self) -> Self {
        self.uncompressed = true;
        self
    }

    /// memory size, in bytes, allocated for each data chunk.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`chunk_size`](TsCreateOptions::chunk_size).
    #[must_use]
    pub fn chunk_size(mut self, chunk_size: u32) -> Self {
        self.chunk_size = Some(chunk_size);
        self
    }

    /// set of label-value pairs that represent metadata labels of the time series.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`labels`](TsCreateOptions::labels).
    #[must_use]
    pub fn labels(mut self, label: &'a str, value: &'a str) -> Self {
        self.labels.push((label, value));
        self
    }
}

/// Options for the [`ts_get`](TimeSeriesCommands::ts_get) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsGetOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    latest: bool,
}

impl TsGetOptions {
    /// Used when a time series is a compaction.
    ///
    /// With `latest`, [`ts_get`](TimeSeriesCommands::ts_get)
    /// also reports the compacted value of the latest possibly partial bucket,
    /// given that this bucket's start time falls within [`from_timestamp`, `to_timestamp`].
    /// Without `latest`, [`ts_get`](TimeSeriesCommands::ts_get)
    ///  does not report the latest possibly partial bucket.
    /// When a time series is not a compaction, `latest` is ignored.
    ///
    /// The data in the latest bucket of a compaction is possibly partial.
    /// A bucket is closed and compacted only upon arrival of a new sample that opens a new latest bucket.
    /// There are cases, however, when the compacted value of the latest possibly partial bucket is also required.
    /// In such a case, use `latest`.
    #[must_use]
    pub fn latest(mut self) -> Self {
        self.latest = true;
        self
    }
}

/// Result for the [`ts_info`](TimeSeriesCommands::ts_info) command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsInfoResult {
    /// key name
    pub key_self_name: String,
    /// Total number of samples in this time series
    pub total_samples: usize,
    /// Total number of bytes allocated for this time series, which is the sum of
    /// * The memory used for storing the series' configuration parameters (retention period, duplication policy, etc.)
    /// * The memory used for storing the series' compaction rules
    /// * The memory used for storing the series' labels (key-value pairs)
    /// * The memory used for storing the chunks (chunk header + compressed/uncompressed data)
    pub memory_usage: usize,
    ///First timestamp present in this time series
    pub first_timestamp: u64,
    /// Last timestamp present in this time series
    pub last_timestamp: u64,
    /// The retention period, in milliseconds, for this time series
    pub retention_time: u64,
    /// Number of chunks used for this time series
    pub chunk_count: usize,
    /// The initial allocation size, in bytes, for the data part of each new chunk.
    /// Actual chunks may consume more memory.
    /// Changing the chunk size (using [`ts_alter`](TimeSeriesCommands::ts_alter)) does not affect existing chunks.
    pub chunk_size: usize,
    /// The chunks type: `compressed` or `uncompressed`
    pub chunk_type: String,
    /// The [`duplicate policy`](https://redis.io/docs/stack/timeseries/configuration/#duplicate_policy) of this time series
    pub duplicate_policy: Option<TsDuplicatePolicy>,
    /// A map of label-value pairs that represent the metadata labels of this time series
    pub labels: HashMap<String, String>,
    /// Key name for source time series in case the current series is a target
    ///  of a [`compaction rule`](https://redis.io/commands/ts.createrule/)
    pub source_key: String,
    /// A nested array of the [`compaction rules`](https://redis.io/commands/ts.createrule/)
    /// defined in this time series, with these elements for each rule:
    /// * The compaction key
    /// * The bucket duration
    /// * The aggregator
    /// * The alignment (since RedisTimeSeries v1.8)
    #[serde(deserialize_with = "deserialize_compation_rules")]
    pub rules: Vec<TsCompactionRule>,
    /// Additional chunk information when the `debug` flag is specified in [`ts_info`](TimeSeriesCommands::ts_info)
    #[serde(rename = "Chunks")]
    pub chunks: Option<Vec<TsInfoChunkResult>>,
    /// Additional values for future versions of the command
    #[serde(flatten)]
    pub additional_values: HashMap<String, Value>,
}

/// Additional debug result for the [`ts_info`](TimeSeriesCommands::ts_info) command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsInfoChunkResult {
    /// First timestamp present in the chunk
    pub start_timestamp: i64,
    /// Last timestamp present in the chunk
    pub end_timestamp: i64,
    /// Total number of samples in the chunk
    pub samples: usize,
    /// The chunk data size in bytes.
    /// This is the exact size that used for data only inside the chunk.
    /// It does not include other overheads.
    pub size: usize,
    /// Ratio of `size` and `samples`
    pub bytes_per_sample: f64,
}

/// information about the [`compaction rules`](https://redis.io/commands/ts.createrule/)
/// of a time series collection, in the context of the [`ts_info`](TimeSeriesCommands::ts_info) command.
#[derive(Debug)]
pub struct TsCompactionRule {
    /// The compaction key
    pub compaction_key: String,
    /// The bucket duration
    pub bucket_duration: u64,
    /// The aggregator
    pub aggregator: TsAggregationType,
    /// The alignment (since RedisTimeSeries v1.8)
    pub alignment: u64,
}

fn deserialize_compation_rules<'de, D>(deserializer: D) -> Result<Vec<TsCompactionRule>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<TsCompactionRule>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array of TsCompactionRule")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut rules = Vec::with_capacity(map.size_hint().unwrap_or_default());
            while let Some(compaction_key) = map.next_key()? {
                let (bucket_duration, aggregator, alignment) =
                    map.next_value::<(u64, TsAggregationType, u64)>()?;
                rules.push(TsCompactionRule {
                    compaction_key,
                    bucket_duration,
                    aggregator,
                    alignment,
                });
            }

            Ok(rules)
        }
    }

    deserializer.deserialize_map(Visitor)
}

// impl<'de> de::Deserialize<'de> for Vec<TsCompactionRule> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: de::Deserializer<'de> {
//         struct Visitor;

//         impl<'de> de::Visitor<'de> for Visitor {
//             type Value = TsCompactionRule;

//             fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//                 formatter.write_str("TsCompactionRule")
//             }

//             fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//                 where
//                     A: de::MapAccess<'de>, {
//                 let Some(entry)
//             }
//         }

//         deserializer.deserialize_map(Visitor)
//     }
// }

/// Options for the [`ts_mget`](TimeSeriesCommands::ts_mget) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsMGetOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    latest: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withlabels: bool,
    #[serde(skip_serializing_if = "SmallVec::is_empty")]
    selected_labels: SmallVec<[&'a str; 10]>,
}

impl<'a> TsMGetOptions<'a> {
    /// Used when a time series is a compaction.
    ///
    /// With `latest`, [`ts_mget`](TimeSeriesCommands::ts_mget)
    /// also reports the compacted value of the latest possibly partial bucket,
    /// given that this bucket's start time falls within [`from_timestamp`, `to_timestamp`].
    /// Without `latest`, [`ts_mget`](TimeSeriesCommands::ts_mget)
    ///  does not report the latest possibly partial bucket.
    /// When a time series is not a compaction, `latest` is ignored.
    ///
    /// The data in the latest bucket of a compaction is possibly partial.
    /// A bucket is closed and compacted only upon arrival of a new sample that opens a new latest bucket.
    /// There are cases, however, when the compacted value of the latest possibly partial bucket is also required.
    /// In such a case, use `latest`.
    #[must_use]
    pub fn latest(mut self) -> Self {
        self.latest = true;
        self
    }

    /// Includes in the reply all label-value pairs representing metadata labels of the time series.
    ///
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn withlabels(mut self) -> Self {
        self.withlabels = true;
        self
    }

    /// returns a subset of the label-value pairs that represent metadata labels of the time series.
    ///
    /// Use when a large number of labels exists per series, but only the values of some of the labels are required.
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn selected_label(mut self, label: &'a str) -> Self {
        self.selected_labels.push(label);
        self
    }
}

/// Result for the [`ts_mget`](TimeSeriesCommands::ts_mget) command.
#[derive(Debug, Deserialize)]
pub struct TsSample {
    /// Label-value pairs
    ///
    /// * By default, an empty list is reported
    /// * If [`withlabels`](TsMGetOptions::withlabels) is specified, all labels associated with this time series are reported
    /// * If [`selected_labels`](TsMGetOptions::selected_labels) is specified, the selected labels are reported
    pub labels: HashMap<String, String>,
    /// Timestamp-value pairs for all samples/aggregations matching the range
    pub timestamp_value: (u64, f64),
}

/// Options for the [`ts_mrange`](TimeSeriesCommands::ts_mrange) and
/// [`ts_mrevrange`](TimeSeriesCommands::ts_mrevrange) commands.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsMRangeOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    latest: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_by_ts: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_by_value: Option<(f64, f64)>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withlabels: bool,
    #[serde(skip_serializing_if = "SmallVec::is_empty")]
    selected_labels: SmallVec<[&'a str; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    align: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aggregation: Option<(TsAggregationType, u64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    buckettimestamp: Option<u64>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    empty: bool,
}

impl<'a> TsMRangeOptions<'a> {
    /// Used when a time series is a compaction.
    ///
    /// With `latest`, [`ts_mrange`](TimeSeriesCommands::ts_mrange)
    /// also reports the compacted value of the latest possibly partial bucket,
    /// given that this bucket's start time falls within [`from_timestamp`, `to_timestamp`].
    /// Without `latest`, [`ts_mrange`](TimeSeriesCommands::ts_mrange)
    /// does not report the latest possibly partial bucket.
    /// When a time series is not a compaction, `latest` is ignored.
    ///
    /// The data in the latest bucket of a compaction is possibly partial.
    /// A bucket is closed and compacted only upon arrival of a new sample that opens a new latest bucket.
    /// There are cases, however, when the compacted value of the latest possibly partial bucket is also required.
    /// In such a case, use `latest`.
    #[must_use]
    pub fn latest(mut self) -> Self {
        self.latest = true;
        self
    }

    /// filters samples by a list of specific timestamps.
    ///
    /// A sample passes the filter if its exact timestamp is specified and falls within [`from_timestamp`, `to_timestamp`].
    #[must_use]
    pub fn filter_by_ts(mut self, ts: &'a str) -> Self {
        self.filter_by_ts = Some(ts);
        self
    }

    /// filters samples by minimum and maximum values.
    #[must_use]
    pub fn filter_by_value(mut self, min: f64, max: f64) -> Self {
        self.filter_by_value = Some((min, max));
        self
    }

    /// Includes in the reply all label-value pairs representing metadata labels of the time series.
    ///
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn withlabels(mut self) -> Self {
        self.withlabels = true;
        self
    }

    /// returns a subset of the label-value pairs that represent metadata labels of the time series.
    ///
    /// Use when a large number of labels exists per series, but only the values of some of the labels are required.
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn selected_label(mut self, label: &'a str) -> Self {
        self.selected_labels.push(label);
        self
    }

    /// limits the number of returned samples.
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// A time bucket alignment control for `aggregation`.
    ///
    /// It controls the time bucket timestamps by changing the reference timestamp on which a bucket is defined.
    ///
    /// Values include:
    /// * `start` or `-`: The reference timestamp will be the query start interval time (`from_timestamp`) which can't be `-`
    /// * `end` or `+`: The reference timestamp will be the query end interval time (`to_timestamp`) which can't be `+`
    /// * A specific timestamp: align the reference timestamp to a specific time
    ///
    /// # Note
    /// When not provided, alignment is set to 0.
    #[must_use]
    pub fn align(mut self, align: &'a str) -> Self {
        self.align = Some(align);
        self
    }

    /// Aggregates results into time buckets, where:
    /// * `aggregator` - takes a value of [`TsAggregationType`](TsAggregationType)
    /// * `bucket_duration` - is duration of each bucket, in milliseconds.
    ///
    /// Without `align`, bucket start times are multiples of `bucket_duration`.
    ///
    /// With `align`, bucket start times are multiples of `bucket_duration` with remainder `align` % `bucket_duration`.
    ///
    /// The first bucket start time is less than or equal to `from_timestamp`.
    #[must_use]
    pub fn aggregation(mut self, aggregator: TsAggregationType, bucket_duration: u64) -> Self {
        self.aggregation = Some((aggregator, bucket_duration));
        self
    }

    /// controls how bucket timestamps are reported.
    /// `bucket_timestamp` values include:
    /// * `-` or `low` - Timestamp reported for each bucket is the bucket's start time (default)
    /// * `+` or `high` - Timestamp reported for each bucket is the bucket's end time
    /// * `~` or `mid` - Timestamp reported for each bucket is the bucket's mid time (rounded down if not an integer)
    #[must_use]
    pub fn bucket_timestamp(mut self, bucket_timestamp: u64) -> Self {
        self.buckettimestamp = Some(bucket_timestamp);
        self
    }

    /// A flag, which, when specified, reports aggregations also for empty buckets.
    /// when `aggregator` values are:
    /// * `sum`, `count` - the value reported for each empty bucket is `0`
    /// * `last` - the value reported for each empty bucket is the value
    ///   of the last sample before the bucket's start.
    ///   `NaN` when no such sample.
    /// * `twa` - the value reported for each empty bucket is the average value
    ///   over the bucket's timeframe based on linear interpolation
    ///   of the last sample before the bucket's start and the first sample after the bucket's end.
    ///   `NaN` when no such samples.
    /// * `min`, `max`, `range`, `avg`, `first`, `std.p`, `std.s` - the value reported for each empty bucket is `NaN`
    ///
    /// Regardless of the values of `from_timestamp` and `to_timestamp`,
    /// no data is reported for buckets that end before the earliest sample or begin after the latest sample in the time series.
    #[must_use]
    pub fn empty(mut self) -> Self {
        self.empty = true;
        self
    }
}

/// Result for the [`ts_mrange`](TimeSeriesCommands::ts_mrange) and
/// [`ts_mrevrange`](TimeSeriesCommands::ts_mrevrange) commands.
#[derive(Debug)]
pub struct TsRangeSample {
    /// Label-value pairs
    ///
    /// * By default, an empty list is reported
    /// * If [`withlabels`](TsMGetOptions::withlabels) is specified, all labels associated with this time series are reported
    /// * If [`selected_labels`](TsMGetOptions::selected_labels) is specified, the selected labels are reported
    pub labels: Vec<(String, String)>,
    pub reducers: Vec<String>,
    pub sources: Vec<String>,
    pub aggregators: Vec<String>,
    /// Timestamp-value pairs for all samples/aggregations matching the range
    pub values: Vec<(u64, f64)>,
}

impl<'de> de::Deserialize<'de> for TsRangeSample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        enum TsRangeSampleField {
            Aggregators(Vec<String>),
            Reducers(Vec<String>),
            Sources(Vec<String>),
            Values(Vec<(u64, f64)>),
        }

        impl<'de> de::Deserialize<'de> for TsRangeSampleField {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct Visitor;

                impl<'de> de::Visitor<'de> for Visitor {
                    type Value = TsRangeSampleField;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("TsRangeSampleField")
                    }

                    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::SeqAccess<'de>,
                    {
                        Ok(TsRangeSampleField::Values(Vec::<(u64, f64)>::deserialize(
                            SeqAccessDeserializer::new(seq),
                        )?))
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::MapAccess<'de>,
                    {
                        let (Some((field, value)), None) = (
                            map.next_entry::<&str, Vec<String>>()?,
                            map.next_entry::<&str, Vec<String>>()?,
                        ) else {
                            return Err(de::Error::invalid_length(0, &"1 in map"));
                        };

                        match field {
                            "reducers" => Ok(TsRangeSampleField::Reducers(value)),
                            "sources" => Ok(TsRangeSampleField::Sources(value)),
                            "aggregators" => Ok(TsRangeSampleField::Aggregators(value)),
                            _ => Err(de::Error::unknown_field(
                                field,
                                &["reducers", "sources", "aggregators"],
                            )),
                        }
                    }
                }

                deserializer.deserialize_any(Visitor)
            }
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = TsRangeSample;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("TsRangeSample")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut sample = TsRangeSample {
                    labels: Vec::new(),
                    reducers: Vec::new(),
                    sources: Vec::new(),
                    aggregators: Vec::new(),
                    values: Vec::new(),
                };

                let Some(labels) = seq.next_element::<Vec<(String, String)>>()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                sample.labels = labels;

                while let Some(field) = seq.next_element::<TsRangeSampleField>()? {
                    match field {
                        TsRangeSampleField::Aggregators(aggregators) => {
                            sample.aggregators = aggregators
                        }
                        TsRangeSampleField::Reducers(reducers) => sample.reducers = reducers,
                        TsRangeSampleField::Sources(sources) => sample.sources = sources,
                        TsRangeSampleField::Values(values) => sample.values = values,
                    }
                }

                Ok(sample)
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

/// Options for the [`ts_mrange`](TimeSeriesCommands::ts_mrange) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsGroupByOptions<'a> {
    groupby: &'a str,
    reduce: TsAggregationType,
}

impl<'a> TsGroupByOptions<'a> {
    /// aggregates results across different time series, grouped by the provided label name.
    ///
    /// When combined with [`aggregation`](TsMRangeOptions::aggregation) the groupby/reduce is applied post aggregation stage.
    ///
    /// # Arguments
    /// * `label` - is the label name to group a series by. A new series for each value is produced.
    /// * `reducer` - is the reducer type used to aggregate series that share the same label value.
    ///
    /// # Notes
    /// * The produced time series is named `<label>=<groupbyvalue>`
    /// * The produced time series contains two labels with these label array structures:
    ///   * `reducer`, the reducer used
    ///   * `source`, the time series keys used to compute the grouped series (key1,key2,key3,...)
    #[must_use]
    pub fn new(label: &'a str, reducer: TsAggregationType) -> Self {
        Self {
            groupby: label,
            reduce: reducer,
        }
    }
}

/// Options for the [`ts_range`](TimeSeriesCommands::ts_range) and
/// [`ts_revrange`](TimeSeriesCommands::ts_revrange) commands.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct TsRangeOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    latest: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_by_ts: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_by_value: Option<(f64, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    align: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aggregation: Option<(TsAggregationType, u64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    buckettimestamp: Option<u64>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    empty: bool,
}

impl<'a> TsRangeOptions<'a> {
    /// Used when a time series is a compaction.
    ///
    /// With `latest`, [`ts_range`](TimeSeriesCommands::ts_range)
    /// also reports the compacted value of the latest possibly partial bucket,
    /// given that this bucket's start time falls within [`from_timestamp`, `to_timestamp`].
    /// Without `latest`, [`ts_range`](TimeSeriesCommands::ts_range)
    /// does not report the latest possibly partial bucket.
    /// When a time series is not a compaction, `latest` is ignored.
    ///
    /// The data in the latest bucket of a compaction is possibly partial.
    /// A bucket is closed and compacted only upon arrival of a new sample that opens a new latest bucket.
    /// There are cases, however, when the compacted value of the latest possibly partial bucket is also required.
    /// In such a case, use `latest`.
    #[must_use]
    pub fn latest(mut self) -> Self {
        self.latest = true;
        self
    }

    /// filters samples by a list of specific timestamps.
    ///
    /// A sample passes the filter if its exact timestamp is specified and falls within [`from_timestamp`, `to_timestamp`].
    #[must_use]
    pub fn filter_by_ts(mut self, ts: &'a str) -> Self {
        self.filter_by_ts = Some(ts);
        self
    }

    /// filters samples by minimum and maximum values.
    #[must_use]
    pub fn filter_by_value(mut self, min: f64, max: f64) -> Self {
        self.filter_by_value = Some((min, max));
        self
    }

    /// limits the number of returned samples.
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// A time bucket alignment control for `aggregation`.
    ///
    /// It controls the time bucket timestamps by changing the reference timestamp on which a bucket is defined.
    ///
    /// Values include:
    /// * `start` or `-`: The reference timestamp will be the query start interval time (`from_timestamp`) which can't be `-`
    /// * `end` or `+`: The reference timestamp will be the query end interval time (`to_timestamp`) which can't be `+`
    /// * A specific timestamp: align the reference timestamp to a specific time
    ///
    /// # Note
    /// When not provided, alignment is set to 0.
    #[must_use]
    pub fn align(mut self, align: &'a str) -> Self {
        self.align = Some(align);
        self
    }

    /// Aggregates results into time buckets, where:
    /// * `aggregator` - takes a value of [`TsAggregationType`](TsAggregationType)
    /// * `bucket_duration` - is duration of each bucket, in milliseconds.
    ///
    /// Without `align`, bucket start times are multiples of `bucket_duration`.
    ///
    /// With `align`, bucket start times are multiples of `bucket_duration` with remainder `align` % `bucket_duration`.
    ///
    /// The first bucket start time is less than or equal to `from_timestamp`.
    #[must_use]
    pub fn aggregation(mut self, aggregator: TsAggregationType, bucket_duration: u64) -> Self {
        self.aggregation = Some((aggregator, bucket_duration));
        self
    }

    /// controls how bucket timestamps are reported.
    /// `bucket_timestamp` values include:
    /// * `-` or `low` - Timestamp reported for each bucket is the bucket's start time (default)
    /// * `+` or `high` - Timestamp reported for each bucket is the bucket's end time
    /// * `~` or `mid` - Timestamp reported for each bucket is the bucket's mid time (rounded down if not an integer)
    #[must_use]
    pub fn bucket_timestamp(mut self, bucket_timestamp: u64) -> Self {
        self.buckettimestamp = Some(bucket_timestamp);
        self
    }

    /// A flag, which, when specified, reports aggregations also for empty buckets.
    /// when `aggregator` values are:
    /// * `sum`, `count` - the value reported for each empty bucket is `0`
    /// * `last` - the value reported for each empty bucket is the value
    ///   of the last sample before the bucket's start.
    ///   `NaN` when no such sample.
    /// * `twa` - the value reported for each empty bucket is the average value
    ///   over the bucket's timeframe based on linear interpolation
    ///   of the last sample before the bucket's start and the first sample after the bucket's end.
    ///   `NaN` when no such samples.
    /// * `min`, `max`, `range`, `avg`, `first`, `std.p`, `std.s` - the value reported for each empty bucket is `NaN`
    ///
    /// Regardless of the values of `from_timestamp` and `to_timestamp`,
    /// no data is reported for buckets that end before the earliest sample or begin after the latest sample in the time series.
    #[must_use]
    pub fn empty(mut self) -> Self {
        self.empty = true;
        self
    }
}

/// Timeseries Timestamp
pub enum TsTimestamp {
    /// User specified timestamp
    Value(u64),
    /// Unix time of the server clock (*)
    ServerClock,
}

impl Serialize for TsTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TsTimestamp::Value(ts) => serializer.serialize_u64(*ts),
            TsTimestamp::ServerClock => serializer.serialize_str("*"),
        }
    }
}
