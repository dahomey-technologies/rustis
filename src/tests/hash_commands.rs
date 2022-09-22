use crate::{
    tests::get_default_addr, Connection, ConnectionCommandResult, GenericCommands, HScanOptions,
    HashCommands, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hdel() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.hset("key", ("field", "value")).send().await?;
    let value: String = connection.hget("key", "field").send().await?;
    assert_eq!("value", value);

    let len = connection.hdel("key", "field").send().await?;
    assert_eq!(1, len);

    let len = connection.hdel("key", "field").send().await?;
    assert_eq!(0, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hexists() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.hset("key", ("field", "value")).send().await?;
    let value: String = connection.hget("key", "field").send().await?;
    assert_eq!("value", value);

    let result = connection.hexists("key", "field").send().await?;
    assert!(result);

    let result = connection.hexists("key", "unknown").send().await?;
    assert!(!result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hget() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.hset("key", ("field", "value")).send().await?;
    let value: String = connection.hget("key", "field").send().await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hget_all() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .send()
        .await?;
    let result: Vec<(String, String)> = connection.hgetall("key").send().await?;
    assert_eq!(2, result.len());
    assert_eq!(("field1".to_owned(), "Hello".to_owned()), result[0]);
    assert_eq!(("field2".to_owned(), "World".to_owned()), result[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hincrby() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.hset("key", ("field", "5")).send().await?;
    let value = connection.hincrby("key", "field", 1).send().await?;
    assert_eq!(6, value);
    let value = connection.hincrby("key", "field", -1).send().await?;
    assert_eq!(5, value);
    let value = connection.hincrby("key", "field", -10).send().await?;
    assert_eq!(-5, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hincrbyfloat() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.hset("key", ("field", "10.50")).send().await?;
    let value = connection.hincrbyfloat("key", "field", 0.1).send().await?;
    assert_eq!(10.6, value);
    let value = connection.hincrbyfloat("key", "field", -5.0).send().await?;
    assert_eq!(5.6, value);
    connection.hset("key", ("field", "5.0e3")).send().await?;
    let value = connection
        .hincrbyfloat("key", "field", 2.0e2)
        .send()
        .await?;
    assert_eq!(5200.0, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hkeys() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .send()
        .await?;
    let fields: Vec<String> = connection.hkeys("key").send().await?;
    assert_eq!(2, fields.len());
    assert_eq!("field1".to_owned(), fields[0]);
    assert_eq!("field2".to_owned(), fields[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hlen() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .send()
        .await?;
    let len = connection.hlen("key").send().await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hmget() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .send()
        .await?;
    let values: Vec<String> = connection
        .hmget("key", ["field1", "field2", "nofield"])
        .send()
        .await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("coin").send().await?;

    let fields_and_values = [("heads", "obverse"), ("tails", "reverse"), ("edge", "")];
    connection.hset("coin", fields_and_values).send().await?;

    let value: String = connection.hrandfield("coin").send().await?;
    assert!(fields_and_values.iter().any(|v| v.0 == value));

    let values: Vec<String> = connection.hrandfields("coin", -5).send().await?;
    assert_eq!(5, values.len());
    for value in values {
        assert!(fields_and_values.iter().any(|v| v.0 == value));
    }

    let values: Vec<String> = connection.hrandfields("coin", 5).send().await?;
    assert_eq!(3, values.len());
    for value in values {
        assert!(fields_and_values.iter().any(|v| v.0 == value));
    }

    let values: Vec<(String, String)> =
        connection.hrandfields_with_values("coin", 5).send().await?;
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
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let fields_and_values: Vec<_> = (1..21)
        .map(|i| (format!("field{}", i), format!("value{}", i)))
        .collect();

    connection.hset("key", fields_and_values).send().await?;

    let result: (u64, Vec<(String, String)>) = connection
        .hscan("key", 0, HScanOptions::default().count(20))
        .send()
        .await?;

    //println!("{:?}", result);
    assert_eq!(0, result.0);
    assert_eq!(20, result.1.len());
    assert_eq!(("field1".to_owned(), "value1".to_owned()), result.1[0]);
    assert_eq!(("field2".to_owned(), "value2".to_owned()), result.1[1]);
    assert_eq!(("field3".to_owned(), "value3".to_owned()), result.1[2]);
    assert_eq!(("field4".to_owned(), "value4".to_owned()), result.1[3]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hsetnx() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    let result = connection.hsetnx("key", "field", "Hello").send().await?;
    assert!(result);

    let result = connection.hsetnx("key", "field", "World").send().await?;
    assert!(!result);

    let value: String = connection.hget("key", "field").send().await?;
    assert_eq!("Hello", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hstrlen() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection.hset("key", ("field", "value")).send().await?;

    let len = connection.hstrlen("key", "field").send().await?;
    assert_eq!(5, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hvals() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.del("key").send().await?;

    connection
        .hset("key", [("field1", "Hello"), ("field2", "World")])
        .send()
        .await?;

    let values: Vec<String> = connection.hvals("key").send().await?;
    assert_eq!(2, values.len());
    assert_eq!("Hello", values[0]);
    assert_eq!("World", values[1]);

    Ok(())
}
