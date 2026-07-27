use crate::{
    client::RespLimits,
    resp::{ParsedFrame, RespBuf, RespFrameParser, RespResponse, RespTapeMut, RespView},
};

/// Parses a complete frame with a throwaway tape buffer.
fn parse(resp: &[u8]) -> crate::Result<(ParsedFrame, usize)> {
    let mut tape = RespTapeMut::default();
    RespFrameParser::new(resp, &mut tape).parse()
}

/// Parses a complete frame and pairs it with its bytes, so the test asserts on
/// what the frame reads back as rather than on how it is represented.
fn parse_response(resp: &[u8]) -> crate::Result<(RespResponse, usize)> {
    let (frame, len) = parse(resp)?;
    Ok((RespResponse::new(RespBuf::from_slice(resp), frame), len))
}

#[test]
fn parse_array() {
    let resp = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"; // ["foo", "bar"]
    let (response, len) = parse_response(resp).unwrap();

    assert_eq!(22, len);
    assert!(matches!(response.view(), Ok(RespView::Array(_))));
    assert_eq!(
        vec!["foo".to_owned(), "bar".to_owned()],
        response.to::<Vec<String>>().unwrap()
    );
}

#[test]
fn parse_null_array() {
    // `*-1\r\n` is a legal RESP2 null array and must decode to Null, not to a
    // collection of `usize::MAX` elements.
    let resp = b"*-1\r\n";
    let (response, len) = parse_response(resp).unwrap();

    assert_eq!(5, len);
    assert!(matches!(response.view(), Ok(RespView::Null)));
}

#[test]
fn parse_negative_array_length_errors() {
    // A negative length other than -1 must be rejected, not wrapped to a huge
    // `usize` element count.
    let resp = b"*-2\r\n";
    assert!(parse(resp).is_err());
}

#[test]
fn parse_negative_bulk_string_length_errors() {
    // A bulk-string length other than -1 (nil) must be rejected, not
    // fed to `pos + len as usize + 2` where it overflows.
    let resp = b"$-2\r\n";
    assert!(parse(resp).is_err());
}

#[test]
fn parse_negative_bulk_error_length_errors() {
    // The bulk-error arm had no negative-length guard at all.
    let resp = b"!-2\r\n";
    assert!(parse(resp).is_err());
}

#[test]
fn parse_negative_bulk_lengths_inside_collection_error() {
    // The same negative-length guards must hold for an element parsed via the
    // tape builder, not only for a top-level frame.
    let resp = b"*1\r\n$-2\r\n";
    assert!(parse(resp).is_err());
    let resp = b"*1\r\n!-2\r\n";
    assert!(parse(resp).is_err());
}

#[test]
fn parse_deeply_nested_frame_is_rejected_not_overflowing() {
    // A crafted `*1\r\n*1\r\n…` reply must be rejected by the depth
    // guard instead of recursing `emit_value` into an uncatchable stack
    // overflow. 100_000 levels would blow any stack without the bound.
    let mut resp = b"*1\r\n".repeat(100_000);
    resp.extend_from_slice(b":1\r\n");
    assert!(parse(&resp).is_err());
}

#[test]
fn parse_nesting_within_limit_succeeds() {
    // A frame nested just under the bound must still parse: the guard rejects
    // pathology, not legitimately nested replies.
    let depth = 100;
    let mut resp = b"*1\r\n".repeat(depth);
    resp.extend_from_slice(b":7\r\n");
    assert!(parse(&resp).is_ok());
}

#[test]
fn parse_oversized_bulk_string_length_is_rejected_before_payload() {
    // A header declaring more than MAX_BULK_LENGTH (512 MiB) must be
    // rejected outright, not returned as `EOF` — otherwise the streaming
    // decoder would keep buffering, waiting for bytes that never come.
    let resp = b"$536870913\r\n"; // 512 MiB + 1, no payload
    assert!(matches!(
        parse(resp),
        Err(crate::Error::Client(crate::ClientError::BulkLengthTooLarge))
    ));
}

#[test]
fn parse_oversized_collection_length_is_rejected() {
    // A collection cardinality beyond MAX_COLLECTION_LENGTH must be
    // rejected before the element loop runs.
    let resp = b"*134217729\r\n"; // 128 Mi + 1 elements
    assert!(matches!(
        parse(resp),
        Err(crate::Error::Client(
            crate::ClientError::CollectionLengthTooLarge
        ))
    ));
}

#[test]
fn parse_oversized_map_length_is_rejected() {
    // The map arm doubles the declared length; a value that overflows the cap
    // only after doubling must still be caught.
    let resp = b"%67108865\r\n"; // 64 Mi + 1 pairs => 128 Mi + 2 elements
    assert!(matches!(
        parse(resp),
        Err(crate::Error::Client(
            crate::ClientError::CollectionLengthTooLarge
        ))
    ));
}

#[test]
fn parse_leading_attribute_is_skipped_and_reply_decodes() {
    // An attribute frame may precede any reply. The parser must skip
    // it and decode the underlying reply normally, without a self-inflicted
    // reconnect.
    // |1\r\n$3\r\nfoo\r\n$3\r\nbar\r\n  then  :42\r\n
    let resp = b"|1\r\n$3\r\nfoo\r\n$3\r\nbar\r\n:42\r\n";
    let (response, len) = parse_response(resp).unwrap();

    assert_eq!(resp.len(), len);
    assert!(matches!(response.view(), Ok(RespView::Integer(42, _))));
}

