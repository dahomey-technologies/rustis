use crate::{
    resp::{RawValue, PUSH_FAKE_FIELD},
    Error, RedisError, Result,
};
use serde::{
    de::{
        DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
    },
    Deserializer,
};
use std::{
    ops::Range,
    str::{self, FromStr},
};

#[inline(always)]
fn eof<T>() -> Result<T> {
    Err(Error::Client("EOF".to_owned()))
}

/// Serde deserialize for [`RESP2`](https://redis.io/docs/reference/protocol-spec/) &
/// [`RESP3`](https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md)
pub struct RespDeserializer2<'de> {
    buf: &'de [u8],
    raw_values: Vec<RawValue>,
    pos: usize,
}

impl<'de> RespDeserializer2<'de> {
    pub fn new(buf: &'de [u8], raw_values: Vec<RawValue>) -> Self {
        RespDeserializer2 {
            buf,
            raw_values,
            pos: 0,
        }
    }

    fn peek(&self) -> Result<RawValue> {
        if self.pos < self.raw_values.len() {
            let value = self.raw_values[self.pos].clone();
            if let RawValue::Error(range) = value {
                let slice = &self.buf[range];
                let str = str::from_utf8(slice)?;
                Err(Error::Redis(RedisError::from_str(str)?))
            } else {
                Ok(value)
            }
        } else {
            eof()
        }
    }

    #[inline(always)]
    fn next(&mut self) -> Result<RawValue> {
        match self.peek() {
            Ok(v) => {
                self.advance();
                Ok(v)
            }
            Err(e) => Err(e),
        }
    }

    #[inline(always)]
    fn advance(&mut self) {
        self.pos += 1;
    }

    #[inline(always)]
    fn parse_integer<T>(&mut self, range: Range<usize>) -> Result<T>
    where
        T: FromStr,
    {
        let str = str::from_utf8(&self.buf[range])?;
        str.parse::<T>()
            .map_err(|_| Error::Client("Cannot parse integer".to_owned()))
    }

    #[inline(always)]
    fn parse_float<T>(&mut self, range: Range<usize>) -> Result<T>
    where
        T: FromStr,
    {
        let str = str::from_utf8(&self.buf[range])?;
        str.parse::<T>()
            .map_err(|_| Error::Client("Cannot parse float".to_owned()))
    }

    #[inline(always)]
    fn parse_bulk_string(&mut self, range: Range<usize>) -> &'de [u8] {
        &self.buf[range]
    }

    #[inline(always)]
    fn parse_string(&mut self, range: Range<usize>) -> Result<&'de str> {
        let str = str::from_utf8(&self.buf[range])?;
        Ok(str)
    }

    fn parse_boolean(&mut self, range: Range<usize>) -> Result<bool> {
        let slice = &self.buf[range];
        match slice {
            b"t" => Ok(true),
            b"f" => Ok(false),
            _ => Err(Error::Client(format!(
                "Expected boolean. Got '{}'",
                String::from_utf8_lossy(slice)
            ))),
        }
    }

    fn parse_integer_ex<T>(&mut self) -> Result<T>
    where
        T: FromStr + Default,
    {
        match self.next()? {
            RawValue::Integer(range) => self.parse_integer::<T>(range),
            RawValue::Nil => Ok(Default::default()),
            RawValue::BulkString(range) => {
                let bs = self.parse_bulk_string(range);
                let str = str::from_utf8(bs)?;
                if str.is_empty() {
                    Ok(Default::default())
                } else {
                    str.parse::<T>()
                        .map_err(|_| Error::Client("Cannot parse number".to_owned()))
                }
            }
            RawValue::SimpleString(range) => self
                .parse_string(range)?
                .parse::<T>()
                .map_err(|_| Error::Client("Cannot number".to_owned())),
            _ => Err(Error::Client("Cannot parse integer".to_owned())),
        }
    }

    fn parse_float_ex<T>(&mut self) -> Result<T>
    where
        T: FromStr + Default,
    {
        match self.next()? {
            RawValue::Integer(range) => self.parse_integer::<T>(range),
            RawValue::Double(range) => self.parse_float::<T>(range),
            RawValue::Nil => Ok(Default::default()),
            RawValue::BulkString(range) => {
                let bs = self.parse_bulk_string(range);
                let str = str::from_utf8(bs)?;
                if str.is_empty() {
                    Ok(Default::default())
                } else {
                    str.parse::<T>()
                        .map_err(|_| Error::Client("Cannot parse number".to_owned()))
                }
            }
            RawValue::SimpleString(range) => self
                .parse_string(range)?
                .parse::<T>()
                .map_err(|_| Error::Client("Cannot number".to_owned())),
            _ => Err(Error::Client("Cannot parse number".to_owned())),
        }
    }
}

