use crate::{
    Result,
    resp::{RespFrameParser, RespResponse, RespView},
};
use bytes::{Bytes, BytesMut};

#[test]
fn array() -> Result<()> {
    let resp = Bytes::from_static(b"*6\r\n$4\r\nelt1\r\n$4\r\nelt2\r\n$4\r\nelt3\r\n$4\r\nelt4\r\n$4\r\nelt5\r\n$4\r\nelt6\r\n"); // ["elt1", "elt2", "elt3", "elt4", "elt5", "elt6"]
    let mut tape = BytesMut::new();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse()?;
    let response = RespResponse::new(resp.into(), frame);
    let view = response.view();
    assert!(matches!(view, RespView::Array(_)));

    let RespView::Array(array) = view else {
        unreachable!()
    };

    let mut it = array.into_iter();
    assert_eq!(Some(RespView::BulkString(b"elt1")), it.next());
    assert_eq!(Some(RespView::BulkString(b"elt2")), it.next());
    assert_eq!(Some(RespView::BulkString(b"elt3")), it.next());
    assert_eq!(Some(RespView::BulkString(b"elt4")), it.next());
    assert_eq!(Some(RespView::BulkString(b"elt5")), it.next());
    assert_eq!(Some(RespView::BulkString(b"elt6")), it.next());
    assert_eq!(None, it.next());

    Ok(())
}

#[test]
fn into_array_iter() {
    let resp = Bytes::from_static(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    let mut tape = BytesMut::new();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    let response = RespResponse::new(resp.into(), frame);
    let mut iter = response.into_array_iter().unwrap();

    assert_eq!(RespView::BulkString(b"foo"), iter.next().unwrap().view());
    assert_eq!(RespView::BulkString(b"bar"), iter.next().unwrap().view());
    assert_eq!(None, iter.next());
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
    let mut tape = BytesMut::new();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    let response = RespResponse::new(resp.into(), frame);
    let iter = response.into_array_iter().unwrap();

    let values: Vec<_> = iter
        .map(|r| match r.view() {
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
