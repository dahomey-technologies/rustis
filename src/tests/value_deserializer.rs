use std::collections::HashMap;

use crate::{
    ClientError, ErrorKind, RedisError, RedisErrorKind, Result, resp::Value, tests::log_try_init,
};
use bytes::Bytes;
use serde::Deserialize;
use smallvec::SmallVec;

#[test]
fn bool() -> Result<()> {
    log_try_init();

    let result = bool::deserialize(&Value::Boolean(true))?;
    assert!(result);

    let result = bool::deserialize(&Value::Boolean(false))?;
    assert!(!result);

    let result = bool::deserialize(&Value::Integer(1))?;
    assert!(result);

    let result = bool::deserialize(&Value::Integer(0))?;
    assert!(!result);

    let result = bool::deserialize(&Value::Double(1.))?;
    assert!(result);

    let result = bool::deserialize(&Value::Double(0.))?;
    assert!(!result);

    let result = bool::deserialize(&Value::SimpleString("OK".to_owned()))?;
    assert!(result);

    let result = bool::deserialize(&Value::BulkString(b"1".to_vec()))?;
    assert!(result);

    let result = bool::deserialize(&Value::BulkString(b"0".to_vec()))?;
    assert!(!result);

    let result = bool::deserialize(&Value::BulkString(b"true".to_vec()))?;
    assert!(result);

    let result = bool::deserialize(&Value::BulkString(b"false".to_vec()))?;
    assert!(!result);

    let result = bool::deserialize(&Value::Null)?;
    assert!(!result);

    // A simple string and a bulk string carrying the same text read the same
    // way, exactly as they do on the wire path.
    let result = bool::deserialize(&Value::BulkString(b"OK".to_vec()))?;
    assert!(result);

    let result = bool::deserialize(&Value::SimpleString("1".to_owned()))?;
    assert!(result);

    let result = bool::deserialize(&Value::SimpleString("true".to_owned()))?;
    assert!(result);

    let result = bool::deserialize(&Value::SimpleString("0".to_owned()))?;
    assert!(!result);

    let result = bool::deserialize(&Value::SimpleString("false".to_owned()))?;
    assert!(!result);

    // Text the rule does not recognize is an error on both paths.
    for unreadable in [
        Value::SimpleString("KO".to_owned()),
        Value::BulkString(b"hello".to_vec()),
        Value::BulkString(Vec::new()),
    ] {
        let result = bool::deserialize(&unreadable);
        let error = result.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                ErrorKind::Client(ClientError::CannotParseBoolean)
            ),
            "{unreadable:?} read as a bool"
        );
    }

    Ok(())
}

#[test]
fn i64() -> Result<()> {
    log_try_init();

    let result = i64::deserialize(&Value::Integer(12))?;
    assert_eq!(12, result);

    let result = i64::deserialize(&Value::Double(12.))?;
    assert_eq!(12, result);

    let result = i64::deserialize(&Value::SimpleString("12".to_owned()))?;
    assert_eq!(12, result);

    let result = i64::deserialize(&Value::BulkString(b"12".to_vec()))?;
    assert_eq!(12, result);

    // A nil carries an absence, which i64 cannot hold.
    let error = i64::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    let result = i64::deserialize(&Value::Array(vec![Value::Integer(12)]))?;
    assert_eq!(12, result);

    Ok(())
}

#[test]
fn u64() -> Result<()> {
    log_try_init();

    let result = u64::deserialize(&Value::Integer(12))?;
    assert_eq!(12, result);

    let result = u64::deserialize(&Value::Double(12.))?;
    assert_eq!(12, result);

    let result = u64::deserialize(&Value::SimpleString("12".to_owned()))?;
    assert_eq!(12, result);

    let result = u64::deserialize(&Value::BulkString(b"12".to_vec()))?;
    assert_eq!(12, result);

    // A nil carries an absence, which u64 cannot hold.
    let error = u64::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    let result = u64::deserialize(&Value::Array(vec![Value::Integer(12)]))?;
    assert_eq!(12, result);

    Ok(())
}

