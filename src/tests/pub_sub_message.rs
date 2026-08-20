use crate::{
    ClientError, ErrorKind, Result,
    client::PubSubMessage,
    resp::{RespBuf, RespFrameParser, RespResponse, RespTapeMut},
};
use bytes::Bytes;

/// Builds the `RespResponse` a push frame decodes to, the way the network
/// handler hands one to the stream.
fn response(buf: &[u8]) -> Result<RespResponse> {
    let mut tape = RespTapeMut::default();
    let (frame, len) = RespFrameParser::new(buf, &mut tape).parse()?;
    Ok(RespResponse::new(
        RespBuf::from(Bytes::copy_from_slice(&buf[..len])),
        frame,
    ))
}

/// A push frame of `kind` carrying `parts` as bulk strings.
fn push(kind: &str, parts: &[&[u8]]) -> Vec<u8> {
    let mut buf = format!(">{}\r\n${}\r\n{kind}\r\n", parts.len() + 1, kind.len()).into_bytes();
    for part in parts {
        buf.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        buf.extend_from_slice(part);
        buf.extend_from_slice(b"\r\n");
    }
    buf
}

fn convert(buf: &[u8]) -> Result<PubSubMessage> {
    PubSubMessage::try_from(&response(buf)?)
}

#[test]
fn message_carries_no_pattern() -> Result<()> {
    let message = convert(&push("message", &[b"mychannel", b"mymessage"]))?;

    assert_eq!(b"", message.pattern());
    assert_eq!(b"mychannel", message.channel());
    assert_eq!(b"mymessage", message.payload());

    Ok(())
}

#[test]
fn smessage_carries_no_pattern() -> Result<()> {
    let message = convert(&push("smessage", &[b"mychannel", b"mymessage"]))?;

    assert_eq!(b"", message.pattern());
    assert_eq!(b"mychannel", message.channel());
    assert_eq!(b"mymessage", message.payload());

    Ok(())
}

/// The three segments share one buffer, so the boundaries are what can go
/// wrong: a `pmessage` is the only shape where all three are non-empty.
#[test]
fn pmessage_keeps_its_three_segments_apart() -> Result<()> {
    let message = convert(&push(
        "pmessage",
        &[b"mychannel*", b"mychannel11", b"mymessage"],
    ))?;

    assert_eq!(b"mychannel*", message.pattern());
    assert_eq!(b"mychannel11", message.channel());
    assert_eq!(b"mymessage", message.payload());

    Ok(())
}

/// Empty segments are legal — `PUBLISH mychannel ""` is a valid publish — and
/// they are the degenerate case for offset arithmetic.
#[test]
fn empty_segments_stay_empty_and_distinct() -> Result<()> {
    let message = convert(&push("message", &[b"mychannel", b""]))?;
    assert_eq!(b"", message.pattern());
    assert_eq!(b"mychannel", message.channel());
    assert_eq!(b"", message.payload());

    let message = convert(&push("pmessage", &[b"*", b"", b""]))?;
    assert_eq!(b"*", message.pattern());
    assert_eq!(b"", message.channel());
    assert_eq!(b"", message.payload());

    Ok(())
}

/// The block is sized from the segments, so its length is the one thing every
/// payload size has to agree on.
#[test]
fn a_payload_of_any_size_reads_back_whole() -> Result<()> {
    for size in [1usize, 63, 64, 65, 1024, 64 * 1024] {
        let payload = vec![b'x'; size];
        let message = convert(&push("message", &[b"mychannel", &payload]))?;

        assert_eq!(b"", message.pattern());
        assert_eq!(b"mychannel", message.channel());
        assert_eq!(size, message.payload().len());
        assert_eq!(payload, message.payload());
    }

    Ok(())
}

/// A push the stream cannot be handed — a subscription confirmation, or a frame
/// that is not a push at all — is refused rather than delivered as a message.
#[test]
fn a_push_that_is_not_a_message_is_refused() -> Result<()> {
    for buf in [
        push("subscribe", &[b"mychannel", b"1"]),
        push("unsubscribe", &[b"mychannel", b"0"]),
        push("unknown", &[b"mychannel", b"mymessage"]),
        b"*3\r\n$7\r\nmessage\r\n$9\r\nmychannel\r\n$9\r\nmymessage\r\n".to_vec(),
    ] {
        let error = convert(&buf).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                ErrorKind::Client(ClientError::UnexpectedPubSubMessage)
            ),
            "expected UnexpectedPubSubMessage, got {error:?}"
        );
    }

    Ok(())
}

/// The text helpers are the reason the segments are not read as bytes by hand.
#[test]
fn the_segments_read_back_as_text() -> Result<()> {
    let message = convert(&push(
        "pmessage",
        &[b"mychannel*", b"mychannel11", b"mymessage"],
    ))?;

    assert_eq!("mychannel*", message.pattern_str()?);
    assert_eq!("mychannel11", message.channel_str()?);
    assert_eq!("mymessage", message.payload_as::<&str>()?);

    Ok(())
}

/// A channel name and a payload are both binary-safe, so text is a request the
/// message may have to refuse.
#[test]
fn a_segment_that_is_not_text_is_refused() -> Result<()> {
    let message = convert(&push("message", &[b"\xff", b"\xff"]))?;

    assert_eq!(b"\xff", message.channel());
    for error in [
        message.channel_str().unwrap_err(),
        message.payload_as::<&str>().unwrap_err(),
        message.payload_as::<String>().unwrap_err(),
    ] {
        assert!(
            matches!(error.kind(), ErrorKind::Utf8(_)),
            "expected a UTF-8 error, got {error:?}"
        );
    }

    Ok(())
}

/// A payload is read with the same serde machinery as a bulk string reply, so a
/// published number is read as a number rather than parsed by the caller.
#[test]
fn a_payload_is_read_as_a_rust_type() -> Result<()> {
    let message = convert(&push("message", &[b"mychannel", b"42"]))?;
    assert_eq!(42i64, message.payload_as::<i64>()?);
    assert_eq!(42u8, message.payload_as::<u8>()?);
    assert_eq!("42", message.payload_as::<String>()?);

    let message = convert(&push("message", &[b"mychannel", b"1.5"]))?;
    assert_eq!(1.5f64, message.payload_as::<f64>()?);

    // A payload that is not the type asked for is an error, not a zero.
    let message = convert(&push("message", &[b"mychannel", b"one"]))?;
    assert!(message.payload_as::<i64>().is_err());

    Ok(())
}

/// A published document is the common case for a payload that is not a scalar.
#[cfg(feature = "json")]
#[test]
fn a_json_payload_is_read_as_a_document() -> Result<()> {
    use crate::resp::Json;

    let message = convert(&push(
        "message",
        &[b"mychannel", br#"{"id":1,"name":"mike"}"#],
    ))?;

    let Json(document): Json<serde_json::Value> = message.payload_as()?;
    assert_eq!(1, document["id"]);
    assert_eq!("mike", document["name"]);

    Ok(())
}
