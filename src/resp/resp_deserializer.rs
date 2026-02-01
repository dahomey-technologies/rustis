use crate::{
    ClientError, Error, RedisError, Result,
    resp::{PUSH_FAKE_FIELD, RespArrayIter, RespArrayView, RespResponse, RespView},
};
use serde::{
    Deserializer,
    de::{self, IntoDeserializer, Visitor},
    forward_to_deserialize_any,
};
use std::str::{self};

/// Serde deserializer for [`RESP3`](https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md)
pub struct RespDeserializer<'de> {
    view: RespView<'de>,
}

impl<'de> RespDeserializer<'de> {
    /// Creates a new `RespDeserializer`
    #[inline]
    pub const fn new(view: RespView<'de>) -> Self {
        RespDeserializer { view }
    }
}

impl<'de> Deserializer<'de> for RespDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.view {
            RespView::SimpleString(_) => self.deserialize_str(visitor),
            RespView::Integer(i) => visitor.visit_i64(i),
            RespView::Double(d) => visitor.visit_f64(d),
            RespView::BulkString(bs) => visitor.visit_borrowed_bytes(bs),
            RespView::Boolean(b) => visitor.visit_bool(b),
            RespView::IntegerArray(a) => visitor.visit_seq(IntegerArraySeqAccess::new(a.iter())),
            RespView::OwnedArray(a) => visitor.visit_seq(OwnedArraySeqAccess::new(a.iter())),
            RespView::Array(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::Map(view) => visitor.visit_map(MapAccess::new(view.into_iter())),
            RespView::Set(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::Push(view) => visitor.visit_map(PushMapAccess::new(view)),
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => ss == b"OK",
            RespView::Integer(i) => i != 0,
            RespView::Double(d) => d != 0.,
            RespView::BulkString(bs) => matches!(bs, b"1" | b"true"),
            RespView::Boolean(b) => b,
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => false,
            _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
        };

        visitor.visit_bool(result)
    }