#[test]
fn f32() -> Result<()> {
    log_try_init();

    let result = f32::deserialize(&Value::Integer(12))?;
    assert_eq!(12., result);

    let result = f32::deserialize(&Value::Double(12.12))?;
    assert_eq!(12.12, result);

    let result = f32::deserialize(&Value::SimpleString("12.12".to_owned()))?;
    assert_eq!(12.12, result);

    let result = f32::deserialize(&Value::BulkString(b"12.12".to_vec()))?;
    assert_eq!(12.12, result);

    // A nil carries an absence, which f32 cannot hold.
    let error = f32::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

#[test]
fn f64() -> Result<()> {
    log_try_init();

    let result = f64::deserialize(&Value::Integer(12))?;
    assert_eq!(12., result);

    let result = f64::deserialize(&Value::Double(12.12))?;
    assert_eq!(12.12, result);

    let result = f64::deserialize(&Value::SimpleString("12.12".to_owned()))?;
    assert_eq!(12.12, result);

    let result = f64::deserialize(&Value::BulkString(b"12.12".to_vec()))?;
    assert_eq!(12.12, result);

    // A nil carries an absence, which f64 cannot hold.
    let error = f64::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

#[test]
fn char() -> Result<()> {
    log_try_init();

    let result = char::deserialize(&Value::SimpleString("a".to_owned()))?;
    assert_eq!('a', result);

    let result = char::deserialize(&Value::BulkString(b"a".to_vec()))?;
    assert_eq!('a', result);

    // A nil carries an absence, which char cannot hold.
    let error = char::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

#[test]
fn str() -> Result<()> {
    log_try_init();

    let value = Value::SimpleString("foo".to_owned());
    let result = <&str>::deserialize(&value)?;
    assert_eq!("foo", result);

    let value = Value::BulkString(b"foo".to_vec());
    let result = <&str>::deserialize(&value)?;
    assert_eq!("foo", result);

    // A nil carries an absence, which <&str> cannot hold.
    let error = <&str>::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

#[test]
fn string() -> Result<()> {
    log_try_init();

    let result = String::deserialize(&Value::SimpleString("foo".to_owned()))?;
    assert_eq!("foo", result);

    let result = String::deserialize(&Value::BulkString(b"foo".to_vec()))?;
    assert_eq!("foo", result);

    let result = String::deserialize(&Value::Double(12.))?;
    assert_eq!("12", result);

    // A nil carries an absence, which String cannot hold.
    let error = String::deserialize(&Value::Null).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

#[test]
fn option() -> Result<()> {
    log_try_init();

    let result = Option::<String>::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result = Option::<String>::deserialize(&Value::BulkString(b"hello".to_vec()))?;
    assert_eq!(Some("hello".to_owned()), result);

    let result = Option::<String>::deserialize(&Value::Null)?;
    assert_eq!(None, result);

    let result = Option::<i64>::deserialize(&Value::Integer(12))?;
    assert_eq!(Some(12), result);

    let result = Option::<i64>::deserialize(&Value::Null)?;
    assert_eq!(None, result);

    let result = Option::<Vec<i32>>::deserialize(&Value::Array(vec![Value::Integer(12)]))?;
    assert_eq!(Some(vec![12]), result);

    // An empty collection is a collection, not a nil: only `Value::Null` is `None`.
    let result = Option::<Vec<i32>>::deserialize(&Value::Array(vec![]))?;
    assert_eq!(Some(vec![]), result);

    let result = Option::<HashMap<String, i32>>::deserialize(&Value::Map(Vec::new()))?;
    assert_eq!(Some(HashMap::new()), result);

    let result = Option::<Vec<i32>>::deserialize(&Value::Set(vec![]))?;
    assert_eq!(Some(vec![]), result);

    Ok(())
}

#[test]
fn unit() -> Result<()> {
    log_try_init();

    let result = <()>::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result = <()>::deserialize(&Value::Null);
    assert!(result.is_ok());

    let result = <()>::deserialize(&Value::BulkString(b"hello".to_vec()));
    assert!(result.is_err());

    let result = <()>::deserialize(&Value::Integer(1));
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn unit_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize, Debug)]
    struct Unit;

    let result = Unit::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result = Unit::deserialize(&Value::Null);
    assert!(result.is_ok());

    let result = Unit::deserialize(&Value::BulkString(b"hello".to_vec()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn newtype_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize, Debug)]
    struct Millimeters(u8);

    let result = Millimeters::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result = Millimeters::deserialize(&Value::Integer(12))?;
    assert_eq!(12, result.0);

    Ok(())
}

#[test]
fn seq() -> Result<()> {
    log_try_init();

    let result = Vec::<i32>::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result =
        Vec::<i32>::deserialize(&Value::Array(vec![Value::Integer(12), Value::Integer(13)]))?;
    assert_eq!(2, result.len());
    assert_eq!(12, result[0]);
    assert_eq!(13, result[1]);

    let result = SmallVec::<[String; 2]>::deserialize(&Value::Array(vec![
        Value::BulkString(b"hello".to_vec()),
        Value::BulkString(b"world".to_vec()),
    ]))?;
    assert_eq!(2, result.len());
    assert_eq!("hello", result[0]);
    assert_eq!("world", result[1]);

    Ok(())
}

#[test]
fn tuple() -> Result<()> {
    log_try_init();

    let result = <(i32, i32)>::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));

    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result = <(i32, i32, i32)>::deserialize(&Value::Array(vec![
        Value::Integer(12),
        Value::Integer(13),
        Value::Integer(14),
    ]))?;
    assert_eq!((12, 13, 14), result);

    let result = <(String, String)>::deserialize(&Value::Array(vec![
        Value::BulkString(b"hello".to_vec()),
        Value::BulkString(b"world".to_vec()),
    ]))?;
    assert_eq!(("hello".to_owned(), "world".to_owned()), result);

    Ok(())
}

#[test]
fn tuple_struct() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Rgb(u8, u8, u8);

    let result = Rgb::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));

    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    let result = Rgb::deserialize(&Value::Array(vec![
        Value::Integer(12),
        Value::Integer(13),
        Value::Integer(14),
    ]))?;
    assert_eq!(Rgb(12, 13, 14), result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result = HashMap::<i32, i32>::deserialize(&Value::Map(Vec::from([
        (Value::Integer(12), Value::Integer(13)),
        (Value::Integer(14), Value::Integer(15)),
    ])))?;
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result = HashMap::<i32, i32>::deserialize(&Value::Array(vec![
        Value::Integer(12),
        Value::Integer(13),
        Value::Integer(14),
        Value::Integer(15),
    ]))?;
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result = HashMap::<i32, i32>::deserialize(&Value::Array(vec![
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
        Value::Array(vec![Value::Integer(14), Value::Integer(15)]),
    ]))?;
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result = HashMap::<String, Vec<String>>::deserialize(&Value::Array(vec![
        Value::Array(vec![
            Value::BulkString(b"a".to_vec()),
            Value::Set(vec![
                Value::SimpleString("OW".to_owned()),
                Value::SimpleString("update".to_owned()),
            ]),
        ]),
        Value::Array(vec![
            Value::BulkString(b"b".to_vec()),
            Value::Set(vec![
                Value::SimpleString("OW".to_owned()),
                Value::SimpleString("update".to_owned()),
            ]),
        ]),
    ]))?;
    assert_eq!(
        Some(&vec!["OW".to_owned(), "update".to_owned()]),
        result.get("a")
    );
    assert_eq!(
        Some(&vec!["OW".to_owned(), "update".to_owned()]),
        result.get("a")
    );

    let result = HashMap::<String, usize>::deserialize(&Value::Array(vec![
        Value::BulkString(b"mychannel1".to_vec()),
        Value::Integer(1),
        Value::BulkString(b"mychannel2".to_vec()),
        Value::Integer(2),
    ]))?;
    assert_eq!(2, result.len());
    assert_eq!(Some(&1usize), result.get("mychannel1"));
    assert_eq!(Some(&2usize), result.get("mychannel2"));

    Ok(())
}

