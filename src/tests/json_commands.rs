use crate::{
    Result,
    commands::{
        FlushingMode, JsonArrIndexOptions, JsonCommands, JsonFpType, JsonGetFormat, JsonGetOptions,
        JsonSetOptions, ServerCommands, SetCondition,
    },
    resp::Value,
    tests::{TestClient, get_test_client},
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
        )
        .await?;

    let result: Vec<usize> = client.json_debug_memory("key", "$.foo[*].bar").await?;
    assert_eq!(2, result.len());
    // The exact byte counts belong to the JSON module allocator. Only their
    // ordering is stable: a three-string array outweighs a single number.
    assert!(result[0] > result[1]);
    assert!(result[1] > 0);

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
            None,
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
            None,
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
            None,
        )
        .await?;

    let json: String = client
        .json_get("key", JsonGetOptions::default().path("$.foo[*].bar"))
        .await?;
    assert_eq!("[[1,2,3],[3,4,5],12]", json);

    // `STRING` is the default: the whole reply as one serialized document.
    let json: String = client
        .json_get(
            "key",
            JsonGetOptions::default()
                .path("$.foo[*].bar")
                .format(JsonGetFormat::String),
        )
        .await?;
    assert_eq!("[[1,2,3],[3,4,5],12]", json);

    // `EXPAND1` groups the matches per path and serializes only the containers,
    // leaving scalars native — hence the mixed element types.
    let values: Vec<Vec<Value>> = client
        .json_get(
            "key",
            JsonGetOptions::default()
                .path("$.foo[*].bar")
                .format(JsonGetFormat::Expand1),
        )
        .await?;
    assert_eq!(1, values.len());
    assert_eq!(3, values[0].len());
    assert_eq!("[1,2,3]", values[0][0].to_string());
    assert_eq!(Value::Integer(12), values[0][2]);

    // `EXPAND` goes all the way down to native RESP3 values.
    let values: Vec<Vec<Value>> = client
        .json_get(
            "key",
            JsonGetOptions::default()
                .path("$.foo[*].bar")
                .format(JsonGetFormat::Expand),
        )
        .await?;
    assert_eq!(1, values.len());
    assert_eq!(
        Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3)
        ]),
        values[0][0]
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_set_fpha() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // FPHA forces the storage type of floating-point homogeneous arrays; the
    // integers come back as floats because the array is stored as FP16.
    client
        .json_set(
            "key",
            "$",
            "[[1,2,3,4e3],[5,6.0,7,8]]",
            JsonSetOptions::default().fpha(JsonFpType::Fp16),
        )
        .await?;
    let result: String = client.json_get("key", JsonGetOptions::default()).await?;
    assert_eq!("[[1.0,2.0,3.0,4000.0],[5.0,6.0,7.0,8.0]]", result);

    // A value that does not fit the requested type is rejected.
    let result = client
        .json_set(
            "key2",
            "$",
            "[1e40]",
            JsonSetOptions::default().fpha(JsonFpType::Fp16),
        )
        .await;
    assert!(result.is_err());

    Ok(())
}

