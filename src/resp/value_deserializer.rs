use crate::{
    ClientError, Error, Result,
    resp::{
        Value,
        util::{bool_from_text, double_to_int, is_field_value_array},
    },
};
use serde::{
    Deserializer,
    de::{DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor},
    forward_to_deserialize_any,
};
use std::{
    collections::{HashMap, hash_map},
    slice, str, vec,
};

/// Reads a string that must hold exactly one character. The slice pattern keeps
/// the length test and the read as one expression, so neither can drift from the
/// other; UTF-8 validity then makes a single byte a single ASCII character.
#[inline]
fn single_char(str: &str) -> Result<char> {
    match str.as_bytes() {
        &[b] => Ok(b as char),
        _ => Err(Error::Client(ClientError::CannotParseChar)),
    }
}

/// Reads the single integer of a one-element array, the only array shape that
/// unwraps to a number: a longer one would have to discard the rest silently.
/// The slice pattern keeps the arity test and the read as one expression, and
/// the rule is the wire path's, so the two deserializers cannot disagree on
/// which arrays are readable as an integer.
#[inline]
fn single_integer(values: &[Value]) -> Result<i64> {
    match values {
        [Value::Integer(i)] => Ok(*i),
        _ => Err(Error::Client(ClientError::CannotParseInteger)),
    }
}

