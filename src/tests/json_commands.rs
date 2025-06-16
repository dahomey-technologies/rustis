use crate::{
    commands::{FlushingMode, JsonArrIndexOptions, JsonCommands, JsonGetOptions, ServerCommands},
    resp::Value,
    tests::get_test_client,
    Result,
};
use serial_test::serial;
use smallvec::SmallVec;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrappend() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":[1,2,3]},{"bar":[3,4,5]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_arrappend("key", "$.fooo", [4, 5]).await?;
    assert_eq!(0, result.len());

    let result: Vec<Option<usize>> = client.json_arrappend("key", "$.foo[*].bar", [4, 5]).await?;
    assert_eq!(3, result.len());
    assert_eq!(Some(5), result[0]);
    assert_eq!(Some(5), result[1]);
    assert_eq!(None, result[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrindex() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":[1,2,3]},{"bar":[3,4,5]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<isize>> = client
        .json_arrindex("key", "$.foo[*].bar", "1", JsonArrIndexOptions::default())
        .await?;
    assert_eq!(3, result.len());
    assert_eq!(Some(0), result[0]);
    assert_eq!(Some(-1), result[1]);
    assert_eq!(None, result[2]);

    let result: Vec<Option<isize>> = client
        .json_arrindex("key", "$.foo[*].bar", "3", JsonArrIndexOptions::default())
        .await?;
    assert_eq!(3, result.len());
    assert_eq!(Some(2), result[0]);
    assert_eq!(Some(0), result[1]);
    assert_eq!(None, result[2]);

    let result: Vec<Option<isize>> = client
        .json_arrindex(
            "key",
            "$.foo[0].bar[0].1",
            "3",
            JsonArrIndexOptions::default(),
        )
        .await?;
    assert_eq!(0, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrinsert() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":[1,2,3]},{"bar":[3,4,5]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client
        .json_arrinsert("key", "$.foo[*].bar", -1, "4")
        .await?;
    assert_eq!(3, result.len());
    assert_eq!(Some(4), result[0]);
    assert_eq!(Some(4), result[1]);
    assert_eq!(None, result[2]);

    let result: Vec<Option<usize>> = client.json_arrinsert("key", "$.foo[0].bar", 1, "5").await?;
    assert_eq!(1, result.len());
    assert_eq!(Some(5), result[0]);

    // not an array
    let result: Vec<Option<usize>> = client
        .json_arrinsert("key", "$.foo[0].bar[0].1", -1, "6")
        .await?;
    assert_eq!(0, result.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrlen() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"prop1":12,"prop2":"foo","prop3":["foo","bar"],"prop4":[12,13,14]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_arrlen("key", "$.[*]").await?;
    assert_eq!(4, result.len());
    assert_eq!(None, result[0]);
    assert_eq!(None, result[1]);
    assert_eq!(Some(2), result[2]);
    assert_eq!(Some(3), result[3]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrpop() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":[1,2,3]},{"bar":[3,4,5]}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<i64> = client.json_arrpop("key", "$.foo[*].bar", -1).await?;
    assert_eq!(2, result.len());
    assert_eq!(3, result[0]);
    assert_eq!(5, result[1]);

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":["a","b","c"]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<String>> = client.json_arrpop("key", "$.foo[*].bar", -1).await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(r#""c""#.to_owned()), result[0]);
    assert_eq!(None, result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrtrim() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":["a","b","c"]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_arrtrim("key", "$.foo[*].bar", 0, -1).await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(3), result[0]);
    assert_eq!(None, result[1]);

    let result: Vec<Option<usize>> = client.json_arrtrim("key", "$.foo[*].bar", 1, 1).await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(1), result[0]);
    assert_eq!(None, result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_clear() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":["a","b","c"]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let num: usize = client.json_clear("key", "$.foo[*].bar").await?;
    assert_eq!(2, num);

    let json: String = client.json_get("key", JsonGetOptions::default()).await?;
    assert_eq!(r#"{"foo":[{"bar":[]},{"bar":0}]}"#, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_debug_memory() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":["a","b","c"]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<usize> = client.json_debug_memory("key", "$.foo[*].bar").await?;
    assert_eq!(2, result.len());
    assert_eq!(59, result[0]);
    assert_eq!(8, result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_del() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":["a","b","c"]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let num_deleted = client.json_del("key", "$").await?;
    assert_eq!(1, num_deleted);

    let json: Option<String> = client.json_get("key", JsonGetOptions::default()).await?;
    assert_eq!(None, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_forget() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":["a","b","c"]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let num_deleted = client.json_forget("key", "$").await?;
    assert_eq!(1, num_deleted);

    let json: Option<String> = client.json_get("key", JsonGetOptions::default()).await?;
    assert_eq!(None, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_get() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":[1,2,3]},{"bar":[3,4,5]},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let json: String = client
        .json_get("key", JsonGetOptions::default().path("$.foo[*].bar"))
        .await?;
    assert_eq!("[[1,2,3],[3,4,5],12]", json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_mget() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key1",
            "$",
            r#"{"a":1, "b": 2, "nested": {"a": 3}, "c": null}"#,
            Default::default(),
        )
        .await?;

    client
        .json_set(
            "key2",
            "$",
            r#"{"a":4, "b": 5, "nested": {"a": 6}, "c": null}"#,
            Default::default(),
        )
        .await?;

    let jsons: SmallVec<[String; 2]> = client.json_mget(["key1", "key2"], "$..a").await?;
    assert_eq!(2, jsons.len());
    assert_eq!("[1,3]", jsons[0]);
    assert_eq!("[4,6]", jsons[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_numincrby() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":"b","b":[{"a":2}, {"a":5}, {"a":"c"}]}"#,
            Default::default(),
        )
        .await?;

    let response: Vec<Option<i32>> = client.json_numincrby("key", "$.a", 2).await?;
    assert_eq!(vec![None], response);

    let response: Vec<Option<i32>> = client.json_numincrby("key", "$..a", 2).await?;
    assert_eq!(vec![None, Some(4), Some(7), None], response);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_nummultby() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":"b","b":[{"a":2}, {"a":5}, {"a":"c"}]}"#,
            Default::default(),
        )
        .await?;

    let response: Vec<Option<i32>> = client.json_nummultby("key", "$.a", 2).await?;
    assert_eq!(vec![None], response);

    let response: Vec<Option<i32>> = client.json_nummultby("key", "$..a", 2).await?;
    assert_eq!(vec![None, Some(4), Some(10), None], response);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_objkeys() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":[3], "nested": {"a": {"b":2, "c": 1}}}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Vec<String>> = client.json_objkeys("key", "$..a").await?;
    assert_eq!(2, result.len());
    assert_eq!(0, result[0].len());
    assert_eq!(2, result[1].len());
    assert_eq!("b", result[1][0]);
    assert_eq!("c", result[1][1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_objlen() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":[3], "nested": {"a": {"b":2, "c": 1}}}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_objlen("key", "$..a").await?;
    assert_eq!(2, result.len());
    assert_eq!(None, result[0]);
    assert_eq!(Some(2), result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_resp() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"prop1":12,"prop2":"foo","prop3": true,"prop4":null,"prop5":["foo","bar"],"prop6":{"prop1": "foo", "prop2": 12}}"#,
            Default::default(),
        )
        .await?;

    let mut result: Vec<Value> = client.json_resp("key", "$").await?;
    assert_eq!(1, result.len());
    let values: Vec<Value> = result.pop().unwrap().into()?;
    assert_eq!(13, values.len());
    let mut iter = values.into_iter();
    assert_eq!("{", iter.next().unwrap().into::<String>()?);
    assert_eq!("prop1", iter.next().unwrap().into::<String>()?);
    assert_eq!(12, iter.next().unwrap().into::<i64>()?);
    assert_eq!("prop2", iter.next().unwrap().into::<String>()?);
    assert_eq!("foo", iter.next().unwrap().into::<String>()?);
    assert_eq!("prop3", iter.next().unwrap().into::<String>()?);
    assert_eq!("true", iter.next().unwrap().into::<String>()?);
    assert_eq!("prop4", iter.next().unwrap().into::<String>()?);
    assert_eq!("", iter.next().unwrap().into::<String>()?);
    assert_eq!("prop5", iter.next().unwrap().into::<String>()?);
    let prop5_values: Vec<Value> = iter.next().unwrap().into()?;
    assert_eq!(3, prop5_values.len());
    let mut iter_prop5 = prop5_values.into_iter();
    assert_eq!("[", iter_prop5.next().unwrap().into::<String>()?);
    assert_eq!("foo", iter_prop5.next().unwrap().into::<String>()?);
    assert_eq!("bar", iter_prop5.next().unwrap().into::<String>()?);
    assert_eq!("prop6", iter.next().unwrap().into::<String>()?);
    let prop6_values: Vec<Value> = iter.next().unwrap().into()?;
    assert_eq!(5, prop6_values.len());
    let mut iter_prop6 = prop6_values.into_iter();
    assert_eq!("{", iter_prop6.next().unwrap().into::<String>()?);
    assert_eq!("prop1", iter_prop6.next().unwrap().into::<String>()?);
    assert_eq!("foo", iter_prop6.next().unwrap().into::<String>()?);
    assert_eq!("prop2", iter_prop6.next().unwrap().into::<String>()?);
    assert_eq!(12, iter_prop6.next().unwrap().into::<i64>()?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_strappend() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":"foo", "nested": {"a": "hello"}, "nested2": {"a": 31}}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_strappend("key", "$..a", r#""baz""#).await?;
    assert_eq!(3, result.len());
    assert_eq!(Some(6), result[0]);
    assert_eq!(Some(8), result[1]);
    assert_eq!(None, result[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_strlen() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":"foo", "nested": {"a": "hello"}, "nested2": {"a": 31}}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_strlen("key", "$..a").await?;
    assert_eq!(3, result.len());
    assert_eq!(Some(3), result[0]);
    assert_eq!(Some(5), result[1]);
    assert_eq!(None, result[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_toggle() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"foo":[{"bar":true},{"bar":12}]}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<Option<usize>> = client.json_toggle("key", "$.foo[*].bar").await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(0), result[0]);
    assert_eq!(None, result[1]);

    let json: String = client.json_get("key", JsonGetOptions::default()).await?;
    assert_eq!(r#"{"foo":[{"bar":false},{"bar":12}]}"#, json);

    let result: Vec<Option<usize>> = client.json_toggle("key", "$.foo[*].bar").await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(1), result[0]);
    assert_eq!(None, result[1]);

    let json: String = client.json_get("key", JsonGetOptions::default()).await?;
    assert_eq!(r#"{"foo":[{"bar":true},{"bar":12}]}"#, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_type() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":2, "nested": {"a": true}, "foo": "bar"}"#,
            Default::default(),
        )
        .await?;

    let result: Vec<String> = client.json_type("key", ".foo").await?;
    assert_eq!(1, result.len());
    assert_eq!("string", result[0]);

    let result: Vec<Vec<String>> = client.json_type("key", "$..foo").await?;
    assert_eq!(1, result.len());
    assert_eq!(1, result[0].len());
    assert_eq!("string", result[0][0]);

    let result: Vec<Vec<String>> = client.json_type("key", "$..a").await?;
    assert_eq!(1, result.len());
    assert_eq!(2, result[0].len());
    assert_eq!("integer", result[0][0]);
    assert_eq!("boolean", result[0][1]);

    let result: Vec<Vec<String>> = client.json_type("key", "$..dummy").await?;
    assert_eq!(1, result.len());
    assert_eq!(0, result[0].len());

    Ok(())
}
