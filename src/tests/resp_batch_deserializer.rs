use crate::{
    Result,
    resp::{RespBatchDeserializer, RespFrameParser, RespResponse, RespTapeMut},
};
use bytes::Bytes;
use serde::de::DeserializeOwned;

/// Parses a complete RESP reply into a self-contained response.
fn parse(resp: &'static [u8]) -> RespResponse {
    let resp = Bytes::from_static(resp);
    let mut tape = RespTapeMut::default();
    let mut parser = RespFrameParser::new(&resp, &mut tape);
    let (frame, _) = parser.parse().unwrap();
    RespResponse::new(resp.into(), frame)
}

/// Reads a batch of replies as `T`, the way a pipeline reads the replies to the
/// commands it queued.
fn read<T: DeserializeOwned>(replies: &[&'static [u8]]) -> Result<T> {
    let responses: Vec<RespResponse> = replies.iter().copied().map(parse).collect();
    let deserializer = RespBatchDeserializer::new(&responses);
    T::deserialize(&deserializer)
}

/// A batch of one is a sequence of one. A caller looping over a variable number
/// of commands reads `Vec<T>` whatever that number turns out to be, so the batch
/// must not stop being a collection when it happens to hold a single reply.
#[test]
fn one_reply_reads_as_a_one_element_collection() -> Result<()> {
    let values: Vec<Option<i64>> = read(&[b"$1\r\n1\r\n"])?;
    assert_eq!(vec![Some(1)], values);

    let values: Vec<Option<i64>> = read(&[b"_\r\n"])?;
    assert_eq!(vec![None], values);

    let values: Vec<String> = read(&[b"$6\r\nvalue1\r\n"])?;
    assert_eq!(vec!["value1".to_owned()], values);

    Ok(())
}

/// The other half of the same batch: asked for the reply's own type, a batch of
/// one is that reply. This is what a one-reply shortcut used to protect, and it
/// must hold without one.
#[test]
fn one_reply_still_reads_as_the_scalar() -> Result<()> {
    let value: Option<i64> = read(&[b"$1\r\n1\r\n"])?;
    assert_eq!(Some(1), value);

    let value: Option<i64> = read(&[b"_\r\n"])?;
    assert_eq!(None, value);

    let value: i64 = read(&[b":42\r\n"])?;
    assert_eq!(42, value);

    let value: String = read(&[b"$6\r\nvalue1\r\n"])?;
    assert_eq!("value1", value);

    let value: bool = read(&[b"#t\r\n"])?;
    assert!(value);

    Ok(())
}

/// A tuple is the batch of a caller who knows its length: one element per reply,
/// down to the batch of one, which `examples/pipelining.rs` reads as `(String,)`.
#[test]
fn one_reply_reads_as_a_one_element_tuple() -> Result<()> {
    let (value,): (String,) = read(&[b"+OK\r\n"])?;
    assert_eq!("OK", value);

    // The reply's own elements are not the batch's: a one-reply batch holds one
    // element whatever that reply is made of.
    let (array,): (Vec<String>,) = read(&[b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"])?;
    assert_eq!(vec!["foo".to_owned(), "bar".to_owned()], array);

    Ok(())
}

/// Above one reply nothing is ambiguous: every form is the batch.
#[test]
fn several_replies_read_as_the_batch() -> Result<()> {
    let (value1, value2): (String, String) = read(&[b"$6\r\nvalue1\r\n", b"$6\r\nvalue2\r\n"])?;
    assert_eq!("value1", value1);
    assert_eq!("value2", value2);

    let values: Vec<Option<i64>> = read(&[b"$1\r\n1\r\n", b"_\r\n"])?;
    assert_eq!(vec![Some(1), None], values);

    Ok(())
}

/// An empty batch is what a pipeline of forgotten commands leaves, and `()` is
/// what such a pipeline is read as.
#[test]
fn an_empty_batch_reads_as_the_unit() -> Result<()> {
    read::<()>(&[])?;

    let values: Vec<String> = read(&[])?;
    assert!(values.is_empty());

    Ok(())
}

/// A reply read for nothing but its success still has to surface its failure:
/// `()` over an error is an error, not a discarded reply.
#[test]
fn an_error_reply_is_surfaced_whichever_form_reads_it() {
    read::<()>(&[b"-ERR boom\r\n"]).expect_err("the unit reading must surface the error");
    read::<Vec<i64>>(&[b"-ERR boom\r\n"]).expect_err("the collection reading must surface it too");
    read::<i64>(&[b"-ERR boom\r\n"]).expect_err("the scalar reading must surface it too");
}