impl<'de> Deserializer<'de> for &'de Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::SimpleString(s) => visitor.visit_borrowed_str(s),
            Value::Integer(i) => visitor.visit_i64(*i),
            Value::Double(d) => visitor.visit_f64(*d),
            Value::BulkString(bs) => visitor.visit_borrowed_bytes(bs),
            Value::Boolean(b) => visitor.visit_bool(*b),
            Value::Array(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Map(values) => visitor.visit_map(MapAccess::new(values)),
            Value::Set(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Push(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            Value::Null => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i != 0,
            Value::Double(d) => *d != 0.,
            Value::Null => false,
            Value::SimpleString(s) => bool_from_text(s.as_bytes())?,
            Value::BulkString(s) => bool_from_text(s)?,
            Value::Boolean(b) => *b,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseBoolean));
            }
        };

        visitor.visit_bool(result)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                i8::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<i8>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i8>()?,
            Value::SimpleString(s) => s.parse::<i8>()?,
            Value::Array(a) => i8::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_i8(result)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                i16::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<i16>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i16>()?,
            Value::SimpleString(s) => s.parse::<i16>()?,
            Value::Array(a) => i16::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_i16(result)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                i32::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<i32>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i32>()?,
            Value::SimpleString(s) => s.parse::<i32>()?,
            Value::Array(a) => i32::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_i32(result)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i128::from(*i),
            Value::Double(d) => double_to_int::<i128>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i128>()?,
            Value::SimpleString(s) => s.parse::<i128>()?,
            Value::Array(a) => i128::from(single_integer(a)?),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_i128(result)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                u128::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<u128>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u128>()?,
            Value::SimpleString(s) => s.parse::<u128>()?,
            Value::Array(a) => u128::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_u128(result)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i,
            Value::Double(d) => double_to_int::<i64>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i64>()?,
            Value::SimpleString(s) => s.parse::<i64>()?,
            Value::Array(a) => single_integer(a)?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_i64(result)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                u8::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<u8>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u8>()?,
            Value::SimpleString(s) => s.parse::<u8>()?,
            Value::Array(a) => u8::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_u8(result)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                u16::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<u16>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u16>()?,
            Value::SimpleString(s) => s.parse::<u16>()?,
            Value::Array(a) => u16::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_u16(result)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                u32::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<u32>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u32>()?,
            Value::SimpleString(s) => s.parse::<u32>()?,
            Value::Array(a) => u32::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_u32(result)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => {
                u64::try_from(*i).map_err(|_| Error::Client(ClientError::CannotParseInteger))?
            }
            Value::Double(d) => double_to_int::<u64>(*d)?,
            Value::Null => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u64>()?,
            Value::SimpleString(s) => s.parse::<u64>()?,
            Value::Array(a) => u64::try_from(single_integer(a)?)
                .map_err(|_| Error::Client(ClientError::CannotParseInteger))?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseInteger));
            }
        };

        visitor.visit_u64(result)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            #[expect(
                clippy::cast_precision_loss,
                reason = "asking for a float is asking for an approximation: unlike an \
                      integer target, where a rounded value would be read as exact, \
                      the requested type is itself the caller's precision bound"
            )]
            Value::Integer(i) => *i as f32,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "asking for a float is asking for an approximation: unlike an \
                      integer target, where a rounded value would be read as exact, \
                      the requested type is itself the caller's precision bound"
            )]
            Value::Double(d) => *d as f32,
            Value::BulkString(bs) => str::from_utf8(bs)?.parse::<f32>()?,
            Value::Null => 0.,
            Value::SimpleString(s) => s.parse::<f32>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseDouble));
            }
        };

        visitor.visit_f32(result)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            #[expect(
                clippy::cast_precision_loss,
                reason = "asking for a float is asking for an approximation: unlike an \
                      integer target, where a rounded value would be read as exact, \
                      the requested type is itself the caller's precision bound"
            )]
            Value::Integer(i) => *i as f64,
            Value::Double(d) => *d,
            Value::BulkString(bs) => str::from_utf8(bs)?.parse::<f64>()?,
            Value::Null => 0.,
            Value::SimpleString(s) => s.parse::<f64>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseDouble));
            }
        };

        visitor.visit_f64(result)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result: char = match self {
            Value::BulkString(bs) => single_char(str::from_utf8(bs)?)?,
            Value::SimpleString(str) => single_char(str)?,
            Value::Null => '\0',
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => return Err(Error::Client(ClientError::CannotParseChar)),
        };

        visitor.visit_char(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => str::from_utf8(s)?,
            Value::Null => "",
            Value::SimpleString(s) => s.as_str(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            // Nothing to borrow: a number or a boolean holds no text of its own,
            // so it is rendered, and the rendering lives in `deserialize_string`
            // so the two entry points cannot disagree on which replies are
            // readable as text, nor on the text they produce. Serde reaches this
            // one through `deserialize_identifier` — struct field names, enum
            // variant names — and the other for a `String`, so a caller's choice
            // of target type must not decide whether their command succeeds.
            Value::Integer(_) | Value::Double(_) | Value::Boolean(_) => {
                return self.deserialize_string(visitor);
            }
            _ => {
                return Err(Error::Client(ClientError::CannotParseStr));
            }
        };

        visitor.visit_borrowed_str(result)
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Each arm hands the visitor the cheapest form it can use: text that
        // already lives in the `Value` stays borrowed, and a value that has to be
        // rendered goes out as a `&str`, so a visitor that does not need to own it
        // — a `Cow`, a field name — copies nothing.
        match self {
            Value::BulkString(s) => visitor.visit_borrowed_str(str::from_utf8(s)?),
            Value::SimpleString(s) => visitor.visit_borrowed_str(s),
            // `itoa` rather than `to_string`, which pulls in the `fmt` machinery
            // and allocates a `String` only to hand it over.
            Value::Integer(i) => {
                let mut buffer = itoa::Buffer::new();
                visitor.visit_str(buffer.format(*i))
            }
            Value::Double(d) => visitor.visit_string(d.to_string()),
            Value::Boolean(b) => visitor.visit_str(if *b { "true" } else { "false" }),
            Value::Null => visitor.visit_borrowed_str(""),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(ClientError::CannotParseString)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => s.as_slice(),
            Value::Null => &[],
            Value::SimpleString(s) => s.as_bytes(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseBytes));
            }
        };

        visitor.visit_borrowed_bytes(result)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => s.clone(),
            Value::Null => vec![],
            Value::SimpleString(s) => s.as_bytes().to_vec(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(ClientError::CannotParseBytes));
            }
        };

        visitor.visit_byte_buf(result)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Only a nil is `None`. An empty array, map or set is a collection that
        // happens to be empty — a different fact from a missing key, and the one
        // `LRANGE` on an empty list reports.
        match self {
            Value::Null => visitor.visit_none(),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Integer(_) => visitor.visit_unit(),
            Value::SimpleString(_) => visitor.visit_unit(),
            Value::BulkString(bs) if bs.is_empty() => visitor.visit_unit(),
            Value::Array(a) if a.is_empty() => visitor.visit_unit(),
            Value::Set(s) if s.is_empty() => visitor.visit_unit(),
            Value::Map(m) if m.is_empty() => visitor.visit_unit(),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(ClientError::CannotParseNil)),
        }
    }

    // Unit struct means a named value containing no data.
    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_seq(NilSeqAccess),
            Value::Array(values) | Value::Set(values) | Value::Push(values) => {
                visitor.visit_seq(SeqAccess::new(values))
            }
            Value::Map(values) => visitor.visit_seq(MapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(ClientError::CannotParseSequence)),
        }
    }

    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Array(values) => visitor.visit_map(SeqAccess::new(values)),
            Value::Map(values) => visitor.visit_map(MapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(ClientError::CannotParseMap)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        /// The first element of a flat array when it is a string, which is what
        /// [`is_field_value_array`] classifies the array on. Any other kind of
        /// element gives `None` — a positional array.
        fn first_key(values: &[Value]) -> Option<&[u8]> {
            match values.first() {
                Some(Value::SimpleString(s)) => Some(s.as_bytes()),
                Some(Value::BulkString(bs)) => Some(bs),
                _ => None,
            }
        }

        match self {
            Value::Array(values) => {
                if is_field_value_array(values.len(), first_key(values), fields) {
                    visitor.visit_map(SeqAccess::new(values))
                } else {
                    visitor.visit_seq(SeqAccess::new(values))
                }
            }
            Value::Map(values) => visitor.visit_map(MapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(ClientError::CannotParseStruct)),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::BulkString(bs) => {
                // Visit a unit variant.
                let str = str::from_utf8(bs)?;
                visitor.visit_enum(str.into_deserializer())
            }
            Value::SimpleString(str) => {
                // Visit a unit variant.
                visitor.visit_enum(str.as_str().into_deserializer())
            }
            // Visit a newtype variant, tuple variant, or struct variant
            // as an array of 2 elements
            Value::Array(a) => visitor.visit_enum(Enum::from_array(a)?),
            // Same, encoded as a map of 1 element
            Value::Map(m) => visitor.visit_enum(Enum::from_map(m)?),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(ClientError::CannotParseEnum)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct NilSeqAccess;

impl<'de> serde::de::SeqAccess<'de> for NilSeqAccess {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        _seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Ok(None)
    }
}

struct SeqAccess<'de> {
    iter: slice::Iter<'de, Value>,
    len: usize,
    value: Option<&'de Value>,
}

