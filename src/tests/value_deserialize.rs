use std::collections::HashMap;

use crate::{
    Error, RedisError, RedisErrorKind, Result,
    resp::{RespDeserializer, Value},
    tests::log_try_init,
};
use serde::Deserialize;

fn deserialize_value(str: &str) -> Result<Value> {
    let buf = str.as_bytes();
    let mut deserializer = RespDeserializer::new(buf);
    Value::deserialize(&mut deserializer)
}

#[test]
fn simple_string() -> Result<()> {
    log_try_init();

    let result = deserialize_value("+OK\r\n")?; // "OK"
    assert_eq!(Value::SimpleString("OK".to_owned()), result);

    Ok(())
}

#[test]
fn integer() -> Result<()> {
    log_try_init();

    let result = deserialize_value(":12\r\n")?; // 12
    assert_eq!(Value::Integer(12), result);

    Ok(())
}

#[test]
fn bool() -> Result<()> {
    log_try_init();

    let result = deserialize_value("#t\r\n")?; // true
    assert_eq!(Value::Boolean(true), result);
    let result = deserialize_value("#f\r\n")?; // false
    assert_eq!(Value::Boolean(false), result);

    Ok(())
}

#[test]
fn double() -> Result<()> {
    log_try_init();

    let result = deserialize_value(",12.12\r\n")?; // 12.12
    assert_eq!(Value::Double(12.12), result);

    Ok(())
}

#[test]
fn bulk_string() -> Result<()> {
    log_try_init();

    let result = deserialize_value("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!(Value::BulkString(b"hello".to_vec()), result);

    let result = deserialize_value("$7\r\nhel\r\nlo\r\n")?; // b"hel\r\nlo"
    assert_eq!(Value::BulkString(b"hel\r\nlo".to_vec()), result);

    let result = deserialize_value("$5\r\nhello\r");
    assert!(matches!(result, Err(Error::EOF)));

    let result = deserialize_value("$5\r\nhello");
    assert!(matches!(result, Err(Error::EOF)));

    let result = deserialize_value("$5\r");
    assert!(matches!(result, Err(Error::EOF)));

    let result = deserialize_value("$5");
    assert!(matches!(result, Err(Error::EOF)));

    let result = deserialize_value("$");
    assert!(matches!(result, Err(Error::EOF)));

    let result = deserialize_value("$6\r\nhello\r\n");
    assert!(matches!(result, Err(Error::EOF)));

    Ok(())
}

#[test]
fn array() -> Result<()> {
    log_try_init();

    let result = deserialize_value("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
        result
    );

    let result = deserialize_value("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Value::Array(vec![
            Value::BulkString(b"hello".to_vec()),
            Value::BulkString(b"world".to_vec())
        ]),
        result
    );

    let result = deserialize_value("*0\r\n")?; // []
    assert_eq!(Value::Nil, result);

    Ok(())
}

#[test]
fn nil() -> Result<()> {
    log_try_init();

    let result = deserialize_value("_\r\n")?;
    assert_eq!(Value::Nil, result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result = deserialize_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n")?; // {b"id": 12, b"name": b"Mike"}
    assert_eq!(
        Value::Map(HashMap::from([
            (Value::BulkString(b"id".to_vec()), Value::Integer(12)),
            (
                Value::BulkString(b"name".to_vec()),
                Value::BulkString(b"Mike".to_vec())
            )
        ])),
        result
    );

    let result = deserialize_value("%0\r\n")?; // {}
    assert_eq!(Value::Nil, result);

    Ok(())
}

#[test]
fn set() -> Result<()> {
    log_try_init();

    let result = deserialize_value("~2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Value::Array(vec![Value::Integer(12), Value::Integer(13)]),
        result
    );

    let result = deserialize_value("~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Value::Array(vec![
            Value::BulkString(b"hello".to_vec()),
            Value::BulkString(b"world".to_vec())
        ]),
        result
    );

    let result = deserialize_value("~0\r\n")?; // []
    assert_eq!(Value::Nil, result);

    Ok(())
}

#[test]
fn push() -> Result<()> {
    log_try_init();

    let result = deserialize_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload\r\n")?; // [b"message, b"channel", b"payload"]
    assert_eq!(
        Value::Push(vec![
            Value::BulkString(b"message".to_vec()),
            Value::BulkString(b"channel".to_vec()),
            Value::BulkString(b"payload".to_vec())
        ]),
        result
    );

    let result = deserialize_value(">0\r\n")?; // []
    assert_eq!(Value::Nil, result);

    Ok(())
}

#[test]
fn error() -> Result<()> {
    log_try_init();

    let result = deserialize_value("-ERR error\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    Ok(())
}

#[test]
fn blob_error() -> Result<()> {
    log_try_init();

    let result = deserialize_value("!9\r\nERR error\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "error"
    ));

    let result = deserialize_value("!11\r\nERR er\r\nror\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "er\r\nror"
    ));

    Ok(())
}