    #[inline]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as i128,
            RespView::Double(d) => d as i128,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as i128,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_i128(result)
    }

    #[inline]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as u128,
            RespView::Double(d) => d as u128,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as u128,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_u128(result)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i,
            RespView::Double(d) => d as i64,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_i64(result)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as u64,
            RespView::Double(d) => d as u64,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as u64,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_u64(result)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as i32,
            RespView::Double(d) => d as i32,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as i32,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_i32(result)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as u32,
            RespView::Double(d) => d as u32,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as u32,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_u32(result)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as i16,
            RespView::Double(d) => d as i16,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as i16,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_i16(result)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as u16,
            RespView::Double(d) => d as u16,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as u16,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_u16(result)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as i8,
            RespView::Double(d) => d as i8,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as i8,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_i8(result)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(ss) => {
                atoi::atoi(ss).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Integer(i) => i as u8,
            RespView::Double(d) => d as u8,
            RespView::BulkString(bs) => {
                atoi::atoi(bs).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
            }
            RespView::Array(a) => match a.into_iter().next() {
                Some(RespView::Integer(i)) => i as u8,
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            },
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            RespView::Null => 0,
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        };
        visitor.visit_u8(result)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result =
            match self.view {
                RespView::SimpleString(ss) => fast_float2::parse(ss)
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?,
                RespView::Integer(i) => i as f64,
                RespView::Double(d) => d,
                RespView::BulkString(bs) => fast_float2::parse(bs)
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?,
                RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
                RespView::Null => 0.0,
                _ => return Err(Error::Client(ClientError::CannotParseDouble)),
            };
        visitor.visit_f64(result)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result =
            match self.view {
                RespView::SimpleString(ss) => fast_float2::parse(ss)
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?,
                RespView::Integer(i) => i as f32,
                RespView::Double(d) => d as f32,
                RespView::BulkString(bs) => fast_float2::parse(bs)
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?,
                RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
                RespView::Null => 0.0,
                _ => return Err(Error::Client(ClientError::CannotParseDouble)),
            };
        visitor.visit_f32(result)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(b) | RespView::BulkString(b) => {
                let str = str::from_utf8(b)?;
                let mut chars = str.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => c,
                    _ => return Err(Error::Client(ClientError::CannotParseChar)),
                }
            }
            RespView::Null => '\0',
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            _ => return Err(Error::Client(ClientError::CannotParseChar)),
        };

        visitor.visit_char(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(b) | RespView::BulkString(b) => str::from_utf8(b)?,
            RespView::Null => "",
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            _ => return Err(Error::Client(ClientError::CannotParseStr)),
        };

        visitor.visit_borrowed_str(result)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(b) | RespView::BulkString(b) => str::from_utf8(b)?.to_owned(),
            RespView::Integer(i) => i.to_string(),
            RespView::Double(d) => d.to_string(),
            RespView::Boolean(b) => b.to_string(),
            RespView::Null => String::new(),
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            _ => return Err(Error::Client(ClientError::CannotParseString)),
        };

        visitor.visit_string(result)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(b) | RespView::BulkString(b) => b,
            RespView::Null => &[],
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            _ => return Err(Error::Client(ClientError::CannotParseBytes)),
        };

        visitor.visit_borrowed_bytes(result)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.view {
            RespView::SimpleString(b) | RespView::BulkString(b) => b.to_vec(),
            RespView::Null => Vec::new(),
            RespView::Error(e) => return Err(Error::Redis(RedisError::try_from(e)?)),
            _ => return Err(Error::Client(ClientError::CannotParseBytes)),
        };

        visitor.visit_byte_buf(result)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.view {
            RespView::Array(ref resp_array_view) => {
                if resp_array_view.len() == 0 {
                    visitor.visit_none()
                } else {
                    visitor.visit_some(self)
                }
            }
            RespView::Null => visitor.visit_none(),
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
            _ => visitor.visit_some(self),
        }
    }

    /// deserialize_unit basically means the next value should be ignored
    ///  expect if it is an error.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.view {
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
            _ => visitor.visit_unit(),
        }
    }

    /// Unit struct means a named value containing no data.
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
        match self.view {
            RespView::Array(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::IntegerArray(a) => visitor.visit_seq(IntegerArraySeqAccess::new(a.iter())),
            RespView::OwnedArray(a) => visitor.visit_seq(OwnedArraySeqAccess::new(a.iter())),
            RespView::Map(view) => visitor.visit_seq(MapAccess::new(view.into_iter())),
            RespView::Set(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::Push(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::Null => visitor.visit_seq(NilSeqAccess),
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
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
        match self.view {
            RespView::Array(view) => visitor.visit_map(SeqAccess::new(view.into_iter())),
            RespView::IntegerArray(a) => visitor.visit_map(IntegerArraySeqAccess::new(a.iter())),
            RespView::OwnedArray(a) => visitor.visit_map(OwnedArraySeqAccess::new(a.iter())),
            RespView::Map(view) => visitor.visit_map(MapAccess::new(view.into_iter())),
            RespView::Set(view) => visitor.visit_map(SeqAccess::new(view.into_iter())),
            RespView::Push(view) => visitor.visit_map(SeqAccess::new(view.into_iter())),
            RespView::Null => visitor.visit_map(NilSeqAccess),
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
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
        #[inline]
        fn check_resp2_array(
            view: RespArrayView<'_>,
            fields: &'static [&'static str],
        ) -> Result<bool> {
            if view.len() >= 2 * fields.len() {
                if let Some(RespView::BulkString(bs)) = view.into_iter().next()
                    && fields.iter().any(|f| f.as_bytes() == bs)
                {
                    Ok(true)
                } else {
                    Err(Error::Client(ClientError::CannotParseStruct))
                }
            } else if view.len() == fields.len() {
                Ok(false)
            } else {
                Err(Error::Client(ClientError::CannotParseStruct))
            }
        }

        match self.view {
            RespView::Array(view) => {
                if check_resp2_array(view.clone(), fields)? {
                    visitor.visit_map(SeqAccess::new(view.into_iter()))
                } else {
                    visitor.visit_seq(SeqAccess::new(view.into_iter()))
                }
            }
            RespView::Set(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::Push(view) => visitor.visit_seq(SeqAccess::new(view.into_iter())),
            RespView::Map(view) => visitor.visit_map(MapAccess::new(view.into_iter())),
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
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
        match self.view {
            RespView::SimpleString(b) | RespView::BulkString(b) => {
                // Visit a unit variant.
                let str = str::from_utf8(b)?;
                visitor.visit_enum(str.into_deserializer())
            }
            RespView::Map(view) => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as a map of 1 element
                let mut iter = view.into_iter();
                if let (Some(key), Some(val), None) = (iter.next(), iter.next(), iter.next()) {
                    visitor.visit_enum(EnumAccess::new(key, val))
                } else {
                    Err(Error::Client(ClientError::CannotParseEnum))
                }
            }
            RespView::Array(view) | RespView::Set(view) | RespView::Push(view) => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as an array of 2 elements
                let mut iter = view.into_iter();
                if let (Some(key), Some(val), None) = (iter.next(), iter.next(), iter.next()) {
                    visitor.visit_enum(EnumAccess::new(key, val))
                } else {
                    Err(Error::Client(ClientError::CannotParseEnum))
                }
            }
            RespView::Error(e) => Err(Error::Redis(RedisError::try_from(e)?)),
            _ => Err(Error::Client(ClientError::CannotParseEnum)),
        }
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct NilSeqAccess;

impl<'de> de::SeqAccess<'de> for NilSeqAccess {
    type Error = Error;

    #[inline]
    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}

impl<'de> de::MapAccess<'de> for NilSeqAccess {
    type Error = Error;

    #[inline]
    fn next_key_seed<K>(&mut self, _seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn next_value_seed<V>(&mut self, _seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        Err(Error::Client(ClientError::Unexpected))
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}

struct IntegerArraySeqAccess<'de> {
    iter: std::slice::Iter<'de, i64>,
}

impl<'de> IntegerArraySeqAccess<'de> {
    #[inline(always)]
    fn new(iter: std::slice::Iter<'de, i64>) -> Self {
        Self { iter }
    }
}

impl<'de> de::SeqAccess<'de> for IntegerArraySeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(i) => seed
                .deserialize(RespDeserializer::new(RespView::Integer(*i)))
                .map(Some),
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

impl<'de> de::MapAccess<'de> for IntegerArraySeqAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(i) => seed
                .deserialize(RespDeserializer::new(RespView::Integer(*i)))
                .map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(i) => seed.deserialize(RespDeserializer::new(RespView::Integer(*i))),
            None => Err(Error::Client(ClientError::CannotParseMap)),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() / 2)
    }
}

struct OwnedArraySeqAccess<'de> {
    iter: std::slice::Iter<'de, RespResponse>,
}

impl<'de> OwnedArraySeqAccess<'de> {
    #[inline(always)]
    fn new(iter: std::slice::Iter<'de, RespResponse>) -> Self {
        Self { iter }
    }
}

impl<'de> de::SeqAccess<'de> for OwnedArraySeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(resp) => {
                let view = resp.view();
                let deserializer = RespDeserializer::new(view);
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

impl<'de> de::MapAccess<'de> for OwnedArraySeqAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next().map(|r| r.view()) {
            Some(view) => {
                if let RespView::Array(array_view) = &view
                    && array_view.len() == 2
                {
                    let mut inner_iter = array_view.clone().into_iter();
                    let key_view = inner_iter
                        .next()
                        .ok_or_else(|| Error::Client(ClientError::CannotParseMap))?;
                    return seed.deserialize(RespDeserializer::new(key_view)).map(Some);
                }

                seed.deserialize(RespDeserializer::new(view)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(r) => seed.deserialize(RespDeserializer::new(r.view())),
            None => Err(Error::Client(ClientError::CannotParseMap)),
        }
    }

    fn next_entry_seed<K, V>(&mut self, kseed: K, vseed: V) -> Result<Option<(K::Value, V::Value)>>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        match self.iter.next().map(|r| r.view()) {
            Some(view) => {
                if let RespView::Array(ref array_view) = view
                    && array_view.len() == 2
                {
                    let mut pair_iter = array_view.clone().into_iter();
                    let kview = pair_iter.next().unwrap();
                    let vview = pair_iter.next().unwrap();

                    let key = kseed.deserialize(RespDeserializer::new(kview))?;
                    let value = vseed.deserialize(RespDeserializer::new(vview))?;
                    return Ok(Some((key, value)));
                }

                let key = kseed.deserialize(RespDeserializer::new(view))?;
                let vview = self
                    .iter
                    .next()
                    .map(|r| r.view())
                    .ok_or_else(|| Error::Client(ClientError::CannotParseMap))?;
                let value = vseed.deserialize(RespDeserializer::new(vview))?;

                Ok(Some((key, value)))
            }
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() / 2)
    }
}

struct SeqAccess<'de> {
    iter: RespArrayIter<'de>,
}

