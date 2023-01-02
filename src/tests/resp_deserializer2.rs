use crate::{
    resp::{RespDeserializer2, RawValueDecoder}, tests::log_try_init, Error, RedisError, RedisErrorKind, Result,
};
use bytes::BytesMut;
use serde::Deserialize;
use smallvec::SmallVec;
use tokio_util::codec::Decoder;
use std::collections::HashMap;

fn deserialize<'a, T>(str: &'a str) -> Result<T>
where
    T: serde::Deserialize<'a>,
{
    let mut decoder = RawValueDecoder;
    let mut bytes: BytesMut = str.into();
    let raw_values = decoder.decode(&mut bytes)?.unwrap();

    let buf = str.as_bytes();
    let mut deserializer = RespDeserializer2::new(buf, raw_values);
    T::deserialize(&mut deserializer)
}

#[test]
fn bool() -> Result<()> {
    log_try_init();

    let result: Result<bool> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: bool = deserialize("$1\r\n1\r\n")?; // b"1"
    assert!(result);

    let result: bool = deserialize("$1\r\n0\r\n")?; // b"0"
    assert!(!result);

    let result: bool = deserialize("$4\r\ntrue\r\n")?; // b"true"
    assert!(result);

    let result: bool = deserialize("$5\r\nfalse\r\n")?; // b"false"
    assert!(!result);

    let result: bool = deserialize(":1\r\n")?; // 1
    assert!(result);

    let result: bool = deserialize(":0\r\n")?; // 0
    assert!(!result);

    let result: bool = deserialize("+OK\r\n")?; // "OK"
    assert!(result);

    let result: bool = deserialize("+KO\r\n")?; // "KO"
    assert!(!result);

    Ok(())
}

#[test]
fn integer() -> Result<()> {
    log_try_init();

    let result: Result<i64> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: i64 = deserialize(":12\r\n")?; // 12
    assert_eq!(12, result);

    let result: i64 = deserialize("$-1\r\n")?; // ""
    assert_eq!(0, result);

    let result: i64 = deserialize("_\r\n")?; // null
    assert_eq!(0, result);

    let result: i64 = deserialize("$2\r\n12\r\n")?; // b"12"
    assert_eq!(12, result);

    let result: i64 = deserialize("+12\r\n")?; // "12"
    assert_eq!(12, result);

    Ok(())
}

#[test]
fn float() -> Result<()> {
    log_try_init();

    let result: Result<f64> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: f64 = deserialize(":12\r\n")?; // 12
    assert_eq!(12.0, result);

    let result: f64 = deserialize(",12.12\r\n")?; // 12.12
    assert_eq!(12.12, result);

    let result: f64 = deserialize("$-1\r\n")?; // ""
    assert_eq!(0.0, result);

    let result: f64 = deserialize("_\r\n")?; // null
    assert_eq!(0.0, result);

    let result: f64 = deserialize("$5\r\n12.12\r\n")?; // b"12.12"
    assert_eq!(12.12, result);

    let result: f64 = deserialize("+12.12\r\n")?; // "12.12"
    assert_eq!(12.12, result);

    Ok(())
}

#[test]
fn char() -> Result<()> {
    log_try_init();

    let result: Result<char> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: char = deserialize("$1\r\nm\r\n")?; // b"m"
    assert_eq!('m', result);

    let result: char = deserialize("+m\r\n")?; // "m"
    assert_eq!('m', result);

    Ok(())
}

#[test]
fn str() -> Result<()> {
    log_try_init();

    let result: Result<&str> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: &str = deserialize("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!("hello", result);

    let result: &str = deserialize("+hello\r\n")?; // "hello"
    assert_eq!("hello", result);

    let result: &str = deserialize("$-1\r\n")?; // b""
    assert_eq!("", result);

    Ok(())
}

#[test]
fn string() -> Result<()> {
    log_try_init();

    let result: Result<String> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: String = deserialize("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!("hello", result);

    let result: String = deserialize("+hello\r\n")?; // "hello"
    assert_eq!("hello", result);

    let result: String = deserialize("$-1\r\n")?; // b""
    assert_eq!("", result);

    Ok(())
}

#[test]
fn option() -> Result<()> {
    log_try_init();

    let result: Result<Option<String>> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Option<String> = deserialize("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!(Some("hello".to_owned()), result);

    let result: Option<String> = deserialize("$-1\r\n")?; // b""
    assert_eq!(None, result);

    let result: Option<String> = deserialize("_\r\n")?; // null
    assert_eq!(None, result);

    let result: Option<i64> = deserialize(":12\r\n")?; // b"12"
    assert_eq!(Some(12), result);

    let result: Option<i64> = deserialize("$-1\r\n")?; // b""
    assert_eq!(None, result);

    let result: Option<i64> = deserialize("_\r\n")?; // null
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn unit() -> Result<()> {
    log_try_init();

    let result: Result<()> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Result<()> = deserialize("$-1\r\n"); // ""
    assert!(result.is_ok());

    let result: Result<()> = deserialize("_\r\n"); // null
    assert!(result.is_ok());

    let result: Result<()> = deserialize("$5\r\nhello\r\n"); // "hello"
    assert!(result.is_err());

    Ok(())
}

#[test]
fn unit_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize)]
    struct Unit;

    let result: Result<Unit> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Result<Unit> = deserialize("$-1\r\n"); // ""
    assert!(result.is_ok());

    let result: Result<Unit> = deserialize("_\r\n"); // null
    assert!(result.is_ok());

    let result: Result<Unit> = deserialize("$5\r\nhello\r\n"); // "hello"
    assert!(result.is_err());

    Ok(())
}

#[test]
fn newtype_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize)]
    struct Millimeters(u8);

    let result: Result<Millimeters> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Millimeters = deserialize(":12\r\n")?; // 12
    assert_eq!(12, result.0);

    Ok(())
}

