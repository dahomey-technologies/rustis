use super::log_try_init;
use crate::{
    resp::{RespBuf, RespSerializer, Value},
    RedisError, RedisErrorKind, Result,
};
use serde::Serialize;

fn serialize(value: Value) -> Result<RespBuf> {
    let mut serializer = RespSerializer::new();
    value.serialize(&mut serializer)?;
    let output = serializer.get_output();
    Ok(RespBuf::new(output.freeze()))
}

#[test]
fn integer() -> Result<()> {
    log_try_init();

    let resp_buf = serialize(Value::Integer(12))?;
    log::debug!("resp_buf: {resp_buf}");
    assert_eq!(b":12\r\n", resp_buf.as_bytes());

    Ok(())
}

#[test]
fn error() -> Result<()> {
    log_try_init();

    let resp_buf = serialize(Value::Error(RedisError {
        kind: RedisErrorKind::Err,
        description: "error".to_owned(),
    }))?;
    log::debug!("resp_buf: {resp_buf}");
    assert_eq!(b"-ERR error\r\n", resp_buf.as_bytes());

    Ok(())
}

#[test]
fn push() -> Result<()> {
    log_try_init();

    let resp_buf = serialize(Value::Push(vec![
        Value::Integer(12),
        Value::BulkString(b"foo".to_vec()),
    ]))?;
    log::debug!("resp_buf: {resp_buf}");
    assert_eq!(b">2\r\n:12\r\n$3\r\nfoo\r\n", resp_buf.as_bytes());

    Ok(())
}

#[test]
fn set() -> Result<()> {
    log_try_init();

    let resp_buf = serialize(Value::Set(vec![
        Value::Integer(12),
        Value::BulkString(b"foo".to_vec()),
    ]))?;
    log::debug!("resp_buf: {resp_buf}");
    assert_eq!(b"~2\r\n:12\r\n$3\r\nfoo\r\n", resp_buf.as_bytes());

    Ok(())
}

#[test]
fn array() -> Result<()> {
    log_try_init();

    let resp_buf = serialize(Value::Array(vec![
        Value::Integer(12),
        Value::BulkString(b"foo".to_vec()),
    ]))?;
    log::debug!("resp_buf: {resp_buf}");
    assert_eq!(b"*2\r\n:12\r\n$3\r\nfoo\r\n", resp_buf.as_bytes());

    Ok(())
}
