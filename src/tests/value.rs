use crate::{
    RedisError, RedisErrorKind, Result,
    commands::{FlushingMode, ServerCommands, SetCommands},
    resp::Value,
    tests::{get_test_client, log_try_init},
};
use serial_test::serial;
use std::collections::{BTreeSet, HashMap, HashSet};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn from_single_value_array() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .sadd("key", ["member1", "member2", "member3"])
        .await?;

    let members: Vec<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    let members: HashSet<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains("member1"));
    assert!(members.contains("member2"));
    assert!(members.contains("member3"));

    let members: BTreeSet<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains("member1"));
    assert!(members.contains("member2"));
    assert!(members.contains("member3"));

    Ok(())
}

#[test]
fn tuple() -> Result<()> {
    log_try_init();

    let value = Value::Array(vec![
        Value::BulkString("first".as_bytes().to_vec()),
        Value::BulkString("second".as_bytes().to_vec()),
    ]);
    let result: Vec<String> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!("first".to_owned(), result[0]);
    assert_eq!("second".to_owned(), result[1]);

    let values = Value::Array(vec![
        Value::BulkString("first".as_bytes().to_vec()),
        Value::BulkString("second".as_bytes().to_vec()),
    ]);
    let result: (String, String) = values.into()?;
    assert_eq!(("first".to_owned(), "second".to_owned()), result);

    let value = Value::Array(vec![
        Value::Array(vec![
            Value::BulkString("first".as_bytes().to_vec()),
            Value::BulkString("second".as_bytes().to_vec()),
        ]),
        Value::Array(vec![
            Value::BulkString("third".as_bytes().to_vec()),
            Value::BulkString("fourth".as_bytes().to_vec()),
        ]),
    ]);
    let result: Vec<(String, String)> = value.into()?;
    assert_eq!(2, result.len());
    assert_eq!(("first".to_owned(), "second".to_owned()), result[0]);
    assert_eq!(("third".to_owned(), "fourth".to_owned()), result[1]);

    Ok(())
}

#[test]
fn display() {
    log_try_init();

    log::debug!(
        "{}",
        Value::Array(vec![
            Value::Integer(12),
            Value::Double(12.12),
            Value::SimpleString("OK".to_owned()),
            Value::BulkString(b"mystring".to_vec()),
            Value::Boolean(true),
            Value::Error(RedisError {
                kind: RedisErrorKind::Err,
                description: "MyError".to_owned()
            }),
            Value::Nil,
            Value::Map(HashMap::from([
                (Value::BulkString(b"field1".to_vec()), Value::Integer(12)),
                (Value::BulkString(b"field2".to_vec()), Value::Double(12.12))
            ]))
        ])
    );
}
