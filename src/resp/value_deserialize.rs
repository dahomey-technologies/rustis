use crate::resp::Value;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, SeqAccess, Visitor},
};
use std::{collections::HashMap, fmt};

pub(crate) const PUSH_FAKE_FIELD: &str = ">>>PUSH>>>";

/// Implementation meant to be used with [`RespDeserializer`](crate::resp::RespDeserializer)
impl<'de> Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any valid resp::Value")
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Boolean(v))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Integer(v))
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Double(v))
    }

    #[inline]
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Value, E> {
        Ok(Value::SimpleString(v.to_owned()))
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::SimpleString(v.to_owned()))
    }

    #[inline]
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::SimpleString(v))
    }

    #[inline]
    fn visit_none<E>(self) -> std::result::Result<Value, E> {
        Ok(Value::Nil)
    }

    fn visit_borrowed_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::BulkString(v.to_vec()))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::BulkString(v.to_vec()))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Value, E> {
        Ok(Value::BulkString(v))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let len = seq.size_hint();

        if let Some(0) = len {
            Ok(Value::Nil)
        } else {
            let mut values: Vec<Value> = Vec::with_capacity(len.unwrap_or_default());
            loop {
                match seq.next_element()? {
                    None => break,
                    Some(value) => values.push(value),
                };
            }
            Ok(Value::Array(values))
        }
    }

    fn visit_map<A>(self, mut map: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let len = map.size_hint();

        if let Some(0) = len {
            Ok(Value::Nil)
        } else {
            let mut values: HashMap<Value, Value> = HashMap::with_capacity(len.unwrap_or_default());
            loop {
                let key = match map.next_key::<PushOrKey>()? {
                    None => break,
                    Some(PushOrKey::Push) => {
                        let values: Vec<Value> = map.next_value()?;
                        if values.is_empty() {
                            return Ok(Value::Nil);
                        } else {
                            return Ok(Value::Push(values));
                        }
                    }
                    Some(PushOrKey::Key(key)) => key,
                };

                values.insert(key, map.next_value()?);
            }
            if values.is_empty() {
                Ok(Value::Nil)
            } else {
                Ok(Value::Map(values))
            }
        }
    }
}

enum PushOrKey {
    Push,
    Key(Value),
}

impl<'de> Deserialize<'de> for PushOrKey {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(PushOrKeyVisitor)
    }
}

struct PushOrKeyVisitor;

impl<'de> Visitor<'de> for PushOrKeyVisitor {
    type Value = PushOrKey;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("PushOrKey")
    }

    #[inline]
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<PushOrKey, E> {
        let value_visitor = ValueVisitor;
        value_visitor.visit_bool(v).map(PushOrKey::Key)
    }

    #[inline]
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<PushOrKey, E> {
        let value_visitor = ValueVisitor;
        value_visitor.visit_i64(v).map(PushOrKey::Key)
    }

    #[inline]
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<PushOrKey, E> {
        let value_visitor = ValueVisitor;
        value_visitor.visit_f64(v).map(PushOrKey::Key)
    }

    #[inline]
    fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> Result<PushOrKey, E> {
        if v == PUSH_FAKE_FIELD {
            Ok(PushOrKey::Push)
        } else {
            let value_visitor = ValueVisitor;
            value_visitor.visit_borrowed_str(v).map(PushOrKey::Key)
        }
    }

    #[inline]
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<PushOrKey, E> {
        if v == PUSH_FAKE_FIELD {
            Ok(PushOrKey::Push)
        } else {
            let value_visitor = ValueVisitor;
            value_visitor.visit_str(v).map(PushOrKey::Key)
        }
    }

    // null BulkString
    #[inline]
    fn visit_none<E: serde::de::Error>(self) -> std::result::Result<PushOrKey, E> {
        let value_visitor = ValueVisitor;
        value_visitor.visit_none().map(PushOrKey::Key)
    }

    #[inline]
    fn visit_borrowed_bytes<E: serde::de::Error>(
        self,
        v: &'de [u8],
    ) -> std::result::Result<PushOrKey, E> {
        let value_visitor = ValueVisitor;
        value_visitor.visit_borrowed_bytes(v).map(PushOrKey::Key)
    }

    #[inline]
    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> std::result::Result<PushOrKey, E> {
        let value_visitor = ValueVisitor;
        value_visitor.visit_bytes(v).map(PushOrKey::Key)
    }

    #[inline]
    fn visit_seq<A>(self, seq: A) -> Result<PushOrKey, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let value_visitor = ValueVisitor;
        value_visitor.visit_seq(seq).map(PushOrKey::Key)
    }

    #[inline]
    fn visit_map<A>(self, map: A) -> Result<PushOrKey, A::Error>
    where
        A: MapAccess<'de>,
    {
        let value_visitor = ValueVisitor;
        value_visitor.visit_map(map).map(PushOrKey::Key)
    }
}
