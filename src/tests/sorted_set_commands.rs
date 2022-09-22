use crate::{
    tests::get_default_addr, Connection, ConnectionCommandResult, GenericCommands, Result,
    SortedSetCommands, ZAddOptions, ZRangeOptions, ZRangeSortBy, ZScanOptions, ZWhere,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zadd() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let len = connection
        .zadd("key", (1.0, "one"), ZAddOptions::default())
        .send()
        .await?;
    assert_eq!(1, len);

    let len = connection
        .zadd(
            "key",
            [(2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let len = connection
        .zadd("key", (1.0, "uno"), ZAddOptions::default())
        .send()
        .await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let len = connection.zcard("key").send().await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zcount() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection.zcount("key", "-inf", "+inf").send().await?;
    assert_eq!(3, len);

    let len = connection.zcount("key", "(1", 3).send().await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zdiff() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2"]).send().await?;

    connection
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;
    connection
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let result: Vec<String> = connection.zdiff(["key1", "key2"]).send().await?;
    assert_eq!(1, result.len());
    assert_eq!("three".to_owned(), result[0]);

    let result: Vec<(String, f64)> = connection
        .zdiff_with_scores(["key1", "key2"])
        .send()
        .await?;
    assert_eq!(1, result.len());
    assert_eq!(("three".to_owned(), 3.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zdiffstore() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2", "out"]).send().await?;

    connection
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;
    connection
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let len = connection
        .zdiffstore("out", ["key1", "key2"])
        .send()
        .await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("out", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(1, values.len());
    assert_eq!(("three".to_owned(), 3.0), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zincrby() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let new_score = connection.zincrby("key", 2.0, "one").send().await?;
    assert_eq!(3.0, new_score);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2"]).send().await?;

    connection
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;
    connection
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let result: Vec<String> = connection
        .zinter(["key1", "key2"], None as Option<f64>, Default::default())
        .send()
        .await?;
    assert_eq!(2, result.len());
    assert_eq!("one".to_owned(), result[0]);
    assert_eq!("two".to_owned(), result[1]);

    let result: Vec<(String, f64)> = connection
        .zinter_with_scores(["key1", "key2"], None as Option<f64>, Default::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2", "out"]).send().await?;

    connection
        .zadd(
            "key1",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;
    connection
        .zadd("key2", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let len = connection
        .zinterstore(
            "out",
            ["key1", "key2"],
            Some([2.0, 3.0]),
            Default::default(),
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("out", 0, -1, ZRangeOptions::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
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
        .send()
        .await?;

    let len = connection.zlexcount("key", "-", "+").send().await?;
    assert_eq!(7, len);

    let len = connection.zlexcount("key", "[b", "[f").send().await?;
    assert_eq!(5, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zmpop() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key", "key2", "unknown"]).send().await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        connection.zmpop("unknown", ZWhere::Min, 1).send().await?;
    assert!(result.is_none());

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let result: Option<(String, Vec<(String, f64)>)> =
        connection.zmpop("key", ZWhere::Min, 1).send().await?;
    match result {
        Some(result) => {
            assert_eq!("key".to_owned(), result.0);
            assert_eq!(1, result.1.len());
            assert_eq!(("one".to_owned(), 1.0), result.1[0]);
        }
        None => assert!(false),
    }

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(2, values.len());
    assert_eq!(("two".to_owned(), 2.0), values[0]);
    assert_eq!(("three".to_owned(), 3.0), values[1]);

    let result: Option<(String, Vec<(String, f64)>)> =
        connection.zmpop("key", ZWhere::Max, 10).send().await?;
    match result {
        Some(result) => {
            assert_eq!("key".to_owned(), result.0);
            assert_eq!(2, result.1.len());
            assert_eq!(("three".to_owned(), 3.0), result.1[0]);
            assert_eq!(("two".to_owned(), 2.0), result.1[1]);
        }
        None => assert!(false),
    }

    connection
        .zadd(
            "key2",
            [(4.0, "four"), (5.0, "five"), (6.0, "six")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let result: Option<(String, Vec<(String, f64)>)> = connection
        .zmpop(["key", "key2"], ZWhere::Min, 10)
        .send()
        .await?;
    match result {
        Some(result) => {
            assert_eq!("key2".to_owned(), result.0);
            assert_eq!(3, result.1.len());
            assert_eq!(("four".to_owned(), 4.0), result.1[0]);
            assert_eq!(("five".to_owned(), 5.0), result.1[1]);
            assert_eq!(("six".to_owned(), 6.0), result.1[2]);
        }
        None => assert!(false),
    }

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(0, values.len());

    let result: Option<(String, Vec<(String, f64)>)> = connection
        .zmpop(["key", "key2"], ZWhere::Min, 10)
        .send()
        .await?;
    assert!(result.is_none());

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key2", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(0, values.len());

    let len = connection.exists(["key", "key2"]).send().await?;
    assert_eq!(0, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zmscore() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd("key", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;

    let scores = connection
        .zmscore("key", ["one", "two", "nofield"])
        .send()
        .await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let result: Vec<(String, f64)> = connection.zpopmax("key", 1).send().await?;
    assert_eq!(1, result.len());
    assert_eq!(("three".to_owned(), 3.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zpopmin() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let result: Vec<(String, f64)> = connection.zpopmin("key", 1).send().await?;
    assert_eq!(1, result.len());
    assert_eq!(("one".to_owned(), 1.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrandmember() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let values = [
        (1.0, "one"),
        (2.0, "two"),
        (3.0, "three"),
        (4.0, "four"),
        (5.0, "five"),
        (6.0, "six"),
    ];

    connection
        .zadd("key", values, ZAddOptions::default())
        .send()
        .await?;

    let result: String = connection.zrandmember("key").send().await?;
    assert!(values.iter().any(|v| v.1 == result));

    let result: Vec<(String, f64)> = connection
        .zrandmembers_with_scores("key", -5)
        .send()
        .await?;
    assert!(result
        .iter()
        .all(|r| values.iter().any(|v| v.0 == r.1 && v.1 == r.0)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrange() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let values: Vec<String> = connection
        .zrange("key", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(3, values.len());
    assert_eq!("one".to_owned(), values[0]);
    assert_eq!("two".to_owned(), values[1]);
    assert_eq!("three".to_owned(), values[2]);

    let values: Vec<String> = connection
        .zrange("key", 2, 3, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(1, values.len());
    assert_eq!("three".to_owned(), values[0]);

    let values: Vec<String> = connection
        .zrange("key", -2, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(2, values.len());
    assert_eq!("two".to_owned(), values[0]);
    assert_eq!("three".to_owned(), values[1]);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 1.0), values[0]);
    assert_eq!(("two".to_owned(), 2.0), values[1]);
    assert_eq!(("three".to_owned(), 3.0), values[2]);

    let values: Vec<String> = connection
        .zrange(
            "key",
            "(1",
            "+inf",
            ZRangeOptions::default()
                .sort_by(ZRangeSortBy::ByScore)
                .limit(1, 1),
        )
        .send()
        .await?;
    assert_eq!(1, values.len());
    assert_eq!("three".to_owned(), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrangestore() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key", "out"]).send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three"), (4.0, "four")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection
        .zrangestore("out", "key", 2, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(2, len);

    let values: Vec<String> = connection
        .zrange("key", -2, -1, ZRangeOptions::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection.zrank("key", "three").send().await?;
    assert_eq!(Some(2), len);

    let len = connection.zrank("key", "four").send().await?;
    assert_eq!(None, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrem() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection.zrem("key", "two").send().await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
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
        .send()
        .await?;

    let len = connection
        .zremrangebylex("key", "[alpha", "[omega")
        .send()
        .await?;
    assert_eq!(6, len);

    let values: Vec<String> = connection
        .zrange("key", 0, -1, ZRangeOptions::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection.zremrangebyrank("key", 0, 1).send().await?;
    assert_eq!(2, len);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("key", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(1, values.len());
    assert_eq!(("three".to_owned(), 3.0), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrevrank() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection.zrevrank("key", "one").send().await?;
    assert_eq!(Some(2), len);

    let len = connection.zrevrank("key", "four").send().await?;
    assert_eq!(None, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zscan() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let result = connection
        .zscan("key", 0, ZScanOptions::default())
        .send()
        .await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .zadd(
            "key",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let score = connection.zscore("key", "one").send().await?;
    assert_eq!(Some(1.0), score);

    let score = connection.zscore("key", "four").send().await?;
    assert_eq!(None, score);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zunion() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2"]).send().await?;

    connection
        .zadd("key1", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;
    connection
        .zadd(
            "key2",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let result: Vec<String> = connection
        .zunion(["key1", "key2"], None as Option<f64>, Default::default())
        .send()
        .await?;
    assert_eq!(3, result.len());
    assert_eq!("one".to_owned(), result[0]);
    assert_eq!("three".to_owned(), result[1]);
    assert_eq!("two".to_owned(), result[2]);

    let result: Vec<(String, f64)> = connection
        .zunion_with_scores(["key1", "key2"], None as Option<f64>, Default::default())
        .send()
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del(["key1", "key2", "out"]).send().await?;

    connection
        .zadd("key1", [(1.0, "one"), (2.0, "two")], ZAddOptions::default())
        .send()
        .await?;
    connection
        .zadd(
            "key2",
            [(1.0, "one"), (2.0, "two"), (3.0, "three")],
            ZAddOptions::default(),
        )
        .send()
        .await?;

    let len = connection
        .zunionstore(
            "out",
            ["key1", "key2"],
            Some([2.0, 3.0]),
            Default::default(),
        )
        .send()
        .await?;
    assert_eq!(3, len);

    let values: Vec<(String, f64)> = connection
        .zrange_with_scores("out", 0, -1, ZRangeOptions::default())
        .send()
        .await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 5.0), values[0]);
    assert_eq!(("three".to_owned(), 9.0), values[1]);
    assert_eq!(("two".to_owned(), 10.0), values[2]);

    Ok(())
}