impl<'de, 'a> Deserializer<'de> for &'a mut RespDeserializer2<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            RawValue::SimpleString(_) => self.deserialize_str(visitor),
            RawValue::Integer(_) => self.deserialize_i64(visitor),
            RawValue::BulkString(_) => self.deserialize_byte_buf(visitor),
            RawValue::Array(_) => self.deserialize_seq(visitor),
            RawValue::Map(_) => self.deserialize_map(visitor),
            RawValue::Set(_) => self.deserialize_seq(visitor),
            RawValue::Double(_) => self.deserialize_f64(visitor),
            RawValue::Nil => self.deserialize_unit(visitor),
            RawValue::Bool(_) => self.deserialize_bool(visitor),
            RawValue::VerbatimString(_) => self.deserialize_byte_buf(visitor),
            RawValue::Push(_) => visitor.visit_map(PushMapAccess::new(self)),
            _ => unreachable!(),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result: bool = match self.next()? {
            RawValue::Integer(range) => self.parse_integer::<i64>(range)? != 0,
            RawValue::BulkString(range) => {
                let bs = self.parse_bulk_string(range);
                match bs {
                    b"1" | b"true" => true,
                    b"0" | b"false" => false,
                    _ => return Err(Error::Client("Cannot parse to bool".to_owned())),
                }
            }
            RawValue::SimpleString(range) => self.parse_string(range)? == "OK",
            RawValue::Bool(range) => self.parse_boolean(range)?,
            _ => return Err(Error::Client("Cannot parse to bool".to_owned())),
        };

        visitor.visit_bool(result)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_integer_ex()?)
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_float_ex()?)
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_float_ex()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result: char = match self.next()? {
            RawValue::BulkString(range) => {
                let bs = self.parse_bulk_string(range);
                let str = str::from_utf8(bs)?;
                if str.len() == 1 {
                    str.chars().next().unwrap()
                } else {
                    return Err(Error::Client("Cannot parse to char".to_owned()));
                }
            }
            RawValue::SimpleString(range) => {
                let str = self.parse_string(range)?;
                if str.len() == 1 {
                    str.chars().next().unwrap()
                } else {
                    return Err(Error::Client("Cannot parse to char".to_owned()));
                }
            }
            _ => return Err(Error::Client("Cannot parse to char".to_owned())),
        };

        visitor.visit_char(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            RawValue::BulkString(range) => {
                let bs = self.parse_bulk_string(range);
                str::from_utf8(bs)?
            }
            RawValue::SimpleString(range) => self.parse_string(range)?,
            RawValue::Nil => "",
            _ => return Err(Error::Client("Cannot parse to str".to_owned())),
        };

        visitor.visit_borrowed_str(result)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            RawValue::BulkString(range) => {
                let bs = self.parse_bulk_string(range);
                str::from_utf8(bs)?
            }
            RawValue::SimpleString(range) => self.parse_string(range)?,
            RawValue::Nil => "",
            _ => return Err(Error::Client("Cannot parse to String".to_owned())),
        };

        visitor.visit_string(result.to_owned())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let RawValue::BulkString(range) = self.next()? {
            visitor.visit_bytes(self.parse_bulk_string(range))
        } else {
            Err(Error::Client("Cannot parse to bytes".to_owned()))
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let RawValue::BulkString(range) = self.next()? {
            let value = self.parse_bulk_string(range);

            if value.is_empty() {
                visitor.visit_none()
            } else {
                visitor.visit_byte_buf(value.to_vec())
            }
        } else {
            Err(Error::Client("Cannot parse to byte buffer".to_owned()))
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            RawValue::BulkString(_) => visitor.visit_some(self),
            RawValue::Nil => {
                self.advance();
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let RawValue::Nil = self.next()? {
            visitor.visit_unit()
        } else {
            Err(Error::Client("Expected null".to_owned()))
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
        if let RawValue::Array(len) | RawValue::Set(len) | RawValue::Push(len) = self.next()? {
            visitor.visit_seq(SeqMapAccess { de: self, len })
        } else {
            Err(Error::Client("Cannot parse to Sequence".to_owned()))
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
        let len = match self.next()? {
            RawValue::Array(len) => {
                if len % 2 == 0 {
                    len / 2
                } else {
                    return Err(Error::Client(
                        "Array len must be even to be able to parse a map".to_owned(),
                    ));
                }
            }
            RawValue::Map(len) => len,
            _ => return Err(Error::Client("Cannot parse map".to_owned())),
        };

        visitor.visit_map(SeqMapAccess { de: self, len })
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
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
        match self.next()? {
            RawValue::BulkString(range) => {
                // Visit a unit variant.
                let bs = self.parse_bulk_string(range);
                let str = str::from_utf8(bs)?;
                visitor.visit_enum(str.into_deserializer())
            }
            RawValue::SimpleString(range) => {
                // Visit a unit variant.
                let str = self.parse_string(range)?;
                visitor.visit_enum(str.into_deserializer())
            }
            RawValue::Array(len) => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as an array of 2 elements
                if len == 2 {
                    visitor.visit_enum(Enum { de: self })
                } else {
                    Err(Error::Client(
                        "Array len must be 2 to parse an enum".to_owned(),
                    ))
                }
            }
            RawValue::Map(len) => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as a map of 1 element
                if len == 1 {
                    visitor.visit_enum(Enum { de: self })
                } else {
                    Err(Error::Client(
                        "Map len must be 1 to parse an enum".to_owned(),
                    ))
                }
            }
            _ => Err(Error::Client("Cannot parse enum".to_owned())),
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
        self.deserialize_any(visitor)
    }
}

pub struct SeqMapAccess<'a, 'de: 'a> {
    de: &'a mut RespDeserializer2<'de>,
    len: usize,
}

impl<'de, 'a> SeqAccess<'de> for SeqMapAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de, 'a> MapAccess<'de> for SeqMapAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut RespDeserializer2<'de>,
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    #[inline]
    fn unit_variant(self) -> Result<()> {
        Err(Error::Client("Expected string or bulk string".to_owned()))
    }

    // Newtype variants are represented as map so
    // deserialize the value here.
    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented as map of array so
    // deserialize the sequence of data here.
    #[inline]
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_seq(visitor)
    }

    // Struct variants are represented as map of map so
    // deserialize the inner map here.
    #[inline]
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_map(visitor)
    }
}

struct PushMapAccess<'de, 'a> {
    de: &'a mut RespDeserializer2<'de>,
    visited: bool,
}

impl<'de, 'a> PushMapAccess<'de, 'a> {
    #[inline]
    fn new(de: &'a mut RespDeserializer2<'de>) -> Self {
        Self { de, visited: false }
    }
}

impl<'de, 'a> MapAccess<'de> for PushMapAccess<'de, 'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
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
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(PushDeserializer { de: self.de })
    }
}

struct PushFieldDeserializer;

impl<'de> Deserializer<'de> for PushFieldDeserializer {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(PUSH_FAKE_FIELD)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

struct PushDeserializer<'de, 'a> {
    de: &'a mut RespDeserializer2<'de>,
}

impl<'de, 'a> Deserializer<'de> for PushDeserializer<'de, 'a> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_seq(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}
