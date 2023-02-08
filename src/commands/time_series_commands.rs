use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CommandArgs, PrimitiveResponse, CollectionResponse, IntoArgs,
        KeyValueArgsCollection, MultipleArgsCollection, SingleArg, SingleArgCollection, Value,
    },
};
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::HashMap;

/// A group of Redis commands related to [`Time Series`](https://redis.io/docs/stack/timeseries/)
///
/// # See Also
/// [Time Series Commands](https://redis.io/commands/?group=timeseries)
pub trait TimeSeriesCommands {
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
        &mut self,
        key: impl SingleArg,
        timestamp: impl SingleArg,
        value: f64,
        options: TsAddOptions,
    ) -> PreparedCommand<Self, u64>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TS.ADD")
                .arg(key)
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
        &mut self,
        key: impl SingleArg,
        options: TsCreateOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.ALTER").arg(key).arg(options))
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
    /// [`ts_add`](TimeSeriesCommands::ts_add), [`ts_incrby`](TimeSeriesCommands::ts_incrby), and [`ts_decrby`](TimeSeriesCommands::ts_decrby).
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.create/>](https://redis.io/commands/ts.create/)
    #[must_use]
    fn ts_create(
        &mut self,
        key: impl SingleArg,
        options: TsCreateOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.CREATE").arg(key).arg(options))
    }

    /// Create a compaction rule
    ///
    /// # Arguments
    /// * `src_key` - key name for the source time series.
    /// * `dst_key` - key name for destination (compacted) time series.
    ///  It must be created before `ts_createrule` is called.
    /// * `aggregator` - aggregates results into time buckets by taking an aggregation type
    /// * `bucket_duration` - duration of each aggregation bucket, in milliseconds.
    /// * `options` - See [`TsCreateRuleOptions`](TsCreateRuleOptions)
    ///
    /// # Notes
    /// * Only new samples that are added into the source series after the creation of the rule will be aggregated.
    /// * Calling `ts_createrule` with a nonempty `dst_key` may result in inconsistencies between the raw and the compacted data.
    /// * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add),
    ///  [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby), or [`ts_decrby`](TimeSeriesCommands::ts_decrby))
    /// may result in inconsistencies between the raw and the compacted data. The compaction process may override such samples.
    /// * If no samples are added to the source time series during a bucket period. no compacted sample is added to the destination time series.
    /// * The timestamp of a compacted sample added to the destination time series is set to the start timestamp the appropriate compaction bucket.
    /// For example, for a 10-minute compaction bucket with no alignment, the compacted samples timestamps are `x:00`, `x:10`, `x:20`, and so on.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.createrule/>](https://redis.io/commands/ts.createrule/)
    #[must_use]
    fn ts_createrule(
        &mut self,
        src_key: impl SingleArg,
        dst_key: impl SingleArg,
        aggregator: TsAggregationType,
        bucket_duration: u64,
        options: TsCreateRuleOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TS.CREATERULE")
                .arg(src_key)
                .arg(dst_key)
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
    ///  [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    /// or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result in inconsistencies between the raw and the compacted data.
    /// The compaction process may override such samples.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.decrby/>](https://redis.io/commands/ts.decrby/)
    #[must_use]
    fn ts_decrby(
        &mut self,
        key: impl SingleArg,
        value: f64,
        options: TsIncrByDecrByOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.DECRBY").arg(key).arg(value).arg(options))
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
        &mut self,
        key: impl SingleArg,
        from_timestamp: u64,
        to_timestamp: u64,
    ) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TS.DEL").arg(key).arg(from_timestamp).arg(to_timestamp),
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
        &mut self,
        src_key: impl SingleArg,
        dst_key: impl SingleArg,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.DELETERULE").arg(src_key).arg(dst_key))
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
        &mut self,
        key: impl SingleArg,
        options: TsGetOptions,
    ) -> PreparedCommand<Self, Option<(u64, f64)>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.GET").arg(key).arg(options))
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
    ///  [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    /// or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result in inconsistencies between the raw and the compacted data.
    /// The compaction process may override such samples.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.incrby/>](https://redis.io/commands/ts.incrby/)
    #[must_use]
    fn ts_incrby(
        &mut self,
        key: impl SingleArg,
        value: f64,
        options: TsIncrByDecrByOptions,
    ) -> PreparedCommand<Self, u64>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.INCRBY").arg(key).arg(value).arg(options))
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
    fn ts_info(&mut self, key: impl SingleArg, debug: bool) -> PreparedCommand<Self, TsInfoResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.INFO").arg(key).arg_if(debug, "DEBUG"))
    }

    /// Append new samples to one or more time series
    ///
    /// # Arguments
    /// * `items` - one or more the following tuple:
    ///   * `key` - the key name for the time series.
    ///   * `timestamp` - the UNIX sample timestamp in milliseconds or * to set the timestamp according to the server clock.
    ///   * `value` - numeric data value of the sample (double). \
    /// The double number should follow [`RFC 7159`](https://tools.ietf.org/html/rfc7159) (a JSON standard).
    /// The parser rejects overly large values that would not fit in binary64.
    /// It does not accept `NaN` or `infinite` values.
    ///
    /// # Return
    /// a collection of the timestamps of added samples
    ///
    /// # Notes
    /// * If timestamp is older than the retention period compared to the maximum existing timestamp,
    /// the sample is discarded and an error is returned.
    /// * Explicitly adding samples to a compacted time series (using [`ts_add`](TimeSeriesCommands::ts_add),
    /// [`ts_madd`](TimeSeriesCommands::ts_madd), [`ts_incrby`](TimeSeriesCommands::ts_incrby),
    /// or [`ts_decrby`](TimeSeriesCommands::ts_decrby)) may result in inconsistencies between the raw and the compacted data.
    /// The compaction process may override such samples.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ts.madd/>](https://redis.io/commands/ts.madd/)
    #[must_use]
    fn ts_madd<K: SingleArg, T: SingleArg, R: CollectionResponse<u64>>(
        &mut self,
        items: impl MultipleArgsCollection<(K, T, f64)>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.MADD").arg(items))
    }

    /// Get the last samples matching a specific filter
    ///
    /// # Arguments
    /// * `key` - key name for the time series.
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
    fn ts_mget<F: SingleArg, R: CollectionResponse<TsSample>>(
        &mut self,
        options: TsMGetOptions,
        filters: impl SingleArgCollection<F>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.MGET").arg(options).arg("FILTER").arg(filters))
    }

    /// Query a range across multiple time series by filters in forward direction
    ///
    /// # Arguments
    /// * `from_timestamp` - start timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///    or `-` to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///    or `+` to denote the timestamp of the latest sample in the time series.
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
    fn ts_mrange<F: SingleArg, R: CollectionResponse<TsRangeSample>>(
        &mut self,
        from_timestamp: impl SingleArg,
        to_timestamp: impl SingleArg,
        options: TsMRangeOptions,
        filters: impl SingleArgCollection<F>,
        groupby_options: TsGroupByOptions,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
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
    ///    or `-` to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///    or `+` to denote the timestamp of the latest sample in the time series.
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
    fn ts_mrevrange<F: SingleArg, R: CollectionResponse<TsRangeSample>>(
        &mut self,
        from_timestamp: impl SingleArg,
        to_timestamp: impl SingleArg,
        options: TsMRangeOptions,
        filters: impl SingleArgCollection<F>,
        groupby_options: TsGroupByOptions,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
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
    fn ts_queryindex<F: SingleArg, R: PrimitiveResponse + DeserializeOwned, RR: CollectionResponse<R>>(
        &mut self,
        filters: impl SingleArgCollection<F>,
    ) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TS.QUERYINDEX").arg(filters))
    }

    /// Query a range in forward direction
    ///
    /// # Arguments
    /// * `key` - the key name for the time series.
    /// * `from_timestamp` - start timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///    or `-`to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///    or `+` to denote the timestamp of the latest sample in the time series.
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
    fn ts_range<R: CollectionResponse<(u64, f64)>>(
        &mut self,
        key: impl SingleArg,
        from_timestamp: impl SingleArg,
        to_timestamp: impl SingleArg,
        options: TsRangeOptions,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TS.RANGE")
                .arg(key)
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
    ///    or `-`to denote the timestamp of the earliest sample in the time series.
    /// * `to_timestamp` - end timestamp for the range query (integer UNIX timestamp in milliseconds)
    ///    or `+` to denote the timestamp of the latest sample in the time series.
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
    fn ts_revrange<R: CollectionResponse<(u64, f64)>>(
        &mut self,
        key: impl SingleArg,
        from_timestamp: impl SingleArg,
        to_timestamp: impl SingleArg,
        options: TsRangeOptions,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("TS.REVRANGE")
                .arg(key)
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
/// This is why [`retention`](TsAddOptions::retention), [`encoding`](TsAddOptions::encoding),
/// [`chunk_size`](TsAddOptions::chunk_size), [`on_duplicate`](TsAddOptions::on_duplicate),
/// and [`labels`](TsAddOptions::labels) are optional arguments.
/// * Setting [`retention`](TsAddOptions::retention) and [`labels`](TsAddOptions::labels) introduces additional time complexity.
#[derive(Default)]
pub struct TsAddOptions {
    command_args: CommandArgs,
}

impl TsAddOptions {
    /// maximum retention period, compared to the maximum existing timestamp, in milliseconds.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`retention`](TsCreateOptions::retention).
    #[must_use]
    pub fn retention(self, retention_period: u64) -> Self {
        Self {
            command_args: self.command_args.arg("RETENTION").arg(retention_period),
        }
    }

    /// specifies the series sample's encoding format.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`encoding`](TsCreateOptions::encoding).
    #[must_use]
    pub fn encoding(self, encoding: TsEncoding) -> Self {
        Self {
            command_args: self.command_args.arg("ENCODING").arg(encoding),
        }
    }

    /// memory size, in bytes, allocated for each data chunk.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`chunk_size`](TsCreateOptions::chunk_size).
    #[must_use]
    pub fn chunk_size(self, chunk_size: usize) -> Self {
        Self {
            command_args: self.command_args.arg("CHUNK_SIZE").arg(chunk_size),
        }
    }

    /// overwrite key and database configuration for
    /// [`DUPLICATE_POLICY`](https://redis.io/docs/stack/timeseries/configuration/#duplicate_policy)
    #[must_use]
    pub fn on_duplicate(self, policy: TsDuplicatePolicy) -> Self {
        Self {
            command_args: self.command_args.arg("ON_DUPLICATE").arg(policy),
        }
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
    pub fn labels<L: SingleArg, V: SingleArg, LL: KeyValueArgsCollection<L, V>>(
        self,
        labels: LL,
    ) -> Self {
        Self {
            command_args: self.command_args.arg("LABELS").arg(labels),
        }
    }
}

impl IntoArgs for TsAddOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// specifies the series samples encoding format.
///
/// `Compressed` is almost always the right choice.
/// Compression not only saves memory but usually improves performance due to a lower number of memory accesses.
/// It can result in about 90% memory reduction. The exception are highly irregular timestamps or values, which occur rarely.
///
/// When not specified, the option is set to `Compressed`.
pub enum TsEncoding {
    /// applies compression to the series samples.
    Compressed,
    /// keeps the raw samples in memory.
    ///
    /// Adding this flag keeps data in an uncompressed form.
    Uncompressed,
}

impl IntoArgs for TsEncoding {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            TsEncoding::Compressed => "COMPRESSED",
            TsEncoding::Uncompressed => "UNCOMPRESSED",
        })
    }
}

/// [`Policy`](https://redis.io/docs/stack/timeseries/configuration/#duplicate_policy)
/// for handling samples with identical timestamps
///
///  It is used with one of the following values:
#[derive(Debug, Deserialize)]
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

impl IntoArgs for TsDuplicatePolicy {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            TsDuplicatePolicy::Block => "BLOCK",
            TsDuplicatePolicy::First => "FIRST",
            TsDuplicatePolicy::Last => "LAST",
            TsDuplicatePolicy::Min => "MIN",
            TsDuplicatePolicy::Max => "MAX",
            TsDuplicatePolicy::Sum => "SUM",
        })
    }
}