#[test]
fn seq() -> Result<()> {
    log_try_init();

    let result: Result<Vec<i32>> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Vec<i32> = deserialize("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(2, result.len());
    assert_eq!(12, result[0]);
    assert_eq!(13, result[1]);

    let result: SmallVec<[String; 2]> = deserialize("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello", b"world"]
    assert_eq!(2, result.len());
    assert_eq!("hello", result[0]);
    assert_eq!("world", result[1]);

    let result: Vec<Option<String>> = deserialize("*3\r\n$5\r\nhello\r\n$-1\r\n$5\r\nworld\r\n")?; // [b"hello", b"world", null]
    assert_eq!(3, result.len());
    assert_eq!(Some("hello".to_owned()), result[0]);
    assert_eq!(None, result[1]);
    assert_eq!(Some("world".to_owned()), result[2]);

    Ok(())
}

#[test]
fn tuple() -> Result<()> {
    log_try_init();

    let result: Result<(i32, i32)> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: (i32, i32) = deserialize("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!((12, 13), result);

    let result: (&str, &str) = deserialize("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello", b"world"]
    assert_eq!(("hello", "world"), result);

    Ok(())
}

#[test]
fn tuple_struct() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Rgb(u8, u8, u8);

    let result: Result<Rgb> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Rgb = deserialize("*3\r\n:12\r\n:13\r\n:14\r\n")?; // [12, 13, 14]
    assert_eq!(Rgb(12, 13, 14), result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result: Result<HashMap<i32, i32>> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: HashMap<i32, i32> = deserialize("*4\r\n:12\r\n:13\r\n:14\r\n:15\r\n")?; // [12, 13, 14, 15]
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result: HashMap<i32, i32> = deserialize("%2\r\n:12\r\n:13\r\n:14\r\n:15\r\n")?; // { 12: 13, 14: 15 }
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    Ok(())
}

#[test]
fn _struct() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Person {
        pub id: u64,
        pub name: String,
    }

    let result: Result<Person> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    let result: Person = deserialize("*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n")?; // [b"id", 12, b"name", b"Mike"]
    assert_eq!(
        Person {
            id: 12,
            name: "Mike".to_owned()
        },
        result
    );

    let result: Person = deserialize("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n")?; // {b"id": 12, b"name": b"Mike"}
    assert_eq!(
        Person {
            id: 12,
            name: "Mike".to_owned()
        },
        result
    );

    Ok(())
}

#[test]
fn enum_variant() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    enum E {
        A,                         // unit_variant
        B(u8),                     // newtype_variant
        C(u8, u8),                 // tuple_variant
        D { r: u8, g: u8, b: u8 }, // struct_variant
    }

    let result: Result<E> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    // unit_variant
    let result: E = deserialize("$1\r\nA\r\n")?; // b"A"
    assert_eq!(E::A, result);

    let result: E = deserialize("+A\r\n")?; // "A"
    assert_eq!(E::A, result);

    // newtype_variant
    let result: E = deserialize("*2\r\n$1\r\nB\r\n:12\r\n")?; // [ b"B", 12 ]
    assert_eq!(E::B(12), result);

    let result: E = deserialize("%1\r\n$1\r\nB\r\n:12\r\n")?; // { b"B": 12 }
    assert_eq!(E::B(12), result);

    // tuple_variant
    let result: E = deserialize("*2\r\n$1\r\nC\r\n*2\r\n:12\r\n:13\r\n")?; // [ b"C", [12, 13] ]
    assert_eq!(E::C(12, 13), result);

    let result: E = deserialize("%1\r\n$1\r\nC\r\n*2\r\n:12\r\n:13\r\n")?; // { b"C": [12, 13] }
    assert_eq!(E::C(12, 13), result);

    // struct_variant
    let result: E = deserialize(
        "*2\r\n$1\r\nD\r\n*6\r\n$1\r\nr\r\n:12\r\n$1\r\ng\r\n:13\r\n$1\r\nb\r\n:14\r\n",
    )?; // [ b"D", [b"r", 12, b"g", 13, b"b", 14] ]
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    let result: E = deserialize(
        "%1\r\n$1\r\nD\r\n*6\r\n$1\r\nr\r\n:12\r\n$1\r\ng\r\n:13\r\n$1\r\nb\r\n:14\r\n",
    )?; // { b"D", [b"r", 12, b"g", 13, b"b", 14] }
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    let result: E = deserialize(
        "%1\r\n$1\r\nD\r\n%3\r\n$1\r\nr\r\n:12\r\n$1\r\ng\r\n:13\r\n$1\r\nb\r\n:14\r\n",
    )?; // { b"D", { b"r": 12, b"g": 13, b"b": 14 } }
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