#[test]
fn json_set_args() {
    let cmd = TestClient
        .json_set(
            "key",
            "$",
            "[1.0]",
            JsonSetOptions::default()
                .condition(SetCondition::NX)
                .fpha(JsonFpType::Bf16),
        )
        .command;
    assert_eq!("JSON.SET key $ [1.0] NX FPHA BF16", cmd.to_string());

    let cmd = TestClient.json_set("key", "$", "[1.0]", None).command;
    assert_eq!("JSON.SET key $ [1.0]", cmd.to_string());
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_merge() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "key",
            "$",
            r#"{"a":2, "b":3, "nested":{"a":4, "b":5}}"#,
            None,
        )
        .await?;

    // A merge updates the keys it names and leaves the others alone. Assertions
    // go through paths rather than the whole document, whose key order the
    // server does not preserve across a delete.
    client.json_merge("key", "$", r#"{"b":8}"#).await?;
    let json: String = client
        .json_get("key", JsonGetOptions::default().path("$.b"))
        .await?;
    assert_eq!("[8]", json);
    let json: String = client
        .json_get("key", JsonGetOptions::default().path("$.a"))
        .await?;
    assert_eq!("[2]", json);

    // A null value deletes the key, which is what makes MERGE more than a set.
    client.json_merge("key", "$", r#"{"a":null}"#).await?;
    let json: String = client
        .json_get("key", JsonGetOptions::default().path("$.a"))
        .await?;
    assert_eq!("[]", json);

    // The path can target a nested document, merging in place.
    client.json_merge("key", "$.nested", r#"{"c":6}"#).await?;
    let json: String = client
        .json_get("key", JsonGetOptions::default().path("$.nested"))
        .await?;
    assert_eq!(r#"[{"a":4,"b":5,"c":6}]"#, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_mset() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // JSON.MSET takes key/path/value triplets and creates the documents in one
    // atomic call.
    client
        .json_mset([
            ("key1", "$", r#"{"a":1, "nested": {"a": 3}}"#),
            ("key2", "$", r#"{"a":4, "nested": {"a": 6}}"#),
        ])
        .await?;

    let jsons: SmallVec<[String; 2]> = client.json_mget(["key1", "key2"], "$..a").await?;
    assert_eq!(2, jsons.len());
    assert_eq!("[1,3]", jsons[0]);
    assert_eq!("[4,6]", jsons[1]);

    // A path inside an existing document updates just that member.
    client.json_mset([("key1", "$.a", "2")]).await?;

    let json: String = client.json_get("key1", JsonGetOptions::default()).await?;
    assert_eq!(r#"{"a":2,"nested":{"a":3}}"#, json);

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
            None,
        )
        .await?;

    client
        .json_set(
            "key2",
            "$",
            r#"{"a":4, "b": 5, "nested": {"a": 6}, "c": null}"#,
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
        .json_set("key", "$", r#"{"foo":[{"bar":true},{"bar":12}]}"#, None)
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
            None,
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

/// `JSON.GET key [INDENT indent] [NEWLINE newline] [SPACE space] [path ...]`.
/// The three formatting options are the server's own pretty-printer: INDENT per
/// nesting level, NEWLINE at each line end, SPACE between a key and its value.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_get_formatting() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set("key", "$", r#"{"a":1,"b":2}"#, None)
        .await?;

    let json: String = client
        .json_get(
            "key",
            JsonGetOptions::default()
                .indent("--")
                .newline("|")
                .space("_")
                .path("$.a"),
        )
        .await?;
    assert_eq!("[|--1|]", json);

    // Each option is independently visible: dropping NEWLINE drops the line
    // breaks but keeps nothing else.
    let json: String = client
        .json_get(
            "key",
            JsonGetOptions::default().indent("--").space("_").path("$"),
        )
        .await?;
    assert_eq!(r#"[--{----"a":_1,----"b":_2--}]"#, json);

    Ok(())
}

/// `JSON.ARRINDEX key path value [start [stop]]`. start and stop are positional
/// and slice the searched range; stop is exclusive except that 0 means "to the
/// end".
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn json_arrindex_stop() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set("key", "$", r#"{"b":[1,2,3,4,5]}"#, None)
        .await?;

    // 4 sits at index 3, inside [0, 4).
    let result: Vec<Option<isize>> = client
        .json_arrindex(
            "key",
            "$.b",
            "4",
            JsonArrIndexOptions::default().start(0).stop(4),
        )
        .await?;
    assert_eq!(vec![Some(3)], result);

    // Outside [0, 3) it is not found.
    let result: Vec<Option<isize>> = client
        .json_arrindex(
            "key",
            "$.b",
            "4",
            JsonArrIndexOptions::default().start(0).stop(3),
        )
        .await?;
    assert_eq!(vec![Some(-1)], result);

    // A negative stop counts from the end and stays exclusive, so -1 drops the
    // last element; stop 0 is the one value that means "to the end".
    let result: Vec<Option<isize>> = client
        .json_arrindex(
            "key",
            "$.b",
            "5",
            JsonArrIndexOptions::default().start(0).stop(-1),
        )
        .await?;
    assert_eq!(vec![Some(-1)], result);

    let result: Vec<Option<isize>> = client
        .json_arrindex(
            "key",
            "$.b",
            "5",
            JsonArrIndexOptions::default().start(0).stop(0),
        )
        .await?;
    assert_eq!(vec![Some(4)], result);

    Ok(())
}