/// Options for the [`ts_add`](TimeSeriesCommands::ts_create) command.
#[derive(Default)]
pub struct TsCreateOptions {
    command_args: CommandArgs,
}

impl TsCreateOptions {
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
    pub fn retention(self, retention_period: u64) -> Self {
        Self {
            command_args: self.command_args.arg("RETENTION").arg(retention_period),
        }
    }

    /// specifies the series sample's encoding format.
    #[must_use]
    pub fn encoding(self, encoding: TsEncoding) -> Self {
        Self {
            command_args: self.command_args.arg("ENCODING").arg(encoding),
        }
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
    pub fn chunk_size(self, chunk_size: usize) -> Self {
        Self {
            command_args: self.command_args.arg("CHUNK_SIZE").arg(chunk_size),
        }
    }

    /// policy for handling insertion ([`ts_add`](TimeSeriesCommands::ts_add) and [`ts_madd`](TimeSeriesCommands::ts_madd))
    /// of multiple samples with identical timestamps
    #[must_use]
    pub fn duplicate_policy(self, policy: TsDuplicatePolicy) -> Self {
        Self {
            command_args: self.command_args.arg("DUPLICATE_POLICY").arg(policy),
        }
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
    pub fn labels<L: SingleArg, V: SingleArg, LL: KeyValueArgsCollection<L, V>>(
        self,
        labels: LL,
    ) -> Self {
        Self {
            command_args: self.command_args.arg("LABELS").arg(labels),
        }
    }
}

impl IntoArgs for TsCreateOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Aggregation type for the [`ts_createrule`](TimeSeriesCommands::ts_createrule)
/// and [`ts_mrange`](TimeSeriesCommands::ts_mrange) commands.
#[derive(Debug, Deserialize)]
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
    #[serde(rename = "VAR.P")]
    VarS,
    /// Time-weighted average over the bucket's timeframe (since RedisTimeSeries v1.8)
    Twa,
}

impl IntoArgs for TsAggregationType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            TsAggregationType::Avg => "avg",
            TsAggregationType::Sum => "sum",
            TsAggregationType::Min => "min",
            TsAggregationType::Max => "max",
            TsAggregationType::Range => "range",
            TsAggregationType::Count => "count",
            TsAggregationType::First => "first",
            TsAggregationType::Last => "last",
            TsAggregationType::StdP => "std.p",
            TsAggregationType::StdS => "std.s",
            TsAggregationType::VarP => "var.p",
            TsAggregationType::VarS => "var.s",
            TsAggregationType::Twa => "twa",
        })
    }
}

