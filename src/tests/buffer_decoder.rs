use crate::{
    ClientError, Error, Result,
    resp::{
        BufferDecoder, RespBuf, RespFrame, RespResponse, TAPE_SHRINK_HYSTERESIS,
        TARGET_TAPE_CAPACITY,
    },
};
use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

fn decode(str: &str) -> Result<Option<RespResponse>> {
    let mut buffer_decoder = BufferDecoder::new();
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
    let response = decode("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?.expect("a complete array frame");
    assert!(matches!(
        response,
        RespResponse::Frame(_, RespFrame::Array { root: 0, .. })
    ));
    assert_eq!(
        vec!["hello".to_owned(), "world".to_owned()],
        response.to::<Vec<String>>()?
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
    let response = decode("%1\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?.expect("a complete map frame");
    assert!(matches!(
        response,
        RespResponse::Frame(_, RespFrame::Map { root: 0, .. })
    ));
    let map = response.to::<std::collections::HashMap<String, String>>()?;
    assert_eq!(Some(&"world".to_owned()), map.get("hello"));

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

#[test]
fn tape_buffer_shrinks_back_after_a_large_collection_spike() {
    // HARD-03 policy applied to the decoder's recycled tape buffer: a single
    // huge collection must not inflate it permanently. Because a post-`split()`
    // tail can pin the whole block while reporting near-zero `capacity()`, the
    // decoder tracks the oversized state directly and releases the block after a
    // quiet streak.
    let mut decoder = BufferDecoder::new();

    // 70_000 elements => 70_002 nodes => ~547 KiB of tape, past 8 x 64 KiB.
    let mut big = format!("*{}\r\n", 70_000).into_bytes();
    for _ in 0..70_000 {
        big.extend_from_slice(b":1\r\n");
    }
    let mut buf = BytesMut::new();
    buf.extend_from_slice(&big);
    let response = decoder.decode(&mut buf).unwrap();
    assert!(response.is_some());
    drop(response); // release the frozen tape so its block becomes reclaimable

    // Small collections (which do touch the tape) reclaim the oversized block,
    // then trip the reset once the hysteresis window elapses.
    for _ in 0..TAPE_SHRINK_HYSTERESIS + 2 {
        let mut small = BytesMut::new();
        small.extend_from_slice(b"*1\r\n:1\r\n");
        drop(decoder.decode(&mut small).unwrap());
    }

    assert!(
        decoder.tape_capacity() <= TARGET_TAPE_CAPACITY,
        "tape buffer should shrink back to the target after a spike, got {} bytes",
        decoder.tape_capacity()
    );
}
