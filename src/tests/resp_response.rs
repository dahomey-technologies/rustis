use crate::{
    Result,
    resp::{RespFrameParser, RespResponse, RespView},
};
use bytes::Bytes;

#[test]
fn array() -> Result<()> {
    let resp = Bytes::from_static(b"*6\r\n$4\r\nelt1\r\n$4\r\nelt2\r\n$4\r\nelt3\r\n$4\r\nelt4\r\n$4\r\nelt5\r\n$4\r\nelt6\r\n"); // ["elt1", "elt2", "elt3", "elt4", "elt5", "elt6"]
    let mut parser = RespFrameParser::new(&resp);
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
