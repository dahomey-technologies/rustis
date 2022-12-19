use std::collections::HashMap;

use crate::{resp::Value, Result};
use smallvec::SmallVec;

#[test]
fn map_to_tuple_vec() -> Result<()> {
    let value = Value::Map(HashMap::from([
        (
            Value::BulkString(b"field1".to_vec()),
            Value::BulkString(b"hello".to_vec()),
        ),
        (
            Value::BulkString(b"field2".to_vec()),
            Value::BulkString(b"world".to_vec()),
        ),
    ]));

    let values: Vec<(String, String)> = value.into()?;
    assert_eq!(2, values.len());
    assert!(values
        .iter()
        .any(|i| *i == ("field1".to_owned(), "hello".to_owned())));
    assert!(values
        .iter()
        .any(|i| *i == ("field2".to_owned(), "world".to_owned())));

    Ok(())
}

#[test]
fn map_to_tuple_smallvec() -> Result<()> {
    let value = Value::Map(HashMap::from([
        (
            Value::BulkString(b"field1".to_vec()),
            Value::BulkString(b"hello".to_vec()),
        ),
        (
            Value::BulkString(b"field2".to_vec()),
            Value::BulkString(b"world".to_vec()),
        ),
    ]));

    let values: SmallVec<[(String, String); 2]> = value.into()?;
    assert_eq!(2, values.len());
    assert!(values
        .iter()
        .any(|i| *i == ("field1".to_owned(), "hello".to_owned())));
    assert!(values
        .iter()
        .any(|i| *i == ("field2".to_owned(), "world".to_owned())));

    Ok(())
}
