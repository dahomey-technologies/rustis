use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, Result, SortedSetCommands,
    ZRangeSortBy, ZScanResult, ZWhere,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zadd() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let len = database.zadd("key").execute((1.0, "one")).await?;
    assert_eq!(1, len);

    let len = database
        .zadd("key")
        .execute([(2.0, "two"), (3.0, "three")])
        .await?;
    assert_eq!(2, len);

    let len = database.zadd("key").execute((1.0, "uno")).await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let len = database.zcard("key").await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zcount() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let len = database.zcount("key", "-inf", "+inf").await?;
    assert_eq!(3, len);

    let len = database.zcount("key", "(1", 3).await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zdiff() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2"]).await?;

    database
        .zadd("key1")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;
    database
        .zadd("key2")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let result: Vec<String> = database.zdiff(["key1", "key2"]).execute().await?;
    assert_eq!(1, result.len());
    assert_eq!("three".to_owned(), result[0]);

    let result: Vec<(String, f64)> = database.zdiff(["key1", "key2"]).with_scores().await?;
    assert_eq!(1, result.len());
    assert_eq!(("three".to_owned(), 3.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zdiffstore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "out"]).await?;

    database
        .zadd("key1")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;
    database
        .zadd("key2")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let len = database.zdiffstore("out", ["key1", "key2"]).await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = database.zrange("out", 0, -1).with_scores().await?;
    assert_eq!(1, values.len());
    assert_eq!(("three".to_owned(), 3.0), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zincrby() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let new_score = database.zincrby("key", 2.0, "one").await?;
    assert_eq!(3.0, new_score);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
    assert_eq!(2, values.len());
    assert_eq!(("two".to_owned(), 2.0), values[0]);
    assert_eq!(("one".to_owned(), 3.0), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zinter() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2"]).await?;

    database
        .zadd("key1")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;
    database
        .zadd("key2")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let result: Vec<String> = database.zinter(["key1", "key2"]).execute().await?;
    assert_eq!(2, result.len());
    assert_eq!("one".to_owned(), result[0]);
    assert_eq!("two".to_owned(), result[1]);

    let result: Vec<(String, f64)> = database.zinter(["key1", "key2"]).with_scores().await?;
    assert_eq!(2, result.len());
    assert_eq!(("one".to_owned(), 2.0), result[0]);
    assert_eq!(("two".to_owned(), 4.0), result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zinterstore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "out"]).await?;

    database
        .zadd("key1")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;
    database
        .zadd("key2")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let len = database
        .zinterstore("out", ["key1", "key2"])
        .weights([2.0, 3.0])
        .execute()
        .await?;
    assert_eq!(2, len);

    let values: Vec<(String, f64)> = database.zrange("out", 0, -1).with_scores().await?;
    assert_eq!(2, values.len());
    assert_eq!(("one".to_owned(), 5.0), values[0]);
    assert_eq!(("two".to_owned(), 10.0), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zlexcount() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([
            (0.0, "a"),
            (0.0, "b"),
            (0.0, "c"),
            (0.0, "d"),
            (0.0, "e"),
            (0.0, "f"),
            (0.0, "g"),
        ])
        .await?;

    let len = database.zlexcount("key", "-", "+").await?;
    assert_eq!(7, len);

    let len = database.zlexcount("key", "[b", "[f").await?;
    assert_eq!(5, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zmpop() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key", "key2", "unknown"]).await?;

    let result: (String, Vec<(String, f64)>) = database.zmpop("unknown", ZWhere::Min, 1).await?;
    assert_eq!("".to_owned(), result.0);
    assert_eq!(0, result.1.len());

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let result: (String, Vec<(String, f64)>) = database.zmpop("key", ZWhere::Min, 1).await?;
    assert_eq!("key".to_owned(), result.0);
    assert_eq!(1, result.1.len());
    assert_eq!(("one".to_owned(), 1.0), result.1[0]);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
    assert_eq!(2, values.len());
    assert_eq!(("two".to_owned(), 2.0), values[0]);
    assert_eq!(("three".to_owned(), 3.0), values[1]);

    let result: (String, Vec<(String, f64)>) = database.zmpop("key", ZWhere::Max, 10).await?;
    assert_eq!("key".to_owned(), result.0);
    assert_eq!(2, result.1.len());
    assert_eq!(("three".to_owned(), 3.0), result.1[0]);
    assert_eq!(("two".to_owned(), 2.0), result.1[1]);

    database
        .zadd("key2")
        .execute([(4.0, "four"), (5.0, "five"), (6.0, "six")])
        .await?;

    let result: (String, Vec<(String, f64)>) =
        database.zmpop(["key", "key2"], ZWhere::Min, 10).await?;
    assert_eq!("key2".to_owned(), result.0);
    assert_eq!(3, result.1.len());
    assert_eq!(("four".to_owned(), 4.0), result.1[0]);
    assert_eq!(("five".to_owned(), 5.0), result.1[1]);
    assert_eq!(("six".to_owned(), 6.0), result.1[2]);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
    assert_eq!(0, values.len());

    let result: (String, Vec<(String, f64)>) =
        database.zmpop(["key", "key2"], ZWhere::Min, 10).await?;
    assert_eq!("".to_owned(), result.0);
    assert_eq!(0, result.1.len());

    let values: Vec<(String, f64)> = database.zrange("key2", 0, -1).with_scores().await?;
    assert_eq!(0, values.len());

    let len = database.exists(["key", "key2"]).await?;
    assert_eq!(0, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zmscore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;

    let scores = database.zmscore("key", ["one", "two", "nofield"]).await?;
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let result: Vec<(String, f64)> = database.zpopmax("key", 1).await?;
    assert_eq!(1, result.len());
    assert_eq!(("three".to_owned(), 3.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zpopmin() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let result: Vec<(String, f64)> = database.zpopmin("key", 1).await?;
    assert_eq!(1, result.len());
    assert_eq!(("one".to_owned(), 1.0), result[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrandmember() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let values = [
        (1.0, "one"),
        (2.0, "two"),
        (3.0, "three"),
        (4.0, "four"),
        (5.0, "five"),
        (6.0, "six"),
    ];

    database.zadd("key").execute(values).await?;

    let result: String = database.zrandmember("key").execute().await?;
    assert!(values.iter().any(|v| v.1 == result));

    let result: Vec<(String, f64)> = database.zrandmember("key").count(-5).with_scores().await?;
    assert!(result
        .iter()
        .all(|r| values.iter().any(|v| v.0 == r.1 && v.1 == r.0)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrange() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let values: Vec<String> = database.zrange("key", 0, -1).execute().await?;
    assert_eq!(3, values.len());
    assert_eq!("one".to_owned(), values[0]);
    assert_eq!("two".to_owned(), values[1]);
    assert_eq!("three".to_owned(), values[2]);

    let values: Vec<String> = database.zrange("key", 2, 3).execute().await?;
    assert_eq!(1, values.len());
    assert_eq!("three".to_owned(), values[0]);

    let values: Vec<String> = database.zrange("key", -2, -1).execute().await?;
    assert_eq!(2, values.len());
    assert_eq!("two".to_owned(), values[0]);
    assert_eq!("three".to_owned(), values[1]);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 1.0), values[0]);
    assert_eq!(("two".to_owned(), 2.0), values[1]);
    assert_eq!(("three".to_owned(), 3.0), values[2]);

    let values: Vec<String> = database
        .zrange("key", "(1", "+inf")
        .sort_by(ZRangeSortBy::ByScore)
        .limit(1, 1)
        .execute()
        .await?;
    assert_eq!(1, values.len());
    assert_eq!("three".to_owned(), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrangestore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key", "out"]).await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three"), (4.0, "four")])
        .await?;

    let len = database.zrangestore("out", "key", 2, -1).execute().await?;
    assert_eq!(2, len);

    let values: Vec<String> = database.zrange("key", -2, -1).execute().await?;
    assert_eq!(2, values.len());
    assert_eq!("three".to_owned(), values[0]);
    assert_eq!("four".to_owned(), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrank() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let len = database.zrank("key", "three").await?;
    assert_eq!(Some(2), len);

    let len = database.zrank("key", "four").await?;
    assert_eq!(None, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrem() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let len = database.zrem("key", "two").await?;
    assert_eq!(1, len);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
    assert_eq!(2, values.len());
    assert_eq!(("one".to_owned(), 1.0), values[0]);
    assert_eq!(("three".to_owned(), 3.0), values[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zremrangebylex() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([
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
        ])
        .await?;

    let len = database.zremrangebylex("key", "[alpha", "[omega").await?;
    assert_eq!(6, len);

    let values: Vec<String> = database.zrange("key", 0, -1).execute().await?;
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let len = database.zremrangebyrank("key", 0, 1).await?;
    assert_eq!(2, len);

    let values: Vec<(String, f64)> = database.zrange("key", 0, -1).with_scores().await?;
    assert_eq!(1, values.len());
    assert_eq!(("three".to_owned(), 3.0), values[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zrevrank() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let len = database.zrevrank("key", "one").await?;
    assert_eq!(Some(2), len);

    let len = database.zrevrank("key", "four").await?;
    assert_eq!(None, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zscan() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let result: ZScanResult<String> = database.zscan("key", 0).execute().await?;
    assert_eq!(0, result.cursor);
    assert_eq!(3, result.elements.len());
    assert_eq!(("one".to_owned(), 1.0), result.elements[0]);
    assert_eq!(("two".to_owned(), 2.0), result.elements[1]);
    assert_eq!(("three".to_owned(), 3.0), result.elements[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zscore() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database
        .zadd("key")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let score = database.zscore("key", "one").await?;
    assert_eq!(Some(1.0), score);

    let score = database.zscore("key", "four").await?;
    assert_eq!(None, score);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn zunion() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2"]).await?;

    database
        .zadd("key1")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;
    database
        .zadd("key2")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let result: Vec<String> = database.zunion(["key1", "key2"]).execute().await?;
    assert_eq!(3, result.len());
    assert_eq!("one".to_owned(), result[0]);
    assert_eq!("three".to_owned(), result[1]);
    assert_eq!("two".to_owned(), result[2]);

    let result: Vec<(String, f64)> = database.zunion(["key1", "key2"]).with_scores().await?;
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["key1", "key2", "out"]).await?;

    database
        .zadd("key1")
        .execute([(1.0, "one"), (2.0, "two")])
        .await?;
    database
        .zadd("key2")
        .execute([(1.0, "one"), (2.0, "two"), (3.0, "three")])
        .await?;

    let len = database
        .zunionstore("out", ["key1", "key2"])
        .weights([2.0, 3.0])
        .execute()
        .await?;
    assert_eq!(3, len);

    let values: Vec<(String, f64)> = database.zrange("out", 0, -1).with_scores().await?;
    assert_eq!(3, values.len());
    assert_eq!(("one".to_owned(), 5.0), values[0]);
    assert_eq!(("three".to_owned(), 9.0), values[1]);
    assert_eq!(("two".to_owned(), 10.0), values[2]);

    Ok(())
}