#[test]
fn _struct() -> Result<()> {
    #[derive(Debug, Deserialize)]
    pub(crate) struct Person {
        pub id: u64,
        pub name: String,
    }

    log_try_init();

    let value = Value::Map(Vec::from([
        (Value::BulkString(b"id".to_vec()), Value::Integer(12)),
        (
            Value::BulkString(b"name".to_vec()),
            Value::BulkString(b"foo".to_vec()),
        ),
    ]));

    let result = Person::deserialize(&value)?;
    assert_eq!(12, result.id);
    assert_eq!("foo", result.name);

    let value = Value::Array(vec![
        Value::BulkString(b"id".to_vec()),
        Value::Integer(12),
        Value::BulkString(b"name".to_vec()),
        Value::BulkString(b"foo".to_vec()),
    ]);

    let result = Person::deserialize(&value)?;
    assert_eq!(12, result.id);
    assert_eq!("foo", result.name);

    let value = Value::Array(vec![Value::Integer(12), Value::BulkString(b"foo".to_vec())]);

    let result = Person::deserialize(&value)?;
    assert_eq!(12, result.id);
    assert_eq!("foo", result.name);

    Ok(())
}

/// The mirror of `resp_deserializer::struct_flat_array_shapes`: the same flat
/// arrays must decode the same way through the materialized `Value` tree.
#[test]
fn struct_flat_array_shapes() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Person {
        id: u64,
        name: String,
    }

    /// Two required fields and two optional ones, to exercise a server that
    /// answers fewer field/value pairs than the struct declares.
    #[derive(Debug, Deserialize, PartialEq)]
    struct PartialPerson {
        id: u64,
        name: String,
        ttl: Option<u64>,
        tag: Option<String>,
    }

    let mike = Person {
        id: 12,
        name: "Mike".to_owned(),
    };

    // A field the server added to a field/value array is ignored.
    let value = Value::Array(vec![
        Value::BulkString(b"id".to_vec()),
        Value::Integer(12),
        Value::BulkString(b"name".to_vec()),
        Value::BulkString(b"Mike".to_vec()),
        Value::BulkString(b"ttl".to_vec()),
        Value::Integer(99),
    ]);
    assert_eq!(mike, Person::deserialize(&value)?);

    // Field names rendered as simple strings are field names too.
    let value = Value::Array(vec![
        Value::SimpleString("id".to_owned()),
        Value::Integer(12),
        Value::SimpleString("name".to_owned()),
        Value::BulkString(b"Mike".to_vec()),
    ]);
    assert_eq!(mike, Person::deserialize(&value)?);

    // An element the server appended to a positional array is ignored.
    let value = Value::Array(vec![
        Value::Integer(12),
        Value::BulkString(b"Mike".to_vec()),
        Value::Integer(99),
    ]);
    assert_eq!(mike, Person::deserialize(&value)?);

    // Two pairs for a four-field struct are pairs, not a positional tuple.
    let value = Value::Array(vec![
        Value::BulkString(b"id".to_vec()),
        Value::Integer(12),
        Value::BulkString(b"name".to_vec()),
        Value::BulkString(b"Mike".to_vec()),
    ]);
    assert_eq!(
        PartialPerson {
            id: 12,
            name: "Mike".to_owned(),
            ttl: None,
            tag: None
        },
        PartialPerson::deserialize(&value)?
    );

    // A missing required field is a serde error naming it, not a blanket
    // `CannotParseStruct`.
    let value = Value::Array(vec![Value::BulkString(b"id".to_vec()), Value::Integer(12)]);
    let result = Person::deserialize(&value);
    let error = result.unwrap_err();
    assert!(
        matches!(error.kind(), ErrorKind::Client(ClientError::SerdeDeserialize(msg)) if msg.contains("name")),
        "{error:?}"
    );

    Ok(())
}