/// Options for the [`ts_createrule`](TimeSeriesCommands::ts_createrule) command.
#[derive(Default)]
pub struct TsCreateRuleOptions {
    command_args: CommandArgs,
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
    pub fn align_timestamp(self, align_timestamp: u64) -> Self {
        Self {
            command_args: self.command_args.arg(align_timestamp),
        }
    }
}

impl IntoArgs for TsCreateRuleOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`ts_incrby`](TimeSeriesCommands::ts_incrby)
/// and [`ts_decrby`](TimeSeriesCommands::ts_decrby) commands.
///
/// # Notes
/// * You can use this command to add data to a nonexisting time series in a single command.
/// This is why `retention`, `uncompressed`, `chunk_size`, and `labels` are optional arguments.
/// * When specified and the key doesn't exist, a new time series is created.
/// Setting the `retention` and `labels` options introduces additional time complexity.
#[derive(Default)]
pub struct TsIncrByDecrByOptions {
    command_args: CommandArgs,
}

impl TsIncrByDecrByOptions {
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
    pub fn timestamp(self, timestamp: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("TIMESTAMP").arg(timestamp),
        }
    }

    /// maximum age for samples compared to the highest reported timestamp, in milliseconds.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series
    ///
    /// See [`retention`](TsCreateOptions::retention).
    #[must_use]
    pub fn retention(self, retention_period: u64) -> Self {
        Self {
            command_args: self.command_args.arg("RETENTION").arg(retention_period),
        }
    }

    /// changes data storage from compressed (default) to uncompressed.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`encoding`](TsCreateOptions::encoding).
    #[must_use]
    pub fn uncompressed(self) -> Self {
        Self {
            command_args: self.command_args.arg("UNCOMPRESSED"),
        }
    }

    /// memory size, in bytes, allocated for each data chunk.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`chunk_size`](TsCreateOptions::chunk_size).
    #[must_use]
    pub fn chunk_size(self, chunk_size: usize) -> Self {
        Self {
            command_args: self.command_args.arg("CHUNK_SIZE").arg(chunk_size),
        }
    }

    /// set of label-value pairs that represent metadata labels of the time series.
    ///
    /// Use it only if you are creating a new time series.
    /// It is ignored if you are adding samples to an existing time series.
    /// See [`labels`](TsCreateOptions::labels).
    #[must_use]
    pub fn labels<L: SingleArg, V: SingleArg, LL: KeyValueArgsCollection<L, V>>(
        self,
        labels: LL,
    ) -> Self {
        Self {
            command_args: self.command_args.arg("LABELS").arg(labels),
        }
    }
}

