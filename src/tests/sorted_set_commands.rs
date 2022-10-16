use crate::{
    spawn,
    tests::{get_test_client, sleep},
    FlushingMode, GenericCommands, Result, ServerCommands, SortedSetCommands, ZAddOptions,
    ZRangeOptions, ZRangeSortBy, ZScanOptions, ZWhere,
};
use serial_test::serial;
use std::time::Duration;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bzmpop() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        client.bzmpop(0.01, "unknown", ZWhere::Min, 1).await?;
    assert!(result.is_none());

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        client.bzmpop(0.0, "key", ZWhere::Min, 1).await?;
    match result {
        Some(result) => {
            assert_eq!("key".to_owned(), result.0);
            assert_eq!(1, result.1.len());
            assert_eq!(("one".to_owned(), 1.0), result.1[0]);
        }
        None => unreachable!(),
    }

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!(("two".to_owned(), 2.0), values[0]);
    assert_eq!(("three".to_owned(), 3.0), values[1]);

    let result: Option<(String, Vec<(String, f64)>)> =
        client.bzmpop(0.0, "key", ZWhere::Max, 10).await?;
    match result {
        Some(result) => {
            assert_eq!("key".to_owned(), result.0);
            assert_eq!(2, result.1.len());
            assert_eq!(("three".to_owned(), 3.0), result.1[0]);
            assert_eq!(("two".to_owned(), 2.0), result.1[1]);
        }
        None => unreachable!(),
    }

    client
        .zadd(
            "key2",
            [(4.0, "four"), (5.0, "five"), (6.0, "six")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        client.bzmpop(0.0, ["key", "key2"], ZWhere::Min, 10).await?;
    match result {
        Some(result) => {
            assert_eq!("key2".to_owned(), result.0);
            assert_eq!(3, result.1.len());
            assert_eq!(("four".to_owned(), 4.0), result.1[0]);
            assert_eq!(("five".to_owned(), 5.0), result.1[1]);
            assert_eq!(("six".to_owned(), 6.0), result.1[2]);
        }
        None => unreachable!(),
    }

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(0, values.len());

    let result: Option<(String, Vec<(String, f64)>)> = client
        .bzmpop(0.01, ["key", "key2"], ZWhere::Min, 10)
        .await?;
    assert!(result.is_none());

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key2", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(0, values.len());

    let len = client.exists(["key", "key2"]).await?;
    assert_eq!(0, len);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let result: Option<(String, Vec<(String, f64)>)> =
                client.bzmpop(0.0, "key", ZWhere::Min, 1).await?;
            match result {
                Some((key, elements)) => {
                    assert_eq!("key", key);
                    assert_eq!(1, elements.len());
                    assert_eq!(("four".to_owned(), 4.0), elements[0]);
                }
                None => unreachable!(),
            }

            Ok(())
        }

        let _result = calls().await;
    });

    client
        .zadd("key", (4.0, "four"), ZAddOptions::default())
        .await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bzpopmax() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .zadd("key", (1.0, "one"), ZAddOptions::default())
        .await?;

    let result: Option<Vec<(String, String, f64)>> =
        client.bzpopmax(["key", "unknown"], 0.0).await?;

    match result {
        Some(result) => {
            assert_eq!(1, result.len());
            assert_eq!(("key".to_owned(), "one".to_owned(), 1.0), result[0]);
        }
        None => unreachable!(),
    }

    let result: Option<Vec<(String, String, f64)>> = client.bzpopmax("unknown", 0.01).await?;
    assert_eq!(None, result);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let result: Option<Vec<(String, String, f64)>> =
                client.bzpopmax(["key", "unknown"], 0.0).await?;

            match result {
                Some(result) => {
                    assert_eq!(1, result.len());
                    assert_eq!(("key".to_owned(), "two".to_owned(), 2.0), result[0]);
                }
                None => unreachable!(),
            }

            Ok(())
        }

        let _result = calls().await;
    });

    client
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bzpopmin() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .zadd("key", (1.0, "one"), ZAddOptions::default())
        .await?;

    let result: Option<Vec<(String, String, f64)>> =
        client.bzpopmin(["key", "unknown"], 0.0).await?;

    match result {
        Some(result) => {
            assert_eq!(1, result.len());
            assert_eq!(("key".to_owned(), "one".to_owned(), 1.0), result[0]);
        }
        None => unreachable!(),
    }

    let result: Option<Vec<(String, String, f64)>> = client.bzpopmin("unknown", 0.01).await?;
    assert_eq!(None, result);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let result: Option<Vec<(String, String, f64)>> =
                client.bzpopmin(["key", "unknown"], 0.0).await?;

            match result {
                Some(result) => {
                    assert_eq!(1, result.len());
                    assert_eq!(("key".to_owned(), "one".to_owned(), 1.0), result[0]);
                }
                None => unreachable!(),
            }

            Ok(())
        }

        let _result = calls().await;
    });

    client
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zadd() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let len = client
        .zadd("key", (1.0, "one"), ZAddOptions::default())
        .await?;
    assert_eq!(1, len);

    let len = client
        .zadd(
            "key",
            [(2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;
    assert_eq!(2, len);

    let len = client
        .zadd("key", (1.0, "uno"), ZAddOptions::default())
        .await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(4, values.len());
    assert_eq!(("one".to_owned(), 1.0), values[0]);
    assert_eq!(("uno".to_owned(), 1.0), values[1]);
    assert_eq!(("two".to_owned(), 2.0), values[2]);
    assert_eq!(("three".to_owned(), 3.0), values[3]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zcard() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let len = client.zcard("key").await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zcount() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zcount("key", "-inf", "+inf").await?;
    assert_eq!(3, len);

    let len = client.zcount("key", "(1", 3).await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zdiff() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;
    client
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let result: Vec<String> = client.zdiff(["key1", "key2"]).await?;
    assert_eq!(1, result.len());
    assert_eq!("three".to_owned(), result[0]);

    let result: Vec<(String, f64)> = client.zdiff_with_scores(["key1", "key2"]).await?;
    assert_eq!(1, result.len());
    assert_eq!(("three".to_owned(), 3.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zdiffstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "out"]).await?;

    client
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;
    client
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let len = client.zdiffstore("out", ["key1", "key2"]).await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("out", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(1, values.len());
    assert_eq!(("three".to_owned(), 3.0), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zincrby() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let new_score = client.zincrby("key", 2.0, "one").await?;
    assert_eq!(3.0, new_score);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!(("two".to_owned(), 2.0), values[0]);
    assert_eq!(("one".to_owned(), 3.0), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zinter() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;
    client
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let result: Vec<String> = client
        .zinter(["key1", "key2"], None as Option<f64>, Default::default())
        .await?;
    assert_eq!(2, result.len());
    assert_eq!("one".to_owned(), result[0]);
    assert_eq!("two".to_owned(), result[1]);

    let result: Vec<(String, f64)> = client
        .zinter_with_scores(["key1", "key2"], None as Option<f64>, Default::default())
        .await?;
    assert_eq!(2, result.len());
    assert_eq!(("one".to_owned(), 2.0), result[0]);
    assert_eq!(("two".to_owned(), 4.0), result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zinterstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "out"]).await?;

    client
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;
    client
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let len = client
        .zinterstore(
            "out",
            ["key1", "key2"],
            Some([2.0, 3.0]),
            Default::default(),
        )
        .await?;
    assert_eq!(2, len);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("out", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!(("one".to_owned(), 5.0), values[0]);
    assert_eq!(("two".to_owned(), 10.0), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zlexcount() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [
                (0.0, "a"),
                (0.0, "b"),
                (0.0, "c"),
                (0.0, "d"),
                (0.0, "e"),
                (0.0, "f"),
                (0.0, "g"),
            ],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zlexcount("key", "-", "+").await?;
    assert_eq!(7, len);

    let len = client.zlexcount("key", "[b", "[f").await?;
    assert_eq!(5, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zmpop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key", "key2", "unknown"]).await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        client.zmpop("unknown", ZWhere::Min, 1).await?;
    assert!(result.is_none());

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Option<(String, Vec<(String, f64)>)> = client.zmpop("key", ZWhere::Min, 1).await?;
    match result {
        Some(result) => {
            assert_eq!("key".to_owned(), result.0);
            assert_eq!(1, result.1.len());
            assert_eq!(("one".to_owned(), 1.0), result.1[0]);
        }
        None => unreachable!(),
    }

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!(("two".to_owned(), 2.0), values[0]);
    assert_eq!(("three".to_owned(), 3.0), values[1]);

    let result: Option<(String, Vec<(String, f64)>)> = client.zmpop("key", ZWhere::Max, 10).await?;
    match result {
        Some(result) => {
            assert_eq!("key".to_owned(), result.0);
            assert_eq!(2, result.1.len());
            assert_eq!(("three".to_owned(), 3.0), result.1[0]);
            assert_eq!(("two".to_owned(), 2.0), result.1[1]);
        }
        None => unreachable!(),
    }

    client
        .zadd(
            "key2",
            [(4.0, "four"), (5.0, "five"), (6.0, "six")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        client.zmpop(["key", "key2"], ZWhere::Min, 10).await?;
    match result {
        Some(result) => {
            assert_eq!("key2".to_owned(), result.0);
            assert_eq!(3, result.1.len());
            assert_eq!(("four".to_owned(), 4.0), result.1[0]);
            assert_eq!(("five".to_owned(), 5.0), result.1[1]);
            assert_eq!(("six".to_owned(), 6.0), result.1[2]);
        }
        None => unreachable!(),
    }

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(0, values.len());

    let result: Option<(String, Vec<(String, f64)>)> =
        client.zmpop(["key", "key2"], ZWhere::Min, 10).await?;
    assert!(result.is_none());

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key2", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(0, values.len());

    let len = client.exists(["key", "key2"]).await?;
    assert_eq!(0, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zmscore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;

    let scores = client.zmscore("key", ["one", "two", "nofield"]).await?;
    assert_eq!(3, scores.len());
    assert_eq!(Some(1.0), scores[0]);
    assert_eq!(Some(2.0), scores[1]);
    assert_eq!(None, scores[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zpopmax() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Vec<(String, f64)> = client.zpopmax("key", 1).await?;
    assert_eq!(1, result.len());
    assert_eq!(("three".to_owned(), 3.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zpopmin() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Vec<(String, f64)> = client.zpopmin("key", 1).await?;
    assert_eq!(1, result.len());
    assert_eq!(("one".to_owned(), 1.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrandmember() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let values = [
        (1.0, "one"),
        (2.0, "two"),
        (3.0, "three"),
        (4.0, "four"),
        (5.0, "five"),
        (6.0, "six"),
    ];

    client.zadd("key", values, ZAddOptions::default()).await?;

    let result: String = client.zrandmember("key").await?;
    assert!(values.iter().any(|v| v.1 == result));

    let result: Vec<(String, f64)> = client.zrandmembers_with_scores("key", -5).await?;
    assert!(result
        .iter()
        .all(|r| values.iter().any(|v| v.0 == r.1 && v.1 == r.0)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrange() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let values: Vec<String> = client
        .zrange("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(3, values.len());
    assert_eq!("one".to_owned(), values[0]);
    assert_eq!("two".to_owned(), values[1]);
    assert_eq!("three".to_owned(), values[2]);

    let values: Vec<String> = client.zrange("key", 2, 3, ZRangeOptions::default()).await?;
    assert_eq!(1, values.len());
    assert_eq!("three".to_owned(), values[0]);

    let values: Vec<String> = client
        .zrange("key", -2, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!("two".to_owned(), values[0]);
    assert_eq!("three".to_owned(), values[1]);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 1.0), values[0]);
    assert_eq!(("two".to_owned(), 2.0), values[1]);
    assert_eq!(("three".to_owned(), 3.0), values[2]);

    let values: Vec<String> = client
        .zrange(
            "key",
            "(1",
            "+inf",
            ZRangeOptions::default()
                .sort_by(ZRangeSortBy::ByScore)
                .limit(1, 1),
        )
        .await?;
    assert_eq!(1, values.len());
    assert_eq!("three".to_owned(), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrangestore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key", "out"]).await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three"), (4.0, "four")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client
        .zrangestore("out", "key", 2, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, len);

    let values: Vec<String> = client
        .zrange("key", -2, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!("three".to_owned(), values[0]);
    assert_eq!("four".to_owned(), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrank() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zrank("key", "three").await?;
    assert_eq!(Some(2), len);

    let len = client.zrank("key", "four").await?;
    assert_eq!(None, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrem() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zrem("key", "two").await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(2, values.len());
    assert_eq!(("one".to_owned(), 1.0), values[0]);
    assert_eq!(("three".to_owned(), 3.0), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zremrangebylex() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [
                (0.0, "aaaa"),
                (0.0, "b"),
                (0.0, "c"),
                (0.0, "d"),
                (0.0, "e"),
                (0.0, "foo"),
                (0.0, "zap"),
                (0.0, "zip"),
                (0.0, "ALPHA"),
                (0.0, "alpha"),
            ],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zremrangebylex("key", "[alpha", "[omega").await?;
    assert_eq!(6, len);

    let values: Vec<String> = client
        .zrange("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(4, values.len());
    assert_eq!("ALPHA".to_owned(), values[0]);
    assert_eq!("aaaa".to_owned(), values[1]);
    assert_eq!("zap".to_owned(), values[2]);
    assert_eq!("zip".to_owned(), values[3]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zremrangebyrank() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zremrangebyrank("key", 0, 1).await?;
    assert_eq!(2, len);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(1, values.len());
    assert_eq!(("three".to_owned(), 3.0), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrevrank() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client.zrevrank("key", "one").await?;
    assert_eq!(Some(2), len);

    let len = client.zrevrank("key", "four").await?;
    assert_eq!(None, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zscan() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let result = client.zscan("key", 0, ZScanOptions::default()).await?;
    assert_eq!(0, result.0);
    assert_eq!(3, result.1.len());
    assert_eq!(("one".to_owned(), 1.0), result.1[0]);
    assert_eq!(("two".to_owned(), 2.0), result.1[1]);
    assert_eq!(("three".to_owned(), 3.0), result.1[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zscore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let score = client.zscore("key", "one").await?;
    assert_eq!(Some(1.0), score);

    let score = client.zscore("key", "four").await?;
    assert_eq!(None, score);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zunion() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client
        .zadd("key1", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;
    client
        .zadd(
            "key2",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let result: Vec<String> = client
        .zunion(["key1", "key2"], None as Option<f64>, Default::default())
        .await?;
    assert_eq!(3, result.len());
    assert_eq!("one".to_owned(), result[0]);
    assert_eq!("three".to_owned(), result[1]);
    assert_eq!("two".to_owned(), result[2]);

    let result: Vec<(String, f64)> = client
        .zunion_with_scores(["key1", "key2"], None as Option<f64>, Default::default())
        .await?;
    assert_eq!(3, result.len());
    assert_eq!(("one".to_owned(), 2.0), result[0]);
    assert_eq!(("three".to_owned(), 3.0), result[1]);
    assert_eq!(("two".to_owned(), 4.0), result[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zunionstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "out"]).await?;

    client
        .zadd("key1", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .await?;
    client
        .zadd(
            "key2",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .await?;

    let len = client
        .zunionstore(
            "out",
            ["key1", "key2"],
            Some([2.0, 3.0]),
            Default::default(),
        )
        .await?;
    assert_eq!(3, len);

    let values: Vec<(String, f64)> = client
        .zrange_with_scores("out", 0, -1, ZRangeOptions::default())
        .await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 5.0), values[0]);
    assert_eq!(("three".to_owned(), 9.0), values[1]);
    assert_eq!(("two".to_owned(), 10.0), values[2]);

    Ok(())
}
