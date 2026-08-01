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