impl IntoArgs for TsIncrByDecrByOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`ts_get`](TimeSeriesCommands::ts_get) command.
#[derive(Default)]
pub struct TsGetOptions {
    command_args: CommandArgs,
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
    pub fn latest(self) -> Self {
        Self {
            command_args: self.command_args.arg("LATEST"),
        }
    }
}

impl IntoArgs for TsGetOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
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
#[derive(Debug, Deserialize)]
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

/// Options for the [`ts_mget`](TimeSeriesCommands::ts_mget) command.
#[derive(Default)]
pub struct TsMGetOptions {
    command_args: CommandArgs,
}

impl TsMGetOptions {
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
    pub fn latest(self) -> Self {
        Self {
            command_args: self.command_args.arg("LATEST"),
        }
    }

    /// Includes in the reply all label-value pairs representing metadata labels of the time series.
    ///
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn withlabels(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHLABELS"),
        }
    }

    /// returns a subset of the label-value pairs that represent metadata labels of the time series.
    ///
    /// Use when a large number of labels exists per series, but only the values of some of the labels are required.
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn selected_labels<L: SingleArg>(self, labels: impl SingleArgCollection<L>) -> Self {
        Self {
            command_args: self.command_args.arg("SELECTED_LABELS").arg(labels),
        }
    }
}

