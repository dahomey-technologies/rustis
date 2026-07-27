use crate::{
    Result,
    resp::{RespFrameParser, RespResponse, RespTapeMut, RespView},
};
use bytes::Bytes;

/// Parses a complete RESP reply into a self-contained response.
fn parse(resp: &'static [u8]) -> RespResponse {
    let resp = Bytes::from_static(resp);
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    RespResponse::new(resp.into(), frame)
}

#[test]
fn array() -> Result<()> {
    let resp = Bytes::from_static(b"*6\r\n$4\r\nelt1\r\n$4\r\nelt2\r\n$4\r\nelt3\r\n$4\r\nelt4\r\n$4\r\nelt5\r\n$4\r\nelt6\r\n"); // ["elt1", "elt2", "elt3", "elt4", "elt5", "elt6"]
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse()?;
    let response = RespResponse::new(resp.into(), frame);
    let view = response.view();
    assert!(matches!(view, RespView::Array(_)));

    let RespView::Array(array) = view else {
        unreachable!()
    };

    let mut it = array.into_iter();
    assert_eq!(RespView::BulkString(b"elt1"), it.next().unwrap()?);
    assert_eq!(RespView::BulkString(b"elt2"), it.next().unwrap()?);
    assert_eq!(RespView::BulkString(b"elt3"), it.next().unwrap()?);
    assert_eq!(RespView::BulkString(b"elt4"), it.next().unwrap()?);
    assert_eq!(RespView::BulkString(b"elt5"), it.next().unwrap()?);
    assert_eq!(RespView::BulkString(b"elt6"), it.next().unwrap()?);
    assert!(it.next().is_none());

    Ok(())
}

#[test]
fn into_array_iter() {
    let resp = Bytes::from_static(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    let response = RespResponse::new(resp.into(), frame);
    let mut iter = response.into_array_iter().unwrap();

    assert_eq!(
        RespView::BulkString(b"foo"),
        iter.next().unwrap().unwrap().view()
    );
    assert_eq!(
        RespView::BulkString(b"bar"),
        iter.next().unwrap().unwrap().view()
    );
    assert!(iter.next().is_none());
}

/// A response is logged with `{:?}` on the connection debug path, so its
/// rendering must show the decoded reply — never the parse tape, an internal
/// index whose raw bytes are both unreadable and larger than the reply itself.
#[test]
fn debug_renders_the_reply_without_the_tape() {
    let response = parse(b"*2\r\n$3\r\nfoo\r\n:1\r\n");
    let rendered = format!("{response:?}");

    assert_eq!(r#"Array([BulkString("foo"), Integer(1)])"#, rendered);
}

#[test]
fn debug_renders_a_map_as_key_value_pairs() {
    let response = parse(b"%2\r\n$2\r\nid\r\n:12\r\n$4\r\nmode\r\n$7\r\ncluster\r\n");
    let rendered = format!("{response:?}");

    assert_eq!(
        r#"Map({BulkString("id"): Integer(12), BulkString("mode"): BulkString("cluster")})"#,
        rendered
    );
}

#[test]
fn debug_renders_nested_collections() {
    let response = parse(b"*2\r\n*2\r\n:1\r\n:2\r\n~1\r\n$1\r\na\r\n");
    let rendered = format!("{response:?}");

    assert_eq!(
        r#"Array([Array([Integer(1), Integer(2)]), Set([BulkString("a")])])"#,
        rendered
    );
}

/// Scalars built without a buffer (`RespResponse::integer`, `::null`) carry
/// their value in the frame alone: the rendering must read the frame, not the
/// empty buffer.
#[test]
fn debug_renders_a_bufferless_scalar() {
    assert_eq!("Integer(42)", format!("{:?}", RespResponse::integer(42)));
    assert_eq!("Null", format!("{:?}", RespResponse::null()));
    assert_eq!("SimpleString(\"OK\")", format!("{:?}", RespResponse::ok()));
}

/// Debug logging a multi-megabyte reply must not build a multi-megabyte string.
#[test]
fn debug_truncates_a_large_reply() {
    let mut resp = format!("*{}\r\n", 1000).into_bytes();
    for _ in 0..1000 {
        resp.extend_from_slice(b"$10\r\nelement123\r\n");
    }
    let resp = Bytes::from(resp);
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    let response = RespResponse::new(resp.into(), frame);

    let rendered = format!("{response:?}");

    assert!(rendered.ends_with("<truncated>"), "got: {rendered}");
    assert!(
        rendered.len() < 1100,
        "rendering too long: {}",
        rendered.len()
    );
}

/// A bare frame is logged on its own in a few places (parser assertions); it has
/// no buffer to decode against, so it renders its shape — still never the tape.
#[test]
fn frame_debug_reports_the_shape_instead_of_the_tape() {
    let resp = Bytes::from_static(b"*2\r\n$3\r\nfoo\r\n:1\r\n");
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();

    assert_eq!("Array { root: 0, len: 2 }", format!("{frame:?}"));
}

/// Regression test: iterating a collection must yield correct data for every
/// element regardless of position. A previous design cached only the first 5
/// element ranges and re-parsed the rest through a fallback that produced ranges
/// relative to a sub-slice while binding them to the full buffer, corrupting
/// elements 6+. The tape indexes every element uniformly, removing that path.
#[test]
fn into_array_iter_beyond_inline_ranges() {
    // 8 bulk strings — well past the 5 the old design cached inline.
    let resp = Bytes::from_static(
        b"*8\r\n$4\r\nelt1\r\n$4\r\nelt2\r\n$4\r\nelt3\r\n$4\r\nelt4\r\n$4\r\nelt5\r\n$4\r\nelt6\r\n$4\r\nelt7\r\n$4\r\nelt8\r\n",
    );
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    let response = RespResponse::new(resp.into(), frame);
    let iter = response.into_array_iter().unwrap();

    let values: Vec<_> = iter
        .map(|r| match r.unwrap().view() {
            RespView::BulkString(b) => b.to_vec(),
            other => panic!("unexpected view: {other:?}"),
        })
        .collect();

    assert_eq!(
        vec![
            b"elt1".to_vec(),
            b"elt2".to_vec(),
            b"elt3".to_vec(),
            b"elt4".to_vec(),
            b"elt5".to_vec(),
            b"elt6".to_vec(),
            b"elt7".to_vec(),
            b"elt8".to_vec(),
        ],
        values
    );
}
