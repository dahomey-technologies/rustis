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