impl IntoArgs for TsMGetOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`ts_mget`](TimeSeriesCommands::ts_mget) command.
#[derive(Debug, Deserialize)]
pub struct TsSample {
    /// The key name
    pub key: String,
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
#[derive(Default)]
pub struct TsMRangeOptions {
    command_args: CommandArgs,
}

/// Result for the [`ts_mrange`](TimeSeriesCommands::ts_mrange) and
/// [`ts_mrevrange`](TimeSeriesCommands::ts_mrevrange) commands.
#[derive(Debug, Deserialize)]
pub struct TsRangeSample {
    /// The key name
    pub key: String,
    /// Label-value pairs
    ///
    /// * By default, an empty list is reported
    /// * If [`withlabels`](TsMGetOptions::withlabels) is specified, all labels associated with this time series are reported
    /// * If [`selected_labels`](TsMGetOptions::selected_labels) is specified, the selected labels are reported
    pub labels: HashMap<String, String>,
    /// Timestamp-value pairs for all samples/aggregations matching the range
    pub values: Vec<(u64, f64)>,
}

impl TsMRangeOptions {
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
    pub fn latest(self) -> Self {
        Self {
            command_args: self.command_args.arg("LATEST"),
        }
    }

    /// filters samples by a list of specific timestamps.
    ///
    /// A sample passes the filter if its exact timestamp is specified and falls within [`from_timestamp`, `to_timestamp`].
    #[must_use]
    pub fn filter_by_ts(self, ts: impl SingleArgCollection<u64>) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER_BY_TS").arg(ts),
        }
    }

    /// filters samples by minimum and maximum values.
    #[must_use]
    pub fn filter_by_value(self, min: f64, max: f64) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER_BY_VALUE").arg(min).arg(max),
        }
    }

    /// Includes in the reply all label-value pairs representing metadata labels of the time series.
    ///
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn withlabels(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHLABELS"),
        }
    }

    /// returns a subset of the label-value pairs that represent metadata labels of the time series.
    ///
    /// Use when a large number of labels exists per series, but only the values of some of the labels are required.
    /// If `withlabels` or `selected_labels` are not specified, by default, an empty list is reported as label-value pairs.
    #[must_use]
    pub fn selected_labels<L: SingleArg>(self, labels: impl SingleArgCollection<L>) -> Self {
        Self {
            command_args: self.command_args.arg("SELECTED_LABELS").arg(labels),
        }
    }

    /// limits the number of returned samples.
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
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
    pub fn align(self, align: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("ALIGN").arg(align),
        }
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
    pub fn aggregation(self, aggregator: TsAggregationType, bucket_duration: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("AGGREGATION")
                .arg(aggregator)
                .arg(bucket_duration),
        }
    }

    /// controls how bucket timestamps are reported.
    /// `bucket_timestamp` values include:
    /// * `-` or `low` - Timestamp reported for each bucket is the bucket's start time (default)
    /// * `+` or `high` - Timestamp reported for each bucket is the bucket's end time
    /// * `~` or `mid` - Timestamp reported for each bucket is the bucket's mid time (rounded down if not an integer)
    #[must_use]
    pub fn bucket_timestamp(self, bucket_timestamp: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("BUCKETTIMESTAMP")
                .arg(bucket_timestamp),
        }
    }

    /// A flag, which, when specified, reports aggregations also for empty buckets.
    /// when `aggregator` values are:
    /// * `sum`, `count` - the value reported for each empty bucket is `0`
    /// * `last` - the value reported for each empty bucket is the value
    ///    of the last sample before the bucket's start.
    ///    `NaN` when no such sample.
    /// * `twa` - the value reported for each empty bucket is the average value
    ///    over the bucket's timeframe based on linear interpolation
    ///    of the last sample before the bucket's start and the first sample after the bucket's end.
    ///    `NaN` when no such samples.
    /// * `min`, `max`, `range`, `avg`, `first`, `std.p`, `std.s` - the value reported for each empty bucket is `NaN`
    ///
    /// Regardless of the values of `from_timestamp` and `to_timestamp`,
    /// no data is reported for buckets that end before the earliest sample or begin after the latest sample in the time series.
    #[must_use]
    pub fn empty(self) -> Self {
        Self {
            command_args: self.command_args.arg("EMPTY"),
        }
    }
}

