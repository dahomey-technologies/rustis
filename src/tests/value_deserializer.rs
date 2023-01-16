use std::collections::HashMap;

use crate::{resp::Value, tests::log_try_init, Error, RedisError, RedisErrorKind, Result};
use serde::Deserialize;
use smallvec::SmallVec;

#[test]
fn bool() -> Result<()> {
    log_try_init();

    let result = bool::deserialize(Value::Boolean(true))?;
    assert!(result);

    let result = bool::deserialize(Value::Boolean(false))?;
    assert!(!result);

    let result = bool::deserialize(Value::Integer(1))?;
    assert!(result);

    let result = bool::deserialize(Value::Integer(0))?;
    assert!(!result);

    let result = bool::deserialize(Value::Double(1.))?;
    assert!(result);

    let result = bool::deserialize(Value::Double(0.))?;
    assert!(!result);

    let result = bool::deserialize(Value::SimpleString("OK".to_owned()))?;
    assert!(result);

    let result = bool::deserialize(Value::BulkString(b"1".to_vec()))?;
    assert!(result);

    let result = bool::deserialize(Value::BulkString(b"0".to_vec()))?;
    assert!(!result);

    let result = bool::deserialize(Value::BulkString(b"true".to_vec()))?;
    assert!(result);

    let result = bool::deserialize(Value::BulkString(b"false".to_vec()))?;
    assert!(!result);

    let result = bool::deserialize(Value::Nil)?;
    assert!(!result);

    Ok(())
}

#[test]
fn i64() -> Result<()> {
    log_try_init();

    let result = i64::deserialize(Value::Integer(12))?;
    assert_eq!(12, result);

    let result = i64::deserialize(Value::Double(12.))?;
    assert_eq!(12, result);

    let result = i64::deserialize(Value::SimpleString("12".to_owned()))?;
    assert_eq!(12, result);

    let result = i64::deserialize(Value::BulkString(b"12".to_vec()))?;
    assert_eq!(12, result);

    let result = i64::deserialize(Value::Nil)?;
    assert_eq!(0, result);

    Ok(())
}

#[test]
fn u64() -> Result<()> {
    log_try_init();

    let result = u64::deserialize(Value::Integer(12))?;
    assert_eq!(12, result);

    let result = u64::deserialize(Value::Double(12.))?;
    assert_eq!(12, result);

    let result = u64::deserialize(Value::SimpleString("12".to_owned()))?;
    assert_eq!(12, result);

    let result = u64::deserialize(Value::BulkString(b"12".to_vec()))?;
    assert_eq!(12, result);

    let result = u64::deserialize(Value::Nil)?;
    assert_eq!(0, result);

    Ok(())
}

#[test]
fn f32() -> Result<()> {
    log_try_init();

    let result = f32::deserialize(Value::Integer(12))?;
    assert_eq!(12., result);

    let result = f32::deserialize(Value::Double(12.12))?;
    assert_eq!(12.12, result);

    let result = f32::deserialize(Value::SimpleString("12.12".to_owned()))?;
    assert_eq!(12.12, result);

    let result = f32::deserialize(Value::BulkString(b"12.12".to_vec()))?;
    assert_eq!(12.12, result);

    let result = f32::deserialize(Value::Nil)?;
    assert_eq!(0., result);

    Ok(())
}

#[test]
fn f64() -> Result<()> {
    log_try_init();

    let result = f64::deserialize(Value::Integer(12))?;
    assert_eq!(12., result);

    let result = f64::deserialize(Value::Double(12.12))?;
    assert_eq!(12.12, result);

    let result = f64::deserialize(Value::SimpleString("12.12".to_owned()))?;
    assert_eq!(12.12, result);

    let result = f64::deserialize(Value::BulkString(b"12.12".to_vec()))?;
    assert_eq!(12.12, result);

    let result = f64::deserialize(Value::Nil)?;
    assert_eq!(0., result);

    Ok(())
}

#[test]
fn char() -> Result<()> {
    log_try_init();

    let result = char::deserialize(Value::SimpleString("a".to_owned()))?;
    assert_eq!('a', result);

    let result = char::deserialize(Value::BulkString(b"a".to_vec()))?;
    assert_eq!('a', result);

    let result = char::deserialize(Value::Nil)?;
    assert_eq!('\0', result);

    Ok(())
}

// #[test]
// fn str() -> Result<()> {
//     log_try_init();

//     let value = Value::SimpleString("foo".to_owned());
//     let result = <&str>::deserialize(&value)?;
//     assert_eq!("foo", result);

//     let value = Value::BulkString(b"foo".to_vec());
//     let result = <&str>::deserialize(&value)?;
//     assert_eq!("foo", result);

//     let result = <&str>::deserialize(&Value::Nil)?;
//     assert_eq!("", result);

//     Ok(())
// }

#[test]
fn string() -> Result<()> {
    log_try_init();

    let result = String::deserialize(Value::SimpleString("foo".to_owned()))?;
    assert_eq!("foo", result);

    let result = String::deserialize(Value::BulkString(b"foo".to_vec()))?;
    assert_eq!("foo", result);

    let result = String::deserialize(Value::Double(12.))?;
    assert_eq!("12", result);

    let result = String::deserialize(Value::Nil)?;
    assert_eq!("", result);

    Ok(())
}

