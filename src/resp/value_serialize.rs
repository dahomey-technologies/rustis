use crate::resp::{Value, ERROR_FAKE_FIELD, PUSH_FAKE_FIELD, SET_FAKE_FIELD};
use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeTupleStruct},
    Serialize,
};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::SimpleString(s) => serializer.serialize_str(s),
            Value::Integer(i) => serializer.serialize_i64(*i),
            Value::Double(d) => serializer.serialize_f64(*d),
            Value::BulkString(bs) => serializer.serialize_bytes(bs),
            Value::Boolean(b) => serializer.serialize_bool(*b),
            Value::Array(a) => {
                let mut seq = serializer.serialize_seq(Some(a.len()))?;
                for e in a {
                    seq.serialize_element(e)?;
                }
                seq.end()
            }
            Value::Map(m) => {
                let mut map = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
            Value::Set(s) => {
                let mut ts = serializer.serialize_tuple_struct(SET_FAKE_FIELD, s.len())?;
                for e in s {
                    ts.serialize_field(e)?;
                }
                ts.end()
            }
            Value::Push(p) => {
                let mut ts = serializer.serialize_tuple_struct(PUSH_FAKE_FIELD, p.len())?;
                for e in p {
                    ts.serialize_field(e)?;
                }
                ts.end()
            }
            Value::Error(e) => {
                serializer.serialize_newtype_struct(ERROR_FAKE_FIELD, e.to_string().as_str())
            }
            Value::Nil => serializer.serialize_unit(),
        }
    }
}