impl<'de> SeqAccess<'de> {
    #[inline(always)]
    fn new(iter: RespArrayIter<'de>) -> Self {
        Self { iter }
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => seed.deserialize(RespDeserializer::new(view)).map(Some),
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

impl<'de> de::MapAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => {
                if let RespView::Array(array_view) = &view
                    && array_view.len() == 2
                {
                    let mut inner_iter = array_view.clone().into_iter();
                    let key_view = inner_iter
                        .next()
                        .ok_or_else(|| Error::Client(ClientError::CannotParseMap))?;
                    return seed.deserialize(RespDeserializer::new(key_view)).map(Some);
                }

                seed.deserialize(RespDeserializer::new(view)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => seed.deserialize(RespDeserializer::new(view)),
            None => Err(Error::Client(ClientError::CannotParseMap)),
        }
    }

    fn next_entry_seed<K, V>(&mut self, kseed: K, vseed: V) -> Result<Option<(K::Value, V::Value)>>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => {
                if let RespView::Array(ref array_view) = view
                    && array_view.len() == 2
                {
                    let mut pair_iter = array_view.clone().into_iter();
                    let kview = pair_iter.next().unwrap();
                    let vview = pair_iter.next().unwrap();

                    let key = kseed.deserialize(RespDeserializer::new(kview))?;
                    let value = vseed.deserialize(RespDeserializer::new(vview))?;
                    return Ok(Some((key, value)));
                }

                let key = kseed.deserialize(RespDeserializer::new(view))?;
                let vview = self
                    .iter
                    .next()
                    .ok_or_else(|| Error::Client(ClientError::CannotParseMap))?;
                let value = vseed.deserialize(RespDeserializer::new(vview))?;

                Ok(Some((key, value)))
            }
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() / 2)
    }
}

