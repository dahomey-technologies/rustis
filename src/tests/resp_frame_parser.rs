use crate::{
    Result,
    resp::{RespFrame, RespFrameParser},
};
use std::ops::Range;

#[test]
fn parse_array() -> Result<()> {
    let resp = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"; // ["foo", "bar"]
    let mut parser = RespFrameParser::new(resp);
    let (frame, len) = parser.parse()?;

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

    Ok(())
}
