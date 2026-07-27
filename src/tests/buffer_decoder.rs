use crate::{
    ClientError, Error, Result,
    client::{BufferConfig, RespLimits},
    resp::{BufferDecoder, RespResponse, RespView},
};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

fn decode(str: &str) -> Result<Option<RespResponse>> {
    let mut buffer_decoder = BufferDecoder::new();
    let mut buf: BytesMut = str.into();
    buffer_decoder.decode(&mut buf)
}

/// Decodes `str`, which must hold exactly one complete frame.
fn decode_one(str: &str) -> RespResponse {
    decode(str).unwrap().expect("one complete frame")
}

#[test]
fn integer() {
    let response = decode_one(":12\r\n");
    assert!(matches!(response.view(), Ok(RespView::Integer(12))));

    let result = decode(":12\r").unwrap();
    assert_eq!(None, result);

    let result = decode(":12").unwrap();
    assert_eq!(None, result);
}

#[test]
fn string() -> Result<()> {
    let response = decode_one("+OK\r\n");
    assert!(matches!(response.view(), Ok(RespView::SimpleString(b"OK"))));

    let result = decode("+OK\r")?;
    assert_eq!(None, result);

    let result = decode("+OK")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn error() -> Result<()> {
    let response = decode_one("-ERR error\r\n");
    assert!(response.is_error());
    assert!(matches!(response.view(), Ok(RespView::Error(b"ERR error"))));

    let result = decode("-ERR error\r")?;
    assert_eq!(None, result);

    let result = decode("-ERR error")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn double() -> Result<()> {
    let response = decode_one(",12.12\r\n");
    assert!(matches!(response.view(), Ok(RespView::Double(d)) if d == 12.12));

    let result = decode(",12.12\r")?;
    assert_eq!(None, result);

    let result = decode(",12.12")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn bool() -> Result<()> {
    let response = decode_one("#f\r\n");
    assert!(matches!(response.view(), Ok(RespView::Boolean(false))));

    let result = decode("#f\r")?;
    assert_eq!(None, result);

    let result = decode("#f")?;
    assert_eq!(None, result);

    // Unlike a malformed number, a malformed boolean is a *framing* failure: the
    // frame is `#` plus exactly one of `t`/`f` plus CRLF, so anything else means
    // the frame boundary itself is unknown.
    let result = decode("#a\r\n");
    assert!(matches!(
        result,
        Err(Error::Client(ClientError::CannotParseBoolean))
    ));

    Ok(())
}

#[test]
fn null() -> Result<()> {
    let response = decode_one("_\r\n");
    assert!(matches!(response.view(), Ok(RespView::Null)));

    let result = decode("_\r")?;
    assert_eq!(None, result);

    let result = decode("_")?;
    assert_eq!(None, result);

    Ok(())
}

#[test]
fn bulk_string() {
    let response = decode_one("$5\r\nhello\r\n");
    assert!(matches!(
        response.view(),
        Ok(RespView::BulkString(b"hello"))
    ));

    // A bulk string is length-prefixed, so an embedded CRLF is payload.
    let response = decode_one("$7\r\nhel\r\nlo\r\n");
    assert!(matches!(
        response.view(),
        Ok(RespView::BulkString(b"hel\r\nlo"))
    ));

    let response = decode_one("$0\r\n\r\n");
    assert!(matches!(response.view(), Ok(RespView::BulkString(b""))));

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
    assert!(matches!(response.view(), Ok(RespView::Array(_))));
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
    assert!(matches!(response.view(), Ok(RespView::Map(_))));
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
fn a_nested_collection_resumes_byte_by_byte_across_chunks() {
    // Fed one byte at a time, a reply must stay incomplete until its last byte
    // and then decode correctly — proof the parser suspends and resumes instead
    // of failing or restarting. The tape is built once, across chunks; the *win*
    // (no re-scan) is what the chunked bench measures.
    let full = b"*2\r\n*2\r\n:1\r\n:2\r\n*2\r\n:3\r\n:4\r\n";
    let mut decoder = BufferDecoder::new();
    let mut buf = BytesMut::new();
    let mut completed = None;

    for (i, &byte) in full.iter().enumerate() {
        buf.extend_from_slice(&[byte]);
        let result = decoder.decode(&mut buf).unwrap();
        if i + 1 < full.len() {
            assert!(result.is_none(), "frame completed early at byte {i}");
        } else {
            completed = result;
        }
    }

    let response = completed.expect("the last byte completes the frame");
    assert_eq!(
        vec![vec![1i64, 2], vec![3, 4]],
        response.to::<Vec<Vec<i64>>>().unwrap()
    );
    assert!(buf.is_empty(), "the completed frame must be consumed");
}

#[test]
fn a_collection_split_at_every_boundary_resumes_without_corruption() {
    // Cutting between chunk 1 and chunk 2 at every byte offset must always yield
    // the same result — the resume path is exercised at each offset (including
    // mid-header and mid-bulk-string), the exact class of boundary the old
    // re-parsing decoder mishandled.
    let full = b"*3\r\n$5\r\nhello\r\n$5\r\nworld\r\n$3\r\nfoo\r\n";
    let expected = vec!["hello".to_owned(), "world".to_owned(), "foo".to_owned()];

    for split in 1..full.len() {
        let mut decoder = BufferDecoder::new();
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&full[..split]);
        assert!(
            decoder.decode(&mut buf).unwrap().is_none(),
            "a proper prefix must be incomplete (split {split})"
        );
        buf.extend_from_slice(&full[split..]);
        let response = decoder
            .decode(&mut buf)
            .unwrap()
            .expect("the remaining bytes complete the frame");
        assert_eq!(
            expected,
            response.to::<Vec<String>>().unwrap(),
            "wrong result when split at {split}"
        );
        assert!(buf.is_empty());
    }
}

#[test]
fn a_top_level_scalar_split_across_chunks_resumes() {
    // Top-level scalars carry no tape and no stack, so their resume path is the
    // empty-stack re-dispatch. Every split must still decode.
    let full = b"$11\r\nhello world\r\n";
    for split in 1..full.len() {
        let mut decoder = BufferDecoder::new();
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&full[..split]);
        assert!(
            decoder.decode(&mut buf).unwrap().is_none(),
            "incomplete at split {split}"
        );
        buf.extend_from_slice(&full[split..]);
        let response = decoder.decode(&mut buf).unwrap().expect("complete");
        assert_eq!("hello world", response.to::<String>().unwrap());
    }
}

#[test]
fn pipelined_frames_decode_one_then_resume_the_next() {
    // A read can deliver one whole frame plus the start of the next. The decoder
    // must return the first, consume exactly its bytes, then resume the second —
    // proof that completing a frame resets the resume state cleanly.
    let mut decoder = BufferDecoder::new();
    let mut buf = BytesMut::new();
    buf.extend_from_slice(b"*1\r\n:1\r\n*2\r\n:2\r\n");

    let first = decoder
        .decode(&mut buf)
        .unwrap()
        .expect("first frame complete");
    assert_eq!(vec![1i64], first.to::<Vec<i64>>().unwrap());

    assert!(
        decoder.decode(&mut buf).unwrap().is_none(),
        "the second frame is buffered but incomplete"
    );

    buf.extend_from_slice(b":3\r\n");
    let second = decoder
        .decode(&mut buf)
        .unwrap()
        .expect("second frame complete");
    assert_eq!(vec![2i64, 3], second.to::<Vec<i64>>().unwrap());
    assert!(buf.is_empty());
}

#[test]
fn tape_buffer_shrinks_back_after_a_large_collection_spike() {
    // The shrink policy applied to the decoder's recycled tape buffer: a single
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
    for _ in 0..BufferConfig::default().shrink_hysteresis + 2 {
        let mut small = BytesMut::new();
        small.extend_from_slice(b"*1\r\n:1\r\n");
        drop(decoder.decode(&mut small).unwrap());
    }

    assert!(
        decoder.tape_capacity() <= BufferConfig::default().tape_capacity,
        "tape buffer should shrink back to the target after a spike, got {} bytes",
        decoder.tape_capacity()
    );
}

#[test]
fn the_tape_shrinks_at_the_configured_capacity_and_hysteresis() {
    // Same policy as the test above, but driven entirely from `BufferConfig`
    // rather than the historical constants: a caller who lowers the target and
    // shortens the hysteresis must see the tape released sooner and smaller.
    let buffers = BufferConfig {
        tape_capacity: 4 * 1024,
        shrink_factor: 2,
        shrink_hysteresis: 3,
        ..Default::default()
    };
    let mut decoder = BufferDecoder::with_config(buffers, RespLimits::default());

    // 5_000 elements => 5_002 nodes => ~39 KiB of tape, past 2 x 4 KiB.
    let mut big = format!("*{}\r\n", 5_000).into_bytes();
    for _ in 0..5_000 {
        big.extend_from_slice(b":1\r\n");
    }
    let mut buf = BytesMut::new();
    buf.extend_from_slice(&big);
    drop(decoder.decode(&mut buf).unwrap());

    // Two quiet frames are one short of the configured hysteresis of 3.
    for _ in 0..2 {
        let mut small = BytesMut::new();
        small.extend_from_slice(b"*1\r\n:1\r\n");
        drop(decoder.decode(&mut small).unwrap());
    }
    assert!(
        decoder.tape_capacity() > buffers.tape_capacity,
        "must not shrink before the configured hysteresis"
    );

    let mut small = BytesMut::new();
    small.extend_from_slice(b"*1\r\n:1\r\n");
    drop(decoder.decode(&mut small).unwrap());
    assert!(
        decoder.tape_capacity() <= buffers.tape_capacity,
        "tape should shrink to the configured target, got {} bytes",
        decoder.tape_capacity()
    );
}

#[test]
fn the_decoder_enforces_the_configured_parser_limits() {
    // The decoder builds the parser, so a limit set on the config must reach it
    // — otherwise the knob would only work on the one-shot parsing path.
    let limits = RespLimits {
        max_collection_length: 2,
        ..Default::default()
    };
    let mut decoder = BufferDecoder::with_config(BufferConfig::default(), limits);
    let mut buf = BytesMut::new();
    buf.extend_from_slice(b"*3\r\n:1\r\n:2\r\n:3\r\n");

    assert!(matches!(
        decoder.decode(&mut buf),
        Err(Error::Client(ClientError::CollectionLengthTooLarge))
    ));
}

#[test]
fn a_malformed_scalar_frames_and_fails_at_read() {
    // RESP framing only needs the `\r\n`, so a scalar whose payload does not
    // parse still delimits a frame. Rejecting it at decode time would tear down
    // the socket and take every other in-flight command with it; rejecting it at
    // read time fails only the command that received it, and the stream stays
    // aligned on the next frame.
    for malformed in [":a\r\n", ",abc\r\n"] {
        let response = decode_one(malformed);
        assert!(
            response.to::<i64>().is_err(),
            "{malformed} must fail at read"
        );
    }
}

#[test]
fn a_malformed_scalar_leaves_the_next_frame_readable() {
    // The point of framing a malformed payload: the bytes after it are still a
    // frame boundary, so a pipelined reply behind a bad one still decodes.
    let mut decoder = BufferDecoder::new();
    let mut buf = BytesMut::new();
    buf.extend_from_slice(b":a\r\n:12\r\n");

    let bad = decoder
        .decode(&mut buf)
        .unwrap()
        .expect("first frame framed");
    assert!(bad.to::<i64>().is_err());

    let good = decoder
        .decode(&mut buf)
        .unwrap()
        .expect("second frame complete");
    assert_eq!(12i64, good.to::<i64>().unwrap());
    assert!(buf.is_empty());
}

#[test]
fn a_top_level_null_collection_decodes_as_null() {
    // `*-1\r\n` is a RESP2 null array: a collection tag with no collection. The
    // decoder must surface it as Null, and its `*` must never be read back as a
    // scalar.
    let response = decode_one("*-1\r\n");
    assert!(matches!(response.view(), Ok(RespView::Null)));
    assert_eq!(None, response.to::<Option<String>>().unwrap());
}

#[test]
fn a_top_level_scalar_reads_the_same_value_twice() {
    // A response is read once on the normal path but repeatedly when retained
    // (a cache entry). Reading must be idempotent, not consume anything.
    let response = decode_one(":12\r\n");
    assert_eq!(12i64, response.to::<i64>().unwrap());
    assert_eq!(12i64, response.to::<i64>().unwrap());

    let response = decode_one("$5\r\nhello\r\n");
    assert_eq!("hello", response.to::<String>().unwrap());
    assert_eq!("hello", response.to::<String>().unwrap());
}
