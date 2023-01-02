use std::collections::{HashMap};

use bytes::BytesMut;
use tokio_util::codec::Decoder;

use crate::{
    resp::{Value, ValueDecoder},
    tests::log_try_init,
    RedisError, RedisErrorKind, Result,
};

fn decode_value(str: &str) -> Result<Option<Value>> {
    let mut buf = BytesMut::from(str);
    let mut value_decoder = ValueDecoder;
    value_decoder.decode(&mut buf)
}

#[test]
fn simple_string() -> Result<()> {
    log_try_init();

    let result = decode_value("+OK\r\n")?; // "OK"
    assert_eq!(Some(Value::SimpleString("OK".to_owned())), result);

    let result = decode_value("+OK\r")?;
    assert_eq!(None, result);

    let result = decode_value("+OK")?;
    assert_eq!(None, result);

    let result = decode_value("+")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn integer() -> Result<()> {
    log_try_init();

    let result = decode_value(":12\r\n")?; // 12
    assert_eq!(Some(Value::Integer(12)), result);

    let result = decode_value(":12\r")?;
    assert_eq!(None, result);

    let result = decode_value(":12")?;
    assert_eq!(None, result);

    let result = decode_value(":")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn double() -> Result<()> {
    log_try_init();

    let result = decode_value(",12.12\r\n")?; // 12.12
    assert_eq!(Some(Value::Double(12.12)), result);

    let result = decode_value(",12.12\r")?;
    assert_eq!(None, result);

    let result = decode_value(",12.12")?;
    assert_eq!(None, result);

    let result = decode_value(",")?;
    assert_eq!(None, result);

    let result = decode_value(",12a\r\n");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn boolean() -> Result<()> {
    log_try_init();

    let result = decode_value("#f\r\n")?; // false
    assert_eq!(Some(Value::Boolean(false)), result);

    let result = decode_value("#t\r\n")?; // true
    assert_eq!(Some(Value::Boolean(true)), result);

    let result = decode_value("#f\r")?;
    assert_eq!(None, result);

    let result = decode_value("#f")?;
    assert_eq!(None, result);

    let result = decode_value("#")?;
    assert_eq!(None, result);

    let result = decode_value("#wrong\r\n");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn null() -> Result<()> {
    log_try_init();

    let result = decode_value("_\r\n")?; // null
    assert_eq!(Some(Value::Nil), result);

    let result = decode_value("_\r")?;
    assert_eq!(None, result);

    let result = decode_value("_")?;
    assert_eq!(None, result);

    let result = decode_value("_wrong\r\n");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn bulk_string() -> Result<()> {
    log_try_init();

    let result = decode_value("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!(Some(Value::BulkString(b"hello".to_vec())), result);

    let result = decode_value("$7\r\nhel\r\nlo\r\n")?; // b"hel\r\nlo"
    assert_eq!(Some(Value::BulkString(b"hel\r\nlo".to_vec())), result);

    let result = decode_value("$5\r\nhello\r")?;
    assert_eq!(None, result);

    let result = decode_value("$5\r\nhello")?;
    assert_eq!(None, result);

    let result = decode_value("$5\r")?;
    assert_eq!(None, result);

    let result = decode_value("$5")?;
    assert_eq!(None, result);

    let result = decode_value("$")?;
    assert_eq!(None, result);

    let result = decode_value("$6\r\nhello\r\n");
    log::debug!("result: {result:?}");
    assert!(result.is_err());

    Ok(())
}

#[test]
fn array() -> Result<()> {
    log_try_init();

    let result = decode_value("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Some(Value::Array(vec![Value::Integer(12), Value::Integer(13)])),
        result
    );

    let result = decode_value("*2\r\n:12\r\n:13\r")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n:12\r\n:13")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n:12\r\n")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n:12")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r")?;
    assert_eq!(None, result);

    let result = decode_value("*2")?;
    assert_eq!(None, result);

    let result = decode_value("*")?;
    assert_eq!(None, result);

    let result = decode_value("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Some(Value::Array(vec![
            Value::BulkString(b"hello".to_vec()),
            Value::BulkString(b"world".to_vec())
        ])),
        result
    );

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n")?; // {b"id": 12, b"name": b"Mike"}
    assert_eq!(
        Some(Value::Map(HashMap::from([
            (Value::BulkString(b"id".to_vec()), Value::Integer(12)),
            (
                Value::BulkString(b"name".to_vec()),
                Value::BulkString(b"Mike".to_vec())
            )
        ]))),
        result
    );

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r")?;
    assert_eq!(None, result);

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike")?;
    assert_eq!(None, result);

    let result = decode_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike")?;
    assert_eq!(None, result);

    let result = decode_value("%2\r")?;
    assert_eq!(None, result);

    let result = decode_value("%2")?;
    assert_eq!(None, result);

    let result = decode_value("%")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn set() -> Result<()> {
    log_try_init();

    let result = decode_value("~2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Some(Value::Set(vec![Value::Integer(12), Value::Integer(13)])),
        result
    );

    let result = decode_value("~2\r\n:12\r\n:13\r")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n:12\r\n:13")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n:12\r\n")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n:12")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r")?;
    assert_eq!(None, result);

    let result = decode_value("~2")?;
    assert_eq!(None, result);

    let result = decode_value("~")?;
    assert_eq!(None, result);

    let result = decode_value("~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Some(Value::Set(vec![
            Value::BulkString(b"hello".to_vec()),
            Value::BulkString(b"world".to_vec())
        ])),
        result
    );

    Ok(())
}

#[test]
fn push() -> Result<()> {
    log_try_init();

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload\r\n")?; // [b"message, b"channel", b"payload"]
    assert_eq!(
        Some(Value::Push(vec![
            Value::BulkString(b"message".to_vec()),
            Value::BulkString(b"channel".to_vec()),
            Value::BulkString(b"payload".to_vec())
        ])),
        result
    );

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload\r")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r\n")?;
    assert_eq!(None, result);

    let result = decode_value(">3\r")?;
    assert_eq!(None, result);

    let result = decode_value(">3")?;
    assert_eq!(None, result);

    let result = decode_value(">")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn error() -> Result<()> {
    log_try_init();

    let result = decode_value("-ERR error\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Ok(Some(Value::Error(RedisError {
            kind: RedisErrorKind::Err,
            description
        }))) if description == "error"
    ));

    let result = decode_value("-ERR error\r")?;
    assert_eq!(None, result);

    let result = decode_value("-ERR error")?;
    assert_eq!(None, result);

    let result = decode_value("-")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn blob_error() -> Result<()> {
    log_try_init();

    let result = decode_value("!9\r\nERR error\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Ok(Some(Value::Error(RedisError {
            kind: RedisErrorKind::Err,
            description
        }))) if description == "error"
    ));

    let result = decode_value("!11\r\nERR er\r\nror\r\n");
    println!("result: {result:?}");
    assert!(matches!(
        result,
        Ok(Some(Value::Error(RedisError {
            kind: RedisErrorKind::Err,
            description
        }))) if description == "er\r\nror"
    ));

    let result = decode_value("!9\r\nERR error\r")?;
    assert_eq!(None, result);

    let result = decode_value("!9\r\nERR error")?;
    assert_eq!(None, result);

    let result = decode_value("!9\r\n")?;
    assert_eq!(None, result);

    let result = decode_value("!9\r")?;
    assert_eq!(None, result);

    let result = decode_value("!9")?;
    assert_eq!(None, result);

    let result = decode_value("!")?;
    assert_eq!(None, result);

    Ok(())
}