#[test]
fn _enum() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    enum E {
        A,                         // unit_variant
        B(u8),                     // newtype_variant
        C(u8, u8),                 // tuple_variant
        D { r: u8, g: u8, b: u8 }, // struct_variant
    }

    let result = E::deserialize(&Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: Bytes::from_static(b"error"),
    }));

    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "error"
    ));

    // unit_variant
    let result = E::deserialize(&Value::BulkString(b"A".to_vec()))?; // b"A"
    assert_eq!(E::A, result);

    let result = E::deserialize(&Value::SimpleString("A".to_owned()))?; // b"A"
    assert_eq!(E::A, result);

    // newtype_variant
    let result = E::deserialize(&Value::Map(Vec::from([(
        Value::BulkString(b"B".to_vec()),
        Value::Integer(12),
    )])))?;
    assert_eq!(E::B(12), result);

    let result = E::deserialize(&Value::Array(vec![
        Value::BulkString(b"B".to_vec()),
        Value::Integer(12),
    ]))?;
    assert_eq!(E::B(12), result);

    // tuple_variant
    let result = E::deserialize(&Value::Map(Vec::from([(
        Value::BulkString(b"C".to_vec()),
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
    )])))?;
    assert_eq!(E::C(12, 13), result);

    let result = E::deserialize(&Value::Array(vec![
        Value::BulkString(b"C".to_vec()),
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
    ]))?;
    assert_eq!(E::C(12, 13), result);

    // struct_variant
    let result = E::deserialize(&Value::Array(vec![
        Value::BulkString(b"D".to_vec()),
        Value::Array(vec![
            Value::BulkString(b"r".to_vec()),
            Value::Integer(12),
            Value::BulkString(b"g".to_vec()),
            Value::Integer(13),
            Value::BulkString(b"b".to_vec()),
            Value::Integer(14),
        ]),
    ]))?;
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    let result = E::deserialize(&Value::Map(Vec::from([(
        Value::BulkString(b"D".to_vec()),
        Value::Array(vec![
            Value::BulkString(b"r".to_vec()),
            Value::Integer(12),
            Value::BulkString(b"g".to_vec()),
            Value::Integer(13),
            Value::BulkString(b"b".to_vec()),
            Value::Integer(14),
        ]),
    )])))?;
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    let result = E::deserialize(&Value::Map(Vec::from([(
        Value::BulkString(b"D".to_vec()),
        Value::Map(Vec::from([
            (Value::BulkString(b"r".to_vec()), Value::Integer(12)),
            (Value::BulkString(b"g".to_vec()), Value::Integer(13)),
            (Value::BulkString(b"b".to_vec()), Value::Integer(14)),
        ])),
    )])))?;
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    Ok(())
}

