use std::collections::HashMap;

use crate::{
    commands::{GenericCommands, HScanOptions, HScanResult, HashCommands},
    tests::get_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hdel() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let len = client.hdel("key", "field").await?;
    assert_eq!(1, len);

    let len = client.hdel("key", "field").await?;
    assert_eq!(0, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hexists() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    let result = client.hexists("key", "field").await?;
    assert!(result);

    let result = client.hexists("key", "unknown").await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hget() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;
    let value: String = client.hget("key", "field").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hget_all() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let result: HashMap<String, String> = client.hgetall("key").await?;
    assert_eq!(2, result.len());
    assert_eq!(Some(&"Hello".to_owned()), result.get("field1"));
    assert_eq!(Some(&"World".to_owned()), result.get("field2"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hincrby() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "5")).await?;
    let value = client.hincrby("key", "field", 1).await?;
    assert_eq!(6, value);
    let value = client.hincrby("key", "field", -1).await?;
    assert_eq!(5, value);
    let value = client.hincrby("key", "field", -10).await?;
    assert_eq!(-5, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hincrbyfloat() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "10.50")).await?;
    let value = client.hincrbyfloat("key", "field", 0.1).await?;
    assert_eq!(10.6, value);
    let value = client.hincrbyfloat("key", "field", -5.0).await?;
    assert_eq!(5.6, value);
    client.hset("key", ("field", "5.0e3")).await?;
    let value = client.hincrbyfloat("key", "field", 2.0e2).await?;
    assert_eq!(5200.0, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hkeys() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let fields: Vec<String> = client.hkeys("key").await?;
    assert_eq!(2, fields.len());
    assert_eq!("field1".to_owned(), fields[0]);
    assert_eq!("field2".to_owned(), fields[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hlen() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let len = client.hlen("key").await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hmget() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;
    let values: Vec<String> = client.hmget("key", ["field1", "field2", "nofield"]).await?;
    assert_eq!(3, values.len());
    assert_eq!("Hello".to_owned(), values[0]);
    assert_eq!("World".to_owned(), values[1]);
    assert_eq!("".to_owned(), values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hrandfield() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("coin").await?;

    let fields_and_values = [("heads", "obverse"), ("tails", "reverse"), ("edge", "")];
    client.hset("coin", fields_and_values).await?;

    let value: String = client.hrandfield("coin").await?;
    assert!(fields_and_values.iter().any(|v| v.0 == value));

    let values: Vec<String> = client.hrandfields("coin", -5).await?;
    assert_eq!(5, values.len());
    for value in values {
        assert!(fields_and_values.iter().any(|v| v.0 == value));
    }

    let values: Vec<String> = client.hrandfields("coin", 5).await?;
    assert_eq!(3, values.len());
    for value in values {
        assert!(fields_and_values.iter().any(|v| v.0 == value));
    }

    let values: Vec<(String, String)> = client.hrandfields_with_values("coin", 5).await?;
    assert_eq!(3, values.len());
    for value in values {
        assert!(fields_and_values
            .iter()
            .any(|v| v.0 == value.0 && v.1 == value.1));
    }

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hscan() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let fields_and_values: Vec<_> = (1..21)
        .map(|i| (format!("field{}", i), format!("value{}", i)))
        .collect();

    client.hset("key", fields_and_values).await?;

    let result: HScanResult<String, String> = client
        .hscan("key", 0, HScanOptions::default().count(20))
        .await?;

    assert_eq!(0, result.cursor);
    assert_eq!(20, result.elements.len());
    assert_eq!(
        ("field1".to_owned(), "value1".to_owned()),
        result.elements[0]
    );
    assert_eq!(
        ("field2".to_owned(), "value2".to_owned()),
        result.elements[1]
    );
    assert_eq!(
        ("field3".to_owned(), "value3".to_owned()),
        result.elements[2]
    );
    assert_eq!(
        ("field4".to_owned(), "value4".to_owned()),
        result.elements[3]
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hsetnx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    let result = client.hsetnx("key", "field", "Hello").await?;
    assert!(result);

    let result = client.hsetnx("key", "field", "World").await?;
    assert!(!result);

    let value: String = client.hget("key", "field").await?;
    assert_eq!("Hello", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hstrlen() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client.hset("key", ("field", "value")).await?;

    let len = client.hstrlen("key", "field").await?;
    assert_eq!(5, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hvals() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").await?;

    client
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .await?;

    let values: Vec<String> = client.hvals("key").await?;
    assert_eq!(2, values.len());
    assert_eq!("Hello", values[0]);
    assert_eq!("World", values[1]);

    Ok(())
}