#[test]
fn option() -> Result<()> {
    log_try_init();

    let result = Option::<String>::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = Option::<String>::deserialize(Value::BulkString(b"hello".to_vec()))?;
    assert_eq!(Some("hello".to_owned()), result);

    let result = Option::<String>::deserialize(Value::Nil)?;
    assert_eq!(None, result);

    let result = Option::<i64>::deserialize(Value::Integer(12))?;
    assert_eq!(Some(12), result);

    let result = Option::<i64>::deserialize(Value::Nil)?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn unit() -> Result<()> {
    log_try_init();

    let result = <()>::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = <()>::deserialize(Value::Nil);
    assert!(result.is_ok());

    let result = <()>::deserialize(Value::BulkString(b"hello".to_vec()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn unit_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize)]
    struct Unit;

    let result = Unit::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = Unit::deserialize(Value::Nil);
    assert!(result.is_ok());

    let result = Unit::deserialize(Value::BulkString(b"hello".to_vec()));
    assert!(result.is_err());

    Ok(())
}

#[test]
fn newtype_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize)]
    struct Millimeters(u8);

    let result = Millimeters::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = Millimeters::deserialize(Value::Integer(12))?;
    assert_eq!(12, result.0);

    Ok(())
}

#[test]
fn seq() -> Result<()> {
    log_try_init();

    let result = Vec::<i32>::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result =
        Vec::<i32>::deserialize(Value::Array(vec![Value::Integer(12), Value::Integer(13)]))?;
    assert_eq!(2, result.len());
    assert_eq!(12, result[0]);
    assert_eq!(13, result[1]);

    let result = SmallVec::<[String; 2]>::deserialize(Value::Array(vec![
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

    let result = <(i32, i32)>::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = <(i32, i32, i32)>::deserialize(Value::Array(vec![
        Value::Integer(12),
        Value::Integer(13),
        Value::Integer(14),
    ]))?;
    assert_eq!((12, 13, 14), result);

    let result = <(String, String)>::deserialize(Value::Array(vec![
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

    let result = Rgb::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = Rgb::deserialize(Value::Array(vec![
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

    let result = HashMap::<i32, i32>::deserialize(Value::Map(HashMap::from([
        (Value::Integer(12), Value::Integer(13)),
        (Value::Integer(14), Value::Integer(15)),
    ])))?;
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result = HashMap::<i32, i32>::deserialize(Value::Array(vec![
        Value::Integer(12),
        Value::Integer(13),
        Value::Integer(14),
        Value::Integer(15),
    ]))?;
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result = HashMap::<i32, i32>::deserialize(Value::Array(vec![
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
        Value::Array(vec![Value::Integer(14), Value::Integer(15)]),
    ]))?;
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result = HashMap::<String, Vec<String>>::deserialize(Value::Array(vec![
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
    assert_eq!(Some(&vec!["OW".to_owned(), "update".to_owned()]), result.get("a"));
    assert_eq!(Some(&vec!["OW".to_owned(), "update".to_owned()]), result.get("a"));

    let result = HashMap::<String, usize>::deserialize(Value::Array(vec![
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
    pub struct Person {
        pub id: u64,
        pub name: String,
    }

    log_try_init();

    let result = Person::deserialize(Value::Map(HashMap::from([
        (Value::BulkString(b"id".to_vec()), Value::Integer(12)),
        (
            Value::BulkString(b"name".to_vec()),
            Value::BulkString(b"foo".to_vec()),
        ),
    ])))?;
    assert_eq!(12, result.id);
    assert_eq!("foo", result.name);

    let value = Value::Array(vec![
        Value::Integer(12),
        Value::BulkString(b"foo".to_vec()),
    ]);

    let result = Person::deserialize(value)?;
    assert_eq!(12, result.id);
    assert_eq!("foo", result.name);

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

    let result = E::deserialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }));

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    // unit_variant
    let result = E::deserialize(Value::BulkString(b"A".to_vec()))?; // b"A"
    assert_eq!(E::A, result);

    let result = E::deserialize(Value::SimpleString("A".to_owned()))?; // b"A"
    assert_eq!(E::A, result);

    // newtype_variant
    let result = E::deserialize(Value::Map(HashMap::from([(
        Value::BulkString(b"B".to_vec()),
        Value::Integer(12),
    )])))?;
    assert_eq!(E::B(12), result);

    let result = E::deserialize(Value::Array(vec![
        Value::BulkString(b"B".to_vec()),
        Value::Integer(12),
    ]))?;
    assert_eq!(E::B(12), result);

    // tuple_variant
    let result = E::deserialize(Value::Map(HashMap::from([(
        Value::BulkString(b"C".to_vec()),
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
    )])))?;
    assert_eq!(E::C(12, 13), result);

    let result = E::deserialize(Value::Array(vec![
        Value::BulkString(b"C".to_vec()),
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
    ]))?;
    assert_eq!(E::C(12, 13), result);

    // struct_variant
    let result = E::deserialize(Value::Array(vec![
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

    let result = E::deserialize(Value::Map(HashMap::from([(
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

    let result = E::deserialize(Value::Map(HashMap::from([(
        Value::BulkString(b"D".to_vec()),
        Value::Map(HashMap::from([
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
