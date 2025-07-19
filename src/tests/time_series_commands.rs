use crate::{
    Result,
    commands::{
        FlushingMode, ServerCommands, TimeSeriesCommands, TsAddOptions, TsAggregationType,
        TsCreateOptions, TsCreateRuleOptions, TsDuplicatePolicy, TsGetOptions, TsGroupByOptions,
        TsIncrByDecrByOptions, TsMGetOptions, TsMRangeOptions, TsRangeOptions, TsRangeSample,
        TsSample,
    },
    tests::get_test_client,
};
use serial_test::serial;
use std::collections::HashMap;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_add() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let timestamp = client
        .ts_add(
            "temperature:3:11",
            1548149183000u64,
            27.,
            TsAddOptions::default().retention(31536000000),
        )
        .await?;
    assert_eq!(1548149183000u64, timestamp);

    let _timestamp = client
        .ts_add("temperature:3:11", "*", 30., TsAddOptions::default())
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_create() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temperature:2:32",
            TsCreateOptions::default()
                .retention(60000)
                .duplicate_policy(TsDuplicatePolicy::Max)
                .labels([("sensor_id", 2), ("area_id", 32)]),
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_alter() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temperature:2:32",
            TsCreateOptions::default()
                .retention(60000)
                .duplicate_policy(TsDuplicatePolicy::Max)
                .labels([("sensor_id", 2), ("area_id", 32)]),
        )
        .await?;

    client
        .ts_alter(
            "temperature:2:32",
            TsCreateOptions::default().labels([
                ("sensor_id", 2),
                ("area_id", 32),
                ("sub_area_id", 15),
            ]),
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_create_delete_rule() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temp:TLV",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    client
        .ts_create(
            "dailyAvgTemp:TLV",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    client
        .ts_createrule(
            "temp:TLV",
            "dailyAvgTemp:TLV",
            TsAggregationType::Twa,
            86400000,
            TsCreateRuleOptions::default(),
        )
        .await?;

    client
        .ts_create(
            "dailyDiffTemp:TLV",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    client
        .ts_createrule(
            "temp:TLV",
            "dailyDiffTemp:TLV",
            TsAggregationType::Range,
            86400000,
            TsCreateRuleOptions::default().align_timestamp(21600000),
        )
        .await?;

    client
        .ts_deleterule("temp:TLV", "dailyDiffTemp:TLV")
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_del() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_add("key", 10, 1., TsAddOptions::default())
        .await?;
    client
        .ts_add("key", 20, 1., TsAddOptions::default())
        .await?;
    client
        .ts_add("key", 30, 1., TsAddOptions::default())
        .await?;
    client
        .ts_add("key", 40, 1., TsAddOptions::default())
        .await?;
    client
        .ts_add("key", 50, 1., TsAddOptions::default())
        .await?;

    let deleted = client.ts_del("key", 20, 40).await?;
    assert_eq!(3, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_get() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temp:JLM",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "JLM")]),
        )
        .await?;

    let result = client.ts_get("temp:JLM", TsGetOptions::default()).await?;
    assert_eq!(None, result);

    client
        .ts_add("temp:JLM", 1005, 30., TsAddOptions::default())
        .await?;
    client
        .ts_add("temp:JLM", 1015, 35., TsAddOptions::default())
        .await?;
    client
        .ts_add("temp:JLM", 1025, 9999., TsAddOptions::default())
        .await?;
    client
        .ts_add("temp:JLM", 1035, 40., TsAddOptions::default())
        .await?;

    let result = client.ts_get("temp:JLM", TsGetOptions::default()).await?;
    assert_eq!(Some((1035, 40.)), result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_incrby() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let timestamp = client
        .ts_incrby(
            "a",
            232.,
            TsIncrByDecrByOptions::default().timestamp(1657811829000u64),
        )
        .await?;
    assert_eq!(1657811829000u64, timestamp);

    let timestamp = client
        .ts_incrby(
            "a",
            157.,
            TsIncrByDecrByOptions::default().timestamp(1657811829000u64),
        )
        .await?;
    assert_eq!(1657811829000u64, timestamp);

    let timestamp = client
        .ts_incrby(
            "a",
            432.,
            TsIncrByDecrByOptions::default().timestamp(1657811829000u64),
        )
        .await?;
    assert_eq!(1657811829000u64, timestamp);

    let timestamp = client
        .ts_incrby("b", 1., TsIncrByDecrByOptions::default())
        .await?;
    assert!(timestamp > 0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_info() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "key",
            TsCreateOptions::default()
                .labels([("label1", "value1"), ("label2", "value2")])
                .chunk_size(10000)
                .duplicate_policy(TsDuplicatePolicy::Max)
                .retention(100000),
        )
        .await?;

    client
        .ts_create(
            "dst",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    client
        .ts_createrule(
            "key",
            "dst",
            TsAggregationType::Avg,
            86400000,
            TsCreateRuleOptions::default(),
        )
        .await?;

    client
        .ts_add("key", 1000, 10., TsAddOptions::default())
        .await?;
    client
        .ts_add("key", 1010, 20., TsAddOptions::default())
        .await?;

    let info = client.ts_info("key", true).await?;
    log::debug!("key info: {info:?}");

    let info = client.ts_info("dst", true).await?;
    log::debug!("dst info: {info:?}");

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_madd() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.ts_create("key", TsCreateOptions::default()).await?;

    let timestamps: Vec<u64> = client.ts_madd(("key", 1000, 10.)).await?;
    assert_eq!(vec![1000], timestamps);

    let timestamps: Vec<u64> = client
        .ts_madd([("key", 1010, 20.), ("key", 1020, 30.)])
        .await?;
    assert_eq!(vec![1010, 1020], timestamps);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_mget() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temp:TLV",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    client
        .ts_create(
            "temp:JLM",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "JLM")]),
        )
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("temp:TLV", 1000, 30.),
            ("temp:TLV", 1010, 35.),
            ("temp:TLV", 1020, 9999.),
            ("temp:TLV", 1030, 40.),
        ])
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("temp:JLM", 1005, 30.),
            ("temp:JLM", 1015, 35.),
            ("temp:JLM", 1025, 9999.),
            ("temp:JLM", 1035, 40.),
        ])
        .await?;

    let results: Vec<(String, TsSample)> = client
        .ts_mget(TsMGetOptions::default().withlabels(), "type=temp")
        .await?;
    assert_eq!(2, results.len());
    assert_eq!("temp:JLM", results[0].0);
    assert_eq!(
        HashMap::from([
            ("type".to_owned(), "temp".to_owned()),
            ("location".to_owned(), "JLM".to_owned())
        ]),
        results[0].1.labels
    );
    assert_eq!((1035, 40.), results[0].1.timestamp_value);

    assert_eq!("temp:TLV", results[1].0);
    assert_eq!(
        HashMap::from([
            ("type".to_owned(), "temp".to_owned()),
            ("location".to_owned(), "TLV".to_owned())
        ]),
        results[1].1.labels
    );
    assert_eq!((1030, 40.), results[1].1.timestamp_value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_mrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "stock:A",
            TsCreateOptions::default().labels([("type", "stock"), ("name", "A")]),
        )
        .await?;

    client
        .ts_create(
            "stock:B",
            TsCreateOptions::default().labels([("type", "stock"), ("name", "B")]),
        )
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("stock:A", 1000, 100.),
            ("stock:A", 1010, 110.),
            ("stock:A", 1020, 120.),
        ])
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("stock:B", 1000, 120.),
            ("stock:B", 1010, 110.),
            ("stock:B", 1020, 120.),
        ])
        .await?;

    let results: Vec<(String, TsRangeSample)> = client
        .ts_mrange(
            "-",
            "+",
            TsMRangeOptions::default().withlabels(),
            "type=stock",
            TsGroupByOptions::new("type", TsAggregationType::Max),
        )
        .await?;

    assert_eq!(1, results.len());
    assert_eq!("type=stock", results[0].0);
    assert_eq!(
        vec![("type".to_owned(), "stock".to_owned()),],
        results[0].1.labels
    );
    assert_eq!(vec!["max"], results[0].1.reducers);
    assert_eq!(vec!["stock:A", "stock:B"], results[0].1.sources);
    assert_eq!(
        vec![(1000, 120.), (1010, 110.), (1020, 120.)],
        results[0].1.values
    );

    client
        .ts_add(
            "ts1",
            1548149180000u64,
            90.,
            TsAddOptions::default().labels([("metric", "cpu"), ("metric_name", "system")]),
        )
        .await?;
    client
        .ts_add("ts1", 1548149185000u64, 45., TsAddOptions::default())
        .await?;
    client
        .ts_add(
            "ts2",
            1548149180000u64,
            99.,
            TsAddOptions::default().labels([("metric", "cpu"), ("metric_name", "user")]),
        )
        .await?;

    let results: Vec<(String, TsRangeSample)> = client
        .ts_mrange(
            "-",
            "+",
            TsMRangeOptions::default().withlabels(),
            "metric=cpu",
            TsGroupByOptions::new("metric_name", TsAggregationType::Max),
        )
        .await?;
    log::debug!("results: {results:?}");
    assert_eq!(
        vec![(1548149180000, 90.), (1548149185000, 45.)],
        results[0].1.values
    );
    assert_eq!(vec![(1548149180000, 99.)], results[1].1.values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_mrevrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "stock:A",
            TsCreateOptions::default().labels([("type", "stock"), ("name", "A")]),
        )
        .await?;

    client
        .ts_create(
            "stock:B",
            TsCreateOptions::default().labels([("type", "stock"), ("name", "B")]),
        )
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("stock:A", 1000, 100.),
            ("stock:A", 1010, 110.),
            ("stock:A", 1020, 120.),
        ])
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("stock:B", 1000, 120.),
            ("stock:B", 1010, 110.),
            ("stock:B", 1020, 120.),
        ])
        .await?;

    let results: Vec<(String, TsRangeSample)> = client
        .ts_mrevrange(
            "-",
            "+",
            TsMRangeOptions::default().withlabels(),
            "type=stock",
            TsGroupByOptions::new("type", TsAggregationType::Max),
        )
        .await?;

    assert_eq!(1, results.len());
    assert_eq!("type=stock", results[0].0);
    assert_eq!(
        vec![("type".to_owned(), "stock".to_owned()),],
        results[0].1.labels
    );
    assert_eq!(vec!["max"], results[0].1.reducers);
    assert_eq!(vec!["stock:A", "stock:B"], results[0].1.sources);
    assert_eq!(
        vec![(1020, 120.), (1010, 110.), (1000, 120.)],
        results[0].1.values
    );

    client
        .ts_add(
            "ts1",
            1548149180000u64,
            90.,
            TsAddOptions::default().labels([("metric", "cpu"), ("metric_name", "system")]),
        )
        .await?;
    client
        .ts_add("ts1", 1548149185000u64, 45., TsAddOptions::default())
        .await?;
    client
        .ts_add(
            "ts2",
            1548149180000u64,
            99.,
            TsAddOptions::default().labels([("metric", "cpu"), ("metric_name", "user")]),
        )
        .await?;

    let results: Vec<(String, TsRangeSample)> = client
        .ts_mrevrange(
            "-",
            "+",
            TsMRangeOptions::default().withlabels(),
            "metric=cpu",
            TsGroupByOptions::new("metric_name", TsAggregationType::Max),
        )
        .await?;
    log::debug!("results: {results:?}");
    assert_eq!(
        vec![(1548149185000, 45.), (1548149180000, 90.)],
        results[0].1.values
    );
    assert_eq!(vec![(1548149180000, 99.)], results[1].1.values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_queryindex() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "telemetry:study:temperature",
            TsCreateOptions::default().labels([("room", "study"), ("type ", "temperature")]),
        )
        .await?;

    client
        .ts_create(
            "telemetry:study:humidity",
            TsCreateOptions::default().labels([("room", "study"), ("type ", "humidity")]),
        )
        .await?;

    client
        .ts_create(
            "telemetry:kitchen:temperature",
            TsCreateOptions::default().labels([("room", "kitchen"), ("type ", "temperature")]),
        )
        .await?;

    client
        .ts_create(
            "telemetry:kitchen:humidity",
            TsCreateOptions::default().labels([("room", "kitchen"), ("type ", "humidity")]),
        )
        .await?;

    let keys: Vec<String> = client.ts_queryindex("room=kitchen").await?;
    assert_eq!(
        vec![
            "telemetry:kitchen:humidity".to_owned(),
            "telemetry:kitchen:temperature".to_owned()
        ],
        keys
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_range() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temp:TLV",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("temp:TLV", 1000, 30.),
            ("temp:TLV", 1010, 35.),
            ("temp:TLV", 1020, 9999.),
            ("temp:TLV", 1030, 40.),
        ])
        .await?;

    // Now, retrieve all values except out-of-range values.
    let results: Vec<(u64, f64)> = client
        .ts_range(
            "temp:TLV",
            "-",
            "+",
            TsRangeOptions::default().filter_by_value(-100., 100.),
        )
        .await?;
    assert_eq!(vec![(1000, 30.), (1010, 35.), (1030, 40.),], results);

    // Now, retrieve the average value, while ignoring out-of-range values.
    let results: Vec<(u64, f64)> = client
        .ts_range(
            "temp:TLV",
            "-",
            "+",
            TsRangeOptions::default()
                .filter_by_value(-100., 100.)
                .aggregation(TsAggregationType::Avg, 1000),
        )
        .await?;
    assert_eq!(vec![(1000, 35.)], results);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ts_revrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ts_create(
            "temp:TLV",
            TsCreateOptions::default().labels([("type", "temp"), ("location", "TLV")]),
        )
        .await?;

    let _timestamps: Vec<u64> = client
        .ts_madd([
            ("temp:TLV", 1000, 30.),
            ("temp:TLV", 1010, 35.),
            ("temp:TLV", 1020, 9999.),
            ("temp:TLV", 1030, 40.),
        ])
        .await?;

    // Now, retrieve all values except out-of-range values.
    let results: Vec<(u64, f64)> = client
        .ts_revrange(
            "temp:TLV",
            "-",
            "+",
            TsRangeOptions::default().filter_by_value(-100., 100.),
        )
        .await?;
    assert_eq!(vec![(1030, 40.), (1010, 35.), (1000, 30.),], results);

    // Now, retrieve the average value, while ignoring out-of-range values.
    let results: Vec<(u64, f64)> = client
        .ts_revrange(
            "temp:TLV",
            "-",
            "+",
            TsRangeOptions::default()
                .filter_by_value(-100., 100.)
                .aggregation(TsAggregationType::Avg, 1000),
        )
        .await?;
    assert_eq!(vec![(1000, 35.)], results);

    Ok(())
}
