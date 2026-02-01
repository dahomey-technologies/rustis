use std::ops::Range;

use crate::{
    ClientError, Error, Result,
    resp::{BufferDecoder, RespBuf, RespFrame, RespResponse},
};
use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

fn decode(str: &str) -> Result<Option<RespResponse>> {
    let mut buffer_decoder = BufferDecoder;
    let mut buf: BytesMut = str.into();
    buffer_decoder.decode(&mut buf)
}

#[test]
fn integer() {
    let result = decode(":12\r\n").unwrap();
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b":12\r\n")),
            RespFrame::Integer(12)
        )),
        result
    );

    let result = decode(":12\r").unwrap();
    assert_eq!(None, result);

    let result = decode(":12").unwrap();
    assert_eq!(None, result);

    let result = decode(":a\r\n");
    assert!(matches!(
        result,
        Err(Error::Client(ClientError::CannotParseInteger))
    ));
}

#[test]
fn string() -> Result<()> {
    let result = decode("+OK\r\n")?;
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"+OK\r\n")),
            RespFrame::SimpleString(1..3)
        )),
        result
    );

    let result = decode("+OK\r")?;
    assert_eq!(None, result);

    let result = decode("+OK")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn error() -> Result<()> {
    let result = decode("-ERR error\r\n")?;
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"-ERR error\r\n")),
            RespFrame::Error(1..10)
        )),
        result
    );

    let result = decode("-ERR error\r")?;
    assert_eq!(None, result);

    let result = decode("-ERR error")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn double() -> Result<()> {
    let result = decode(",12.12\r\n")?;
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b",12.12\r\n")),
            RespFrame::Double(12.12)
        )),
        result
    );

    let result = decode(",12.12\r")?;
    assert_eq!(None, result);

    let result = decode(",12.12")?;
    assert_eq!(None, result);

    let result = decode(",a\r\n");
    assert!(matches!(
        result,
        Err(Error::Client(ClientError::CannotParseDouble))
    ));

    Ok(())
}

#[test]
fn bool() -> Result<()> {
    let result = decode("#f\r\n")?;
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"#f\r\n")),
            RespFrame::Boolean(false)
        )),
        result
    );

    let result = decode("#f\r")?;
    assert_eq!(None, result);

    let result = decode("#f")?;
    assert_eq!(None, result);

    let result = decode("#a\r\n");
    assert!(matches!(
        result,
        Err(Error::Client(ClientError::CannotParseBoolean))
    ));

    Ok(())
}

#[test]
fn null() -> Result<()> {
    let result = decode("_\r\n")?;
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"_\r\n")),
            RespFrame::Null
        )),
        result
    );

    let result = decode("_\r")?;
    assert_eq!(None, result);

    let result = decode("_")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn bulk_string() {
    let result = decode("$5\r\nhello\r\n").unwrap();
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"$5\r\nhello\r\n")),
            RespFrame::BulkString(4..9)
        )),
        result
    );

    let result = decode("$7\r\nhel\r\nlo\r\n").unwrap(); // b"hel\r\nlo"
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"$7\r\nhel\r\nlo\r\n")),
            RespFrame::BulkString(4..11)
        )),
        result
    );

    let result = decode("$0\r\n\r\n").unwrap(); // b""
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"$0\r\n\r\n")),
            RespFrame::BulkString(4..4)
        )),
        result
    );

    let result = decode("$5").unwrap();
    assert_eq!(None, result);

    let result = decode("$5\r").unwrap();
    assert_eq!(None, result);

    let result = decode("$5\r\n").unwrap();
    assert_eq!(None, result);

    let result = decode("$5\r\nhello").unwrap();
    assert_eq!(None, result);

    let result = decode("$5\r\nhello\r").unwrap();
    assert_eq!(None, result);

    let result = decode("$5\r\nhello\ra");
    assert!(matches!(
        result,
        Err(Error::Client(ClientError::CannotParseBulkString))
    ));
}

#[test]
fn array() -> Result<()> {
    let result = decode("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?;
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")),
            RespFrame::Array {
                len: 2,
                ranges: [
                    Range { start: 4, end: 15 },
                    Range { start: 15, end: 26 },
                    Range { start: 0, end: 0 },
                    Range { start: 0, end: 0 },
                    Range { start: 0, end: 0 }
                ]
            }
        )),
        result
    );

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
    assert_eq!(
        Some(RespResponse::Frame(
            RespBuf::from(Bytes::from_static(b"%1\r\n$5\r\nhello\r\n$5\r\nworld\r\n")),
            RespFrame::Map {
                len: 2,
                ranges: [
                    Range { start: 4, end: 15 },
                    Range { start: 15, end: 26 },
                    Range { start: 0, end: 0 },
                    Range { start: 0, end: 0 },
                    Range { start: 0, end: 0 }
                ]
            }
        )),
        result
    );

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