impl<'de> SeqAccess<'de> {
    pub(crate) fn new(values: &'de [Value]) -> Self {
        Self {
            len: values.len(),
            iter: values.iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the remaining count is only decremented when the iterator yielded \
                  an element, so it never passes zero."
    )]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                self.len -= 1;
                seed.deserialize(value).map(Some)
            }
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

/// in RESP, arrays can be seen as maps with a succession of keys and their values
impl<'de> serde::de::MapAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(entry) => {
                if let Value::Array(values) = entry
                    && let [key, value] = values.as_slice()
                {
                    self.value = Some(value);
                    seed.deserialize(key).map(Some)
                } else {
                    seed.deserialize(entry).map(Some)
                }
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => match self.iter.next() {
                Some(value) => seed.deserialize(value),
                None => Err(serde::de::Error::custom(
                    "SeqAccess::next_value_seed: value is missing",
                )),
            },
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len / 2)
    }
}

struct MapAccess<'de> {
    len: usize,
    iter: hash_map::Iter<'de, Value, Value>,
    value: Option<&'de Value>,
}

impl<'de> MapAccess<'de> {
    pub(crate) fn new(values: &'de HashMap<Value, Value>) -> Self {
        Self {
            len: values.len(),
            iter: values.iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for MapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing in map")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> serde::de::SeqAccess<'de> for MapAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => seed.deserialize(ValuePair(key, value)).map(Some),
            None => Ok(None),
        }
    }
}

struct ValuePair<'de>(&'de Value, &'de Value);

impl<'de> Deserializer<'de> for ValuePair<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(2, visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        pub(crate) struct ValuePairSeqAccess<'de> {
            first: Option<&'de Value>,
            second: Option<&'de Value>,
        }

        impl<'de> serde::de::SeqAccess<'de> for ValuePairSeqAccess<'de> {
            type Error = Error;

            fn next_element_seed<T>(
                &mut self,
                seed: T,
            ) -> std::result::Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                if let Some(first) = self.first.take() {
                    seed.deserialize(first).map(Some)
                } else if let Some(second) = self.second.take() {
                    seed.deserialize(second).map(Some)
                } else {
                    Ok(None)
                }
            }
        }

        visitor.visit_seq(ValuePairSeqAccess {
            first: Some(self.0),
            second: Some(self.1),
        })
    }
}

struct Enum<'de> {
    variant_identifier: &'de Value,
    variant_value: &'de Value,
}

impl<'de> Enum<'de> {
    /// Reads the 2-element array form. The slice pattern *is* the length test,
    /// so the check and the two reads are one expression and cannot drift apart
    /// — the caller no longer holds an invariant this function depends on.
    fn from_array(values: &'de [Value]) -> Result<Self> {
        match values {
            [variant_identifier, variant_value] => Ok(Self {
                variant_identifier,
                variant_value,
            }),
            _ => Err(Error::Client(ClientError::CannotParseEnum)),
        }
    }

    /// Reads the 1-element map form. A `HashMap` has no slice pattern, so the
    /// cardinality is tested by asking for a second entry and requiring none.
    fn from_map(values: &'de HashMap<Value, Value>) -> Result<Self> {
        let mut iter = values.iter();
        match (iter.next(), iter.next()) {
            (Some((variant_identifier, variant_value)), None) => Ok(Self {
                variant_identifier,
                variant_value,
            }),
            _ => Err(Error::Client(ClientError::CannotParseEnum)),
        }
    }
}

impl<'de> EnumAccess<'de> for Enum<'de> {
    type Error = Error;
    type Variant = &'de Value;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(self.variant_identifier)?;
        Ok((val, self.variant_value))
    }
}

impl<'de> VariantAccess<'de> for &'de Value {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::Client(ClientError::Unexpected))
    }

    // Newtype variants are represented as map so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    // Tuple variants are represented as map of array so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Struct variants are represented as map of map so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}