#[test]
fn out_of_range_integer_errors_instead_of_truncating() {
    log_try_init();

    // Mirror of the RESP deserializer: an out-of-range `Value::Integer` no longer
    // silently truncates/wraps when deserialized into a narrower target.
    let result = u8::deserialize(&Value::Integer(300));
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(crate::ClientError::CannotParseInteger)
        ),
        "u8 from 300 should error, got {error:?}"
    );

    let result = u32::deserialize(&Value::Integer(-1));
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(crate::ClientError::CannotParseInteger)
        ),
        "u32 from -1 should error, got {error:?}"
    );

    // In-range values are preserved; a nil is refused, not defaulted.
    assert_eq!(42u8, u8::deserialize(&Value::Integer(42)).unwrap());
    assert!(i32::deserialize(&Value::Null).is_err());
}

#[test]
fn lossy_double_to_integer_errors_instead_of_truncating() {
    log_try_init();

    // Mirror of the RESP deserializer: `Value::Double` only converts to an
    // integer when the conversion is exact.
    let result = i64::deserialize(&Value::Double(3.9));
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(crate::ClientError::CannotParseInteger)
        ),
        "i64 from 3.9 should error, got {error:?}"
    );

    let result = i64::deserialize(&Value::Double(1e300));
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(crate::ClientError::CannotParseInteger)
        ),
        "i64 from 1e300 should error, got {error:?}"
    );

    let result = u32::deserialize(&Value::Double(-1.));
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(crate::ClientError::CannotParseInteger)
        ),
        "u32 from -1.0 should error, got {error:?}"
    );

    let result = i8::deserialize(&Value::Double(f64::NAN));
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(crate::ClientError::CannotParseInteger)
        ),
        "i8 from NaN should error, got {error:?}"
    );

    // An exactly representable double still deserializes to the integer.
    assert_eq!(3i64, i64::deserialize(&Value::Double(3.)).unwrap());
    assert_eq!(255u8, u8::deserialize(&Value::Double(255.)).unwrap());
}

