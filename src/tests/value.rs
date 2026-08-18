use crate::{
    RedisError, RedisErrorKind, Result,
    commands::{FlushingMode, ServerCommands, SetCommands},
    resp::Value,
    tests::{get_test_client, log_try_init},
};
use serial_test::serial;
use std::collections::{BTreeSet, HashSet};

#[tokio::test]
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

    // Bound first on purpose: a bare path passed positionally to a `tracing`
    // macro is parsed as a field name, not as a value to format.
    let value = Value::Array(vec![
        Value::Integer(12),
        Value::Double(12.12),
        Value::SimpleString("OK".to_owned()),
        Value::BulkString(b"mystring".to_vec()),
        Value::Boolean(true),
        Value::Error(RedisError {
            kind: RedisErrorKind::Err,
            description: "MyError".to_owned(),
        }),
        Value::Null,
        Value::Map(Vec::from([
            (Value::BulkString(b"field1".to_vec()), Value::Integer(12)),
            (Value::BulkString(b"field2".to_vec()), Value::Double(12.12)),
        ])),
    ]);

    tracing::debug!("{value}");
}

#[test]
fn double_equality() {
    // `Value` asserts `Eq`, so double equality is reflexive: a `,nan` reply
    // equals itself.
    assert_eq!(Value::Double(f64::NAN), Value::Double(f64::NAN));
    assert_eq!(
        Value::Array(vec![Value::Double(f64::NAN)]),
        Value::Array(vec![Value::Double(f64::NAN)])
    );

    // Every NaN payload collapses onto the same value.
    let other_nan = Value::Double(f64::from_bits(f64::NAN.to_bits() | 1));
    assert_eq!(Value::Double(f64::NAN), other_nan);

    // The two zeros are equal.
    assert_eq!(Value::Double(0.0), Value::Double(-0.0));

    // Distinct doubles stay distinct.
    assert_ne!(Value::Double(1.0), Value::Double(2.0));
    assert_ne!(Value::Double(f64::NAN), Value::Double(1.0));
}

/// A map keeps the entries it was given, in order, and a lookup answers the
/// first entry carrying that field.
#[test]
fn a_map_is_read_as_a_sequence_of_entries() {
    let map = Value::Map(vec![
        (Value::SimpleString("a".to_owned()), Value::Integer(1)),
        (Value::SimpleString("b".to_owned()), Value::Integer(2)),
        (Value::SimpleString("a".to_owned()), Value::Integer(3)),
    ]);

    assert_eq!(3, map.as_map().expect("a map").len());
    assert_eq!(
        Some(&Value::Integer(1)),
        map.get(&Value::SimpleString("a".to_owned()))
    );
    assert_eq!(None, map.get(&Value::SimpleString("c".to_owned())));
    assert_eq!(None, Value::Integer(1).as_map());
}

#[test]
fn boolean_equality() {
    // Two booleans must compare on their inner value, not merely on the variant.
    assert_eq!(Value::Boolean(true), Value::Boolean(true));
    assert_eq!(Value::Boolean(false), Value::Boolean(false));
    assert_ne!(Value::Boolean(true), Value::Boolean(false));
    assert_ne!(Value::Boolean(false), Value::Boolean(true));
}
