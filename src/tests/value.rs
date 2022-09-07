use crate::{
    resp::{Array, Value},
    tests::get_default_addr,
    ConnectionMultiplexer, GenericCommands, Result, SetCommands,
};
use serial_test::serial;
use std::collections::{BTreeSet, HashSet};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn from_single_value_array() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del("key").await?;

    database
        .sadd("key", ["member1", "member2", "member3"])
        .await?;

    let members: Vec<String> = database.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    let members: HashSet<String> = database.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    let members: BTreeSet<String> = database.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    Ok(())
}

#[test]
fn tuple() -> Result<()> {
    let value = Value::Array(Array::Vec(vec![
        Value::BulkString("first".into()),
        Value::BulkString("second".into()),
    ]));
    let result: Vec<String> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!("first".to_owned(), result[0]);
    assert_eq!("second".to_owned(), result[1]);

    let values = Value::Array(Array::Vec(vec![
        Value::BulkString("first".into()),
        Value::BulkString("second".into()),
    ]));
    let result: (String, String) = values.into()?;
    assert_eq!(("first".to_owned(), "second".to_owned()), result);

    let value = Value::Array(Array::Vec(vec![
        Value::BulkString("first".into()),
        Value::BulkString("second".into()),
        Value::BulkString("third".into()),
        Value::BulkString("fourth".into()),
    ]));
    let result: Vec<(String, String)> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!(("first".to_owned(), "second".to_owned()), result[0]);
    assert_eq!(("third".to_owned(), "fourth".to_owned()), result[1]);

    let value = Value::Array(Array::Vec(vec![
        Value::Array(Array::Vec(vec![
            Value::BulkString("first".into()),
            Value::BulkString("second".into()),
        ])),
        Value::Array(Array::Vec(vec![
            Value::BulkString("third".into()),
            Value::BulkString("fourth".into()),
        ])),
    ]));
    let result: Vec<(String, String)> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!(("first".to_owned(), "second".to_owned()), result[0]);
    assert_eq!(("third".to_owned(), "fourth".to_owned()), result[1]);

    Ok(())
}