struct MapAccess<'a> {
    iter: RespArrayIter<'a>,
}

impl<'a> MapAccess<'a> {
    #[inline(always)]
    fn new(iter: RespArrayIter<'a>) -> Self {
        Self { iter }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => seed.deserialize(RespDeserializer::new(view)).map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => seed.deserialize(RespDeserializer::new(view)),
            None => Err(Error::Client(ClientError::Unexpected)),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() / 2)
    }
}

impl<'de> de::SeqAccess<'de> for MapAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.iter.has_next() {
            seed.deserialize(RespTuple2Deserializer {
                iter: &mut self.iter,
            })
            .map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len() / 2)
    }
}

struct RespTuple2Deserializer<'a, 'de> {
    iter: &'a mut RespArrayIter<'de>,
}

impl<'de, 'a> de::Deserializer<'de> for RespTuple2Deserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // A tuple is processed like a n elements sequence
        visitor.visit_seq(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de, 'a> de::SeqAccess<'de> for RespTuple2Deserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(view) => seed.deserialize(RespDeserializer::new(view)).map(Some),
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(2)
    }
}

pub struct EnumAccess<'de> {
    variant_name: RespView<'de>,
    content: RespView<'de>,
}

impl<'de> EnumAccess<'de> {
    #[inline(always)]
    pub fn new(variant_name: RespView<'de>, content: RespView<'de>) -> Self {
        Self {
            variant_name,
            content,
        }
    }
}

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(RespDeserializer::new(self.variant_name))?;
        Ok((val, VariantAccess::new(self.content)))
    }
}

pub struct VariantAccess<'de> {
    content: RespView<'de>,
}

impl<'de> VariantAccess<'de> {
    #[inline(always)]
    pub fn new(content: RespView<'de>) -> Self {
        Self { content }
    }
}

impl<'de> de::VariantAccess<'de> for VariantAccess<'de> {
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
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(RespDeserializer::new(self.content))
    }

    // Tuple variants are represented as map of array so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        RespDeserializer::new(self.content).deserialize_seq(visitor)
    }

    // Struct variants are represented as map of map so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        RespDeserializer::new(self.content).deserialize_map(visitor)
    }
}

struct PushMapAccess<'de> {
    view: RespArrayView<'de>,
    visited: bool,
}

impl<'de> PushMapAccess<'de> {
    #[inline(always)]
    const fn new(view: RespArrayView<'de>) -> Self {
        Self {
            view,
            visited: false,
        }
    }
}

impl<'de> de::MapAccess<'de> for PushMapAccess<'de> {
    type Error = Error;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.visited {
            return Ok(None);
        }

        self.visited = true;
        seed.deserialize(PushFieldDeserializer).map(Some)
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(PushDeserializer::new(self.view.clone()))
    }

    #[inline(always)]
    fn size_hint(&self) -> Option<usize> {
        Some(self.view.len())
    }
}

struct PushFieldDeserializer;

impl<'de> de::Deserializer<'de> for PushFieldDeserializer {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(PUSH_FAKE_FIELD)
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

struct PushDeserializer<'de> {
    view: RespArrayView<'de>,
}

impl<'de> PushDeserializer<'de> {
    #[inline(always)]
    const fn new(view: RespArrayView<'de>) -> Self {
        Self { view }
    }
}

impl<'de> Deserializer<'de> for PushDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess::new(self.view.into_iter()))
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}
