use crate::resp::{ArgSerializer, BulkString, RefBulkString};
use bytes::BytesMut;
use serde::Serialize;

#[test]
pub fn byte_slice() {
    let mut buffer = BytesMut::new();
    let mut serializer = ArgSerializer::from_buffer(&mut buffer);
    RefBulkString::from(b"foo")
        .serialize(&mut serializer)
        .unwrap();
    RefBulkString::from(b"bar")
        .serialize(&mut serializer)
        .unwrap();
    assert_eq!(
        "$3\r\nfoo\r\n$3\r\nbar\r\n",
        str::from_utf8(buffer.freeze().as_ref()).unwrap()
    );
}

#[test]
pub fn bute_vec() {
    let mut buffer = BytesMut::new();
    let mut serializer = ArgSerializer::from_buffer(&mut buffer);
    BulkString::from(b"foo".to_vec())
        .serialize(&mut serializer)
        .unwrap();
    BulkString::from(b"bar".to_vec())
        .serialize(&mut serializer)
        .unwrap();
    assert_eq!(
        "$3\r\nfoo\r\n$3\r\nbar\r\n",
        str::from_utf8(buffer.freeze().as_ref()).unwrap()
    );
}