#[test]
fn parse_attribute_preceding_an_array_element_is_skipped() {
    // Attributes can precede an element inside a collection, so the skip must
    // happen at frame-dispatch level, not only at the top level.
    // *2\r\n :1\r\n  |1\r\n$1\r\na\r\n$1\r\nb\r\n :2\r\n
    let resp = b"*2\r\n:1\r\n|1\r\n$1\r\na\r\n$1\r\nb\r\n:2\r\n";
    let (response, len) = parse_response(resp).unwrap();

    assert_eq!(resp.len(), len);
    assert!(matches!(response.view(), Ok(RespView::Array(_))));
    assert_eq!(vec![1i64, 2], response.to::<Vec<i64>>().unwrap());
}

#[test]
fn parse_big_number_is_exposed_as_its_string_payload() {
    // A big number does not fit in an i64 and is surfaced as its
    // decimal-string payload.
    let resp = b"(3492890328409238509324850943850943825024385\r\n";
    let (response, len) = parse_response(resp).unwrap();

    assert_eq!(resp.len(), len);
    assert!(matches!(
        response.view(),
        Ok(RespView::BulkString(
            b"3492890328409238509324850943850943825024385"
        ))
    ));
}

#[test]
fn parse_map() {
    let resp = b"%1\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"; // {"foo": "bar"}
    let (response, len) = parse_response(resp).unwrap();

    assert_eq!(22, len);
    assert!(matches!(response.view(), Ok(RespView::Map(_))));
    let map = response
        .to::<std::collections::HashMap<String, String>>()
        .unwrap();
    assert_eq!(1, map.len());
    assert_eq!(Some(&"bar".to_owned()), map.get("foo"));
}

/// Parses a complete frame under caller-chosen limits.
fn parse_with_limits(resp: &[u8], limits: RespLimits) -> crate::Result<(ParsedFrame, usize)> {
    let mut tape = RespTapeMut::default();
    RespFrameParser::with_limits(resp, &mut tape, limits).parse()
}

#[test]
fn a_lowered_nesting_limit_rejects_a_frame_the_default_accepts() {
    // 50 levels are far below the 128 default, so only the lowered limit can
    // reject this frame — proving the knob is the value actually enforced.
    let mut resp = b"*1\r\n".repeat(50);
    resp.extend_from_slice(b":7\r\n");
    assert!(parse(&resp).is_ok());

    let limits = RespLimits {
        max_nesting_depth: 10,
        ..Default::default()
    };
    assert!(matches!(
        parse_with_limits(&resp, limits),
        Err(crate::Error::Client(
            crate::ClientError::MaxNestingDepthExceeded
        ))
    ));
}

#[test]
fn a_lowered_bulk_limit_rejects_a_length_the_default_accepts() {
    let resp = b"$16\r\n0123456789abcdef\r\n";
    assert!(parse(resp).is_ok());

    let limits = RespLimits {
        max_bulk_length: 8,
        ..Default::default()
    };
    assert!(matches!(
        parse_with_limits(resp, limits),
        Err(crate::Error::Client(crate::ClientError::BulkLengthTooLarge))
    ));
}

#[test]
fn a_lowered_collection_limit_rejects_a_cardinality_the_default_accepts() {
    let resp = b"*3\r\n:1\r\n:2\r\n:3\r\n";
    assert!(parse(resp).is_ok());

    let limits = RespLimits {
        max_collection_length: 2,
        ..Default::default()
    };
    assert!(matches!(
        parse_with_limits(resp, limits),
        Err(crate::Error::Client(
            crate::ClientError::CollectionLengthTooLarge
        ))
    ));
}

#[test]
fn a_raised_bulk_limit_reads_back_without_being_re_capped() {
    // Raising the cap must work end-to-end: the frame has to parse *and* read
    // back. The read-back path re-derives each scalar's layout, so if it re-applied
    // the default cap instead of trusting the already-validated frame, a value
    // legal under the raised limit would decode as garbage or vanish.
    let payload = "x".repeat(600);
    let resp = format!("*1\r\n${}\r\n{payload}\r\n", payload.len()).into_bytes();

    let limits = RespLimits {
        max_bulk_length: 1024,
        ..Default::default()
    };
    let mut tape = RespTapeMut::default();
    let (frame, len) = RespFrameParser::with_limits(&resp, &mut tape, limits)
        .parse()
        .unwrap();
    assert_eq!(resp.len(), len);

    let response = RespResponse::new(RespBuf::from_slice(&resp), frame);
    assert_eq!(vec![payload], response.to::<Vec<String>>().unwrap());
}

#[test]
fn a_lowered_bulk_limit_is_enforced_inside_a_collection() {
    // The element loop reaches bulk values through `scalar_end`, a separate
    // path from a top-level scalar. An oversized element there must be rejected
    // outright rather than reported as EOF, which would leave the streaming
    // decoder buffering for bytes that will never come.
    let resp = b"*1\r\n$16\r\n0123456789abcdef\r\n";
    assert!(parse(resp).is_ok());

    let limits = RespLimits {
        max_bulk_length: 8,
        ..Default::default()
    };
    assert!(matches!(
        parse_with_limits(resp, limits),
        Err(crate::Error::Client(crate::ClientError::BulkLengthTooLarge))
    ));
}