#[test]
fn one_element_array_unwraps_for_every_integer_width() {
    log_try_init();

    // Mirror of the RESP deserializer: a one-element array holding an integer
    // unwraps to that integer, for every width and not just `i64`/`u64`.
    let value = Value::Array(vec![Value::Integer(12)]);
    assert_eq!(12i8, i8::deserialize(&value).unwrap());
    assert_eq!(12u8, u8::deserialize(&value).unwrap());
    assert_eq!(12i16, i16::deserialize(&value).unwrap());
    assert_eq!(12u16, u16::deserialize(&value).unwrap());
    assert_eq!(12i32, i32::deserialize(&value).unwrap());
    assert_eq!(12u32, u32::deserialize(&value).unwrap());
    assert_eq!(12i64, i64::deserialize(&value).unwrap());
    assert_eq!(12u64, u64::deserialize(&value).unwrap());
    assert_eq!(12i128, i128::deserialize(&value).unwrap());
    assert_eq!(12u128, u128::deserialize(&value).unwrap());

    // The element still has to fit the target.
    let value = Value::Array(vec![Value::Integer(300)]);
    let error = u8::deserialize(&value).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::CannotParseInteger)
    ));

    // A longer array would have to discard the rest silently.
    let value = Value::Array(vec![Value::Integer(12), Value::Integer(13)]);
    for result in [
        i8::deserialize(&value).map(i64::from),
        u32::deserialize(&value).map(i64::from),
        i64::deserialize(&value),
        u64::deserialize(&value).map(|u| i64::try_from(u).unwrap()),
    ] {
        let error = result.unwrap_err();
        assert!(matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ));
    }
}

#[test]
fn integers_128_are_supported() {
    log_try_init();

    assert_eq!(12i128, i128::deserialize(&Value::Integer(12)).unwrap());
    assert_eq!(12u128, u128::deserialize(&Value::Integer(12)).unwrap());
    assert_eq!(
        12i128,
        i128::deserialize(&Value::SimpleString("12".to_owned())).unwrap()
    );
    assert_eq!(
        12u128,
        u128::deserialize(&Value::BulkString(b"12".to_vec())).unwrap()
    );
    assert_eq!(12i128, i128::deserialize(&Value::Double(12.)).unwrap());
    assert!(i128::deserialize(&Value::Null).is_err());
    let error = u128::deserialize(&Value::Integer(-1)).unwrap_err();
    assert!(matches!(
        error.kind(),
        ErrorKind::Client(ClientError::CannotParseInteger)
    ));
}

#[test]
fn numbers_and_booleans_read_as_text() {
    log_try_init();

    // A visitor that takes the text however it comes, so `deserialize_str` can be
    // compared with `deserialize_string` on the same reply.
    struct AnyStr;

    impl serde::de::Visitor<'_> for AnyStr {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<String, E> {
            Ok(v.to_owned())
        }
    }

    fn as_str(value: &Value) -> Result<String> {
        serde::Deserializer::deserialize_str(value, AnyStr)
    }

    // Mirror of the RESP deserializer: a numeric or boolean reply is readable as
    // text, and the entry point — `deserialize_str` for an identifier,
    // `deserialize_string` for a `String` — never decides whether the command
    // succeeds, nor the text it produces.
    for (value, text) in [
        (Value::Integer(12), "12"),
        (Value::Double(12.), "12"),
        (Value::Boolean(true), "true"),
        (Value::Boolean(false), "false"),
        (Value::SimpleString("foo".to_owned()), "foo"),
    ] {
        assert_eq!(text, String::deserialize(&value).unwrap());
        assert_eq!(text, as_str(&value).unwrap());
    }
}
