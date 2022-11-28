use crate::{
    resp::{RespDeserializer, Value},
    tests::log_try_init,
    Error, RedisError, RedisErrorKind, Result,
};
use serde::Deserialize;

fn deserialize_value(str: &str) -> Result<Value> {
    let buf = str.as_bytes();
    let mut deserializer = RespDeserializer::from_bytes(buf);
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
    assert_eq!(
        Value::BulkString(Some(b"hello".to_vec())),
        result
    );

    let result = deserialize_value("$-1\r\n")?; // b""
    assert_eq!(Value::BulkString(None), result);

    Ok(())
}

#[test]
fn array() -> Result<()> {
    log_try_init();

    let result = deserialize_value("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Value::Array(Some(vec![Value::Integer(12), Value::Integer(13)])),
        result
    );

    let result = deserialize_value("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Value::Array(Some(vec![
            Value::BulkString(Some(b"hello".to_vec())),
            Value::BulkString(Some(b"world".to_vec()))
        ])),
        result
    );

    let result = deserialize_value("*0\r\n")?; // []
    assert_eq!(Value::Array(None), result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result = deserialize_value("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n")?; // {b"id": 12, b"name": b"Mike"}
    assert_eq!(
        Value::Array(Some(vec![
            Value::BulkString(Some(b"id".to_vec())),
            Value::Integer(12),
            Value::BulkString(Some(b"name".to_vec())),
            Value::BulkString(Some(b"Mike".to_vec()))
        ])),
        result
    );

    let result = deserialize_value("%0\r\n")?; // {}
    assert_eq!(Value::Array(None), result);

    Ok(())
}

#[test]
fn set() -> Result<()> {
    log_try_init();

    let result = deserialize_value("~2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(
        Value::Array(Some(vec![Value::Integer(12), Value::Integer(13)])),
        result
    );

    let result = deserialize_value("~2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello, b"world"]
    assert_eq!(
        Value::Array(Some(vec![
            Value::BulkString(Some(b"hello".to_vec())),
            Value::BulkString(Some(b"world".to_vec()))
        ])),
        result
    );

    let result = deserialize_value("~0\r\n")?; // []
    assert_eq!(Value::Array(None), result);

    Ok(())
}

#[test]
fn push() -> Result<()> {
    log_try_init();

    let result = deserialize_value(">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$7\r\npayload\r\n")?; // [b"message, b"channel", b"payload"]
    assert_eq!(
        Value::Push(Some(vec![
            Value::BulkString(Some(b"message".to_vec())),
            Value::BulkString(Some(b"channel".to_vec())),
            Value::BulkString(Some(b"payload".to_vec()))
        ])),
        result
    );

    let result = deserialize_value(">0\r\n")?; // []
    assert_eq!(Value::Push(None), result);

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
            description: _
        }))
    ));

    Ok(())
}