impl IntoArgs for TsMRangeOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`ts_mrange`](TimeSeriesCommands::ts_mrange) command.
pub struct TsGroupByOptions {
    command_args: CommandArgs,
}

impl TsGroupByOptions {
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
    pub fn new(label: impl SingleArg, reducer: TsAggregationType) -> Self {
        Self {
            command_args: CommandArgs::Empty
                .arg("GROUPBY")
                .arg(label)
                .arg("REDUCE")
                .arg(reducer),
        }
    }
}

impl IntoArgs for TsGroupByOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`ts_range`](TimeSeriesCommands::ts_range) and
/// [`ts_revrange`](TimeSeriesCommands::ts_revrange) commands.
#[derive(Default)]
pub struct TsRangeOptions {
    command_args: CommandArgs,
}

impl TsRangeOptions {
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
    pub fn latest(self) -> Self {
        Self {
            command_args: self.command_args.arg("LATEST"),
        }
    }

    /// filters samples by a list of specific timestamps.
    ///
    /// A sample passes the filter if its exact timestamp is specified and falls within [`from_timestamp`, `to_timestamp`].
    #[must_use]
    pub fn filter_by_ts(self, ts: impl SingleArgCollection<u64>) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER_BY_TS").arg(ts),
        }
    }

    /// filters samples by minimum and maximum values.
    #[must_use]
    pub fn filter_by_value(self, min: f64, max: f64) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER_BY_VALUE").arg(min).arg(max),
        }
    }

    /// limits the number of returned samples.
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
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
    pub fn align(self, align: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("ALIGN").arg(align),
        }
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
    pub fn aggregation(self, aggregator: TsAggregationType, bucket_duration: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("AGGREGATION")
                .arg(aggregator)
                .arg(bucket_duration),
        }
    }

    /// controls how bucket timestamps are reported.
    /// `bucket_timestamp` values include:
    /// * `-` or `low` - Timestamp reported for each bucket is the bucket's start time (default)
    /// * `+` or `high` - Timestamp reported for each bucket is the bucket's end time
    /// * `~` or `mid` - Timestamp reported for each bucket is the bucket's mid time (rounded down if not an integer)
    #[must_use]
    pub fn bucket_timestamp(self, bucket_timestamp: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("BUCKETTIMESTAMP")
                .arg(bucket_timestamp),
        }
    }

    /// A flag, which, when specified, reports aggregations also for empty buckets.
    /// when `aggregator` values are:
    /// * `sum`, `count` - the value reported for each empty bucket is `0`
    /// * `last` - the value reported for each empty bucket is the value
    ///    of the last sample before the bucket's start.
    ///    `NaN` when no such sample.
    /// * `twa` - the value reported for each empty bucket is the average value
    ///    over the bucket's timeframe based on linear interpolation
    ///    of the last sample before the bucket's start and the first sample after the bucket's end.
    ///    `NaN` when no such samples.
    /// * `min`, `max`, `range`, `avg`, `first`, `std.p`, `std.s` - the value reported for each empty bucket is `NaN`
    ///
    /// Regardless of the values of `from_timestamp` and `to_timestamp`,
    /// no data is reported for buckets that end before the earliest sample or begin after the latest sample in the time series.
    #[must_use]
    pub fn empty(self) -> Self {
        Self {
            command_args: self.command_args.arg("EMPTY"),
        }
    }
}

impl IntoArgs for TsRangeOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
