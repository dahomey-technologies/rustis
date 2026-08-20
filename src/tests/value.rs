use crate::{RedisError, RedisErrorKind, Result, resp::Value, tests::log_try_init};
use bytes::Bytes;

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
            description: Bytes::from_static(b"MyError"),
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

/// The two string variants carry the same payload and the deserializer reads
/// them identically, so which one a reply arrives in is a server-version
/// detail — `+OK` from one release, `$2\r\nOK` from the next. Comparing them
/// on the variant would make caller code version-fragile for no gain.
#[test]
fn a_simple_string_equals_the_same_bulk_string() {
    assert_eq!(
        Value::SimpleString("OK".to_owned()),
        Value::BulkString(b"OK".to_vec())
    );
    assert_eq!(
        Value::BulkString(b"OK".to_vec()),
        Value::SimpleString("OK".to_owned())
    );

    // Nesting must follow, since it compares elementwise.
    assert_eq!(
        Value::Array(vec![Value::SimpleString("OK".to_owned())]),
        Value::Array(vec![Value::BulkString(b"OK".to_vec())])
    );

    // Different payloads stay different, and a non-UTF-8 bulk string equals no
    // simple string at all.
    assert_ne!(
        Value::SimpleString("OK".to_owned()),
        Value::BulkString(b"KO".to_vec())
    );
    assert_ne!(
        Value::SimpleString("\u{fffd}".to_owned()),
        Value::BulkString(vec![0xff])
    );
}

/// `Value` is what a caller gets when it does not model the reply shape. It
/// must therefore be readable without `serde`: one accessor per variant, each
/// answering `None` when the variant does not match.
#[test]
fn a_value_can_be_read_without_serde() {
    // Both string variants read as text, since they mean the same thing.
    assert_eq!(Some("OK"), Value::SimpleString("OK".to_owned()).as_str());
    assert_eq!(Some("OK"), Value::BulkString(b"OK".to_vec()).as_str());
    assert_eq!(None, Value::BulkString(vec![0xff]).as_str());
    assert_eq!(
        Some(b"OK".as_slice()),
        Value::SimpleString("OK".to_owned()).as_bytes()
    );

    assert_eq!(Some(12), Value::Integer(12).as_i64());
    assert_eq!(None, Value::SimpleString("12".to_owned()).as_i64());
    assert_eq!(Some(12.5), Value::Double(12.5).as_f64());
    assert_eq!(Some(true), Value::Boolean(true).as_bool());

    assert!(Value::Null.is_null());
    assert!(!Value::Integer(0).is_null());

    let array = Value::Array(vec![Value::Integer(1), Value::Integer(2)]);
    assert_eq!(2, array.as_array().expect("an array").len());
    assert_eq!(None, array.as_map());

    let map = Value::Map(vec![
        (Value::SimpleString("a".to_owned()), Value::Integer(1)),
        (Value::SimpleString("b".to_owned()), Value::Integer(2)),
    ]);
    assert_eq!(2, map.as_map().expect("a map").len());
    // A lookup answers the first entry with that field, and reads a bulk-string
    // field against a simple-string key.
    assert_eq!(
        Some(&Value::Integer(1)),
        map.get(&Value::BulkString(b"a".to_vec()))
    );
    assert_eq!(None, map.get(&Value::SimpleString("c".to_owned())));

    let error = Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"MyError"),
    });
    assert_eq!(
        Some(&RedisErrorKind::Err),
        error.as_error().map(|e| &e.kind)
    );
}

#[test]
fn boolean_equality() {
    // Two booleans must compare on their inner value, not merely on the variant.
    assert_eq!(Value::Boolean(true), Value::Boolean(true));
    assert_eq!(Value::Boolean(false), Value::Boolean(false));
    assert_ne!(Value::Boolean(true), Value::Boolean(false));
    assert_ne!(Value::Boolean(false), Value::Boolean(true));
}
