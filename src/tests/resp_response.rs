use crate::{
    Result,
    resp::{RespFrameParser, RespResponse, RespTapeMut, RespView},
};
use bytes::Bytes;

/// Parses a complete RESP reply, owning a copy of its bytes.
fn parse_owned(resp: &[u8]) -> RespResponse {
    let resp = Bytes::copy_from_slice(resp);
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    RespResponse::new(resp.into(), frame)
}

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
    let view = response.view()?;
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
        iter.next().unwrap().unwrap().view().unwrap()
    );
    assert_eq!(
        RespView::BulkString(b"bar"),
        iter.next().unwrap().unwrap().view().unwrap()
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

/// The parser's own output is logged on a few debug paths. It has no buffer to
/// decode against, so it reports its shape — and never the tape, whose raw bytes
/// are unreadable and larger than the reply.
#[test]
fn a_parsed_frame_reports_its_shape_instead_of_the_tape() {
    let resp = Bytes::from_static(b"*2\r\n$3\r\nfoo\r\n:1\r\n");
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();

    assert_eq!("Collection { nodes: 4 }", format!("{frame:?}"));

    let mut tape = RespTapeMut::default();
    let (frame, _) = RespFrameParser::new(b":12\r\n", &mut tape).parse().unwrap();
    assert_eq!("Scalar { at: 0 }", format!("{frame:?}"));
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
        .map(|r| match r.unwrap().view().unwrap() {
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

#[test]
fn into_array_iter_accepts_every_container_tag() {
    // The four container tags index their elements the same way, so all four are
    // iterable. A map yields its keys and values flattened in wire order, and a
    // push yields its kind as the first element.
    for resp in [
        b"*2\r\n:1\r\n:2\r\n".as_slice(),
        b"%1\r\n:1\r\n:2\r\n".as_slice(),
        b"~2\r\n:1\r\n:2\r\n".as_slice(),
        b">2\r\n:1\r\n:2\r\n".as_slice(),
    ] {
        let response = parse_owned(resp);
        let values: Vec<i64> = response
            .into_array_iter()
            .unwrap()
            .map(|r| r.unwrap().to::<i64>().unwrap())
            .collect();
        assert_eq!(vec![1i64, 2], values, "failed on {}", resp[0] as char);
    }

    // A scalar has no elements to walk.
    assert!(parse_owned(b":1\r\n").into_array_iter().is_err());
}

/// An error reply is surfaced as the Redis error itself rather than as an empty
/// sequence, so a caller iterating a reply cannot mistake a failure for no rows.
#[test]
fn into_array_iter_on_an_error_reply_yields_the_redis_error() {
    let err = parse_owned(b"-ERR nope\r\n").into_array_iter();
    assert!(matches!(err, Err(crate::Error::Redis(_))));
}

#[test]
fn a_synthesized_response_reads_back_its_value() {
    // The cluster and the cache build responses that never came off the wire.
    // They must read back like a parsed one.
    assert_eq!(42i64, RespResponse::integer(42).to::<i64>().unwrap());
    assert_eq!(None, RespResponse::null().to::<Option<String>>().unwrap());
    assert_eq!("OK", RespResponse::ok().to::<String>().unwrap());
    assert_eq!(
        vec![1i64, 2],
        RespResponse::integer_array(vec![1, 2])
            .to::<Vec<i64>>()
            .unwrap()
    );
    assert_eq!(
        vec![1i64, 2],
        RespResponse::owned_array(vec![RespResponse::integer(1), RespResponse::integer(2)])
            .to::<Vec<i64>>()
            .unwrap()
    );
}

#[test]
fn compact_preserves_the_value_it_copies_out() {
    // Compacting releases the shared block a response was carved from. Whatever
    // it does to the representation, the value read back must be identical.
    let scalar = parse_owned(b"$5\r\nhello\r\n");
    assert_eq!("hello", scalar.compact().to::<String>().unwrap());

    let integer = parse_owned(b":12\r\n");
    assert_eq!(12i64, integer.compact().to::<i64>().unwrap());

    let double = parse_owned(b",12.5\r\n");
    assert_eq!(12.5f64, double.compact().to::<f64>().unwrap());

    let null = parse_owned(b"_\r\n");
    assert_eq!(None, null.compact().to::<Option<String>>().unwrap());

    let collection = parse_owned(b"*2\r\n$3\r\nfoo\r\n:1\r\n");
    assert_eq!(
        r#"Array([BulkString("foo"), Integer(1)])"#,
        format!("{:?}", collection.compact())
    );

    let synthesized = RespResponse::owned_array(vec![RespResponse::integer(7)]);
    assert_eq!(vec![7i64], synthesized.compact().to::<Vec<i64>>().unwrap());
}
