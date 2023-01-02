use bytes::BytesMut;
use tokio_util::codec::Decoder;

use crate::{resp::BufferDecoder, Result};

fn decode(str: &str) -> Result<Option<Vec<u8>>> {
    let mut buffer_decoder = BufferDecoder;
    let mut buf: BytesMut = str.into();
    buffer_decoder.decode(&mut buf)
}

#[test]
fn integer() -> Result<()> {
    let result = decode(":12\r\n")?;
    assert_eq!(Some(":12\r\n".as_bytes().to_vec()), result);

    let result = decode(":12\r")?;
    assert_eq!(None, result);

    let result = decode(":12")?;
    assert_eq!(None, result);

    // malformed numbers are not checked
    let result = decode(":a\r\n");
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn string() -> Result<()> {
    let result = decode("+OK\r\n")?;
    assert_eq!(Some("+OK\r\n".as_bytes().to_vec()), result);

    let result = decode("+OK\r")?;
    assert_eq!(None, result);

    let result = decode("+OK")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn error() -> Result<()> {
    let result = decode("-ERR error\r\n")?;
    assert_eq!(Some("-ERR error\r\n".as_bytes().to_vec()), result);

    let result = decode("-ERR error\r")?;
    assert_eq!(None, result);

    let result = decode("-ERR error")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn double() -> Result<()> {
    let result = decode(",12.12\r\n")?;
    assert_eq!(Some(",12.12\r\n".as_bytes().to_vec()), result);

    let result = decode(",12.12\r")?;
    assert_eq!(None, result);

    let result = decode(",12.12")?;
    assert_eq!(None, result);

    // malformed numbers are not checked
    let result = decode(",a\r\n");
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn bool() -> Result<()> {
    let result = decode("#f\r\n")?;
    assert_eq!(Some("#f\r\n".as_bytes().to_vec()), result);

    let result = decode("#f\r")?;
    assert_eq!(None, result);

    let result = decode("#f")?;
    assert_eq!(None, result);

    // malformed booleans are not checked
    let result = decode("#a\r\n");
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn nil() -> Result<()> {
    let result = decode("_\r\n")?;
    assert_eq!(Some("_\r\n".as_bytes().to_vec()), result);

    let result = decode("_\r")?;
    assert_eq!(None, result);

    let result = decode("_")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn bulk_string() -> Result<()> {
    let result = decode("$5\r\nhello\r\n")?;
    assert_eq!(Some("$5\r\nhello\r\n".as_bytes().to_vec()), result);

    let result = decode("$7\r\nhel\r\nlo\r\n")?; // b"hel\r\nlo"
    assert_eq!(Some("$7\r\nhel\r\nlo\r\n".as_bytes().to_vec()), result);

    let result = decode("$0\r\n\r\n")?; // b""
    assert_eq!(Some("$0\r\n\r\n".as_bytes().to_vec()), result);

    let result = decode("$5")?;
    assert_eq!(None, result);

    let result = decode("$5\r")?;
    assert_eq!(None, result);

    let result = decode("$5\r\n")?;
    assert_eq!(None, result);

    let result = decode("$5\r\nhello")?;
    assert_eq!(None, result);

    let result = decode("$5\r\nhello\r")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn array() -> Result<()> {
    let result = decode("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?;
    assert_eq!(Some("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n".as_bytes().to_vec()), result);

    let result = decode("*2")?;
    assert_eq!(None, result);

    let result = decode("*2\r")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n$5")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n$5\r")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n$5\r\n")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n$5\r\nhello")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n$5\r\nhello\r")?;
    assert_eq!(None, result);

    let result = decode("*2\r\n$5\r\nhello\r\n")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    let result = decode("%1\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?;
    assert_eq!(Some("%1\r\n$5\r\nhello\r\n$5\r\nworld\r\n".as_bytes().to_vec()), result);

    let result = decode("%1")?;
    assert_eq!(None, result);

    let result = decode("%1\r")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n$5")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n$5\r")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n$5\r\n")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n$5\r\nhello")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n$5\r\nhello\r")?;
    assert_eq!(None, result);

    let result = decode("%1\r\n$5\r\nhello\r\n")?;
    assert_eq!(None, result);

    Ok(())
}