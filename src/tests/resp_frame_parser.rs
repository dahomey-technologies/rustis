use crate::resp::{RespFrame, RespFrameParser};
use std::ops::Range;

#[test]
fn parse_array() {
    let resp = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"; // ["foo", "bar"]
    let mut parser = RespFrameParser::new(resp);
    let (frame, len) = parser.parse().unwrap();

    println!("{frame:?}");
    assert_eq!(22, len);
    assert!(matches!(
        frame,
        RespFrame::Array {
            len: 2,
            ranges: [
                Range { start: 4, end: 13 },
                Range { start: 13, end: 22 },
                Range { start: 0, end: 0 },
                Range { start: 0, end: 0 },
                Range { start: 0, end: 0 }
            ]
        }
    ));
}

#[test]
fn parse_null_array() {
    // `*-1\r\n` is a legal RESP2 null array and must decode to Null, not to a
    // collection of `usize::MAX` elements.
    let resp = b"*-1\r\n";
    let mut parser = RespFrameParser::new(resp);
    let (frame, len) = parser.parse().unwrap();

    assert_eq!(5, len);
    assert!(matches!(frame, RespFrame::Null));
}

#[test]
fn parse_negative_array_length_errors() {
    // A negative length other than -1 must be rejected, not wrapped to a huge
    // `usize` element count.
    let resp = b"*-2\r\n";
    let mut parser = RespFrameParser::new(resp);
    assert!(parser.parse().is_err());
}

#[test]
fn parse_negative_bulk_string_length_errors() {
    // RESP-03: a bulk-string length other than -1 (nil) must be rejected, not
    // fed to `pos + len as usize + 2` where it overflows.
    let resp = b"$-2\r\n";
    let mut parser = RespFrameParser::new(resp);
    assert!(parser.parse().is_err());
}

#[test]
fn parse_negative_bulk_error_length_errors() {
    // RESP-03: the bulk-error arm had no negative-length guard at all.
    let resp = b"!-2\r\n";
    let mut parser = RespFrameParser::new(resp);
    assert!(parser.parse().is_err());
}

#[test]
fn parse_range_negative_bulk_lengths_error() {
    // RESP-03: `parse_range` re-validates nothing on its own, so cover both
    // negative-length arms there too.
    let resp = b"$-2\r\n";
    assert!(
        RespFrameParser::new(resp)
            .parse_range(0..resp.len())
            .is_err()
    );
    let resp = b"!-2\r\n";
    assert!(
        RespFrameParser::new(resp)
            .parse_range(0..resp.len())
            .is_err()
    );
}

#[test]
fn parse_deeply_nested_frame_is_rejected_not_overflowing() {
    // HARD-01: a crafted `*1\r\n*1\r\n…` reply must be rejected by the depth
    // guard instead of recursing `parse_value` into an uncatchable stack
    // overflow. 100_000 levels would blow any stack without the bound.
    let mut resp = b"*1\r\n".repeat(100_000);
    resp.extend_from_slice(b":1\r\n");
    let mut parser = RespFrameParser::new(&resp);
    assert!(parser.parse().is_err());
}

#[test]
fn parse_nesting_within_limit_succeeds() {
    // A frame nested just under the bound must still parse: the guard rejects
    // pathology, not legitimately nested replies.
    let depth = 100;
    let mut resp = b"*1\r\n".repeat(depth);
    resp.extend_from_slice(b":7\r\n");
    let mut parser = RespFrameParser::new(&resp);
    assert!(parser.parse().is_ok());
}

#[test]
fn parse_map() {
    let resp = b"%1\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"; // {"foo": "bar"}
    let mut parser = RespFrameParser::new(resp);
    let (frame, len) = parser.parse().unwrap();

    println!("{frame:?}");
    assert_eq!(22, len);
    assert!(matches!(
        frame,
        RespFrame::Map {
            len: 2,
            ranges: [
                Range { start: 4, end: 13 },
                Range { start: 13, end: 22 },
                Range { start: 0, end: 0 },
                Range { start: 0, end: 0 },
                Range { start: 0, end: 0 }
            ]
        }
    ));
}
