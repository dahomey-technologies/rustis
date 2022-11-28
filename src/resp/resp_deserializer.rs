use crate::{Error, RedisError, Result};
use serde::{
    de::{
        DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
    },
    Deserializer,
};
use std::str::{self, FromStr};

pub(crate) const PUSH_FAKE_FIELD: &str = ">>>PUSH>>>";

const SIMPLE_STRING_TAG: u8 = b'+';
const ERROR_TAG: u8 = b'-';
const INTEGER_TAG: u8 = b':';
const BULK_STRING_TAG: u8 = b'$';
const ARRAY_TAG: u8 = b'*';
const MAP_TAG: u8 = b'%';
const SET_TAG: u8 = b'~';
const DOUBLE_TAG: u8 = b',';
const NULL_TAG: u8 = b'_';
const BOOL_TAG: u8 = b'#';
const VERBATIM_STRING_TAG: u8 = b'=';
const PUSH_TAG: u8 = b'>';

#[inline(always)]
fn eof<T>() -> Result<T> {
    Err(Error::Client("EOF".to_owned()))
}

/// Serde deserialize for [`RESP2`](https://redis.io/docs/reference/protocol-spec/) &
/// [`RESP3`](https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md)
pub struct RespDeserializer<'de> {
    buf: &'de [u8],
    pos: usize,
}

impl<'de> RespDeserializer<'de> {
    pub fn from_bytes(buf: &'de [u8]) -> Self {
        RespDeserializer { buf, pos: 0 }
    }

    // Look at the first byte in the input without consuming it.
    fn peek(&mut self) -> Result<u8> {
        if self.pos < self.buf.len() {
            let byte = self.buf[self.pos];
            if byte == ERROR_TAG {
                self.advance()?;
                let next_line = self.next_line()?;
                let str = str::from_utf8(next_line)?;
                Err(Error::Redis(RedisError::from_str(str)?))
            } else {
                Ok(self.buf[self.pos])
            }
        } else {
            eof()
        }
    }

    fn advance(&mut self) -> Result<()> {
        if self.pos < self.buf.len() {
            self.pos += 1;
            Ok(())
        } else {
            eof()
        }
    }

    fn expect_byte(&mut self, byte: u8) -> Result<()> {
        if self.peek()? != byte {
            Err(Error::Client(format!(
                "Expected byte '{}'(0x{:02x})",
                byte as char, byte
            )))
        } else {
            self.advance()
        }
    }

    fn next_line(&mut self) -> Result<&'de [u8]> {
        match self.buf[self.pos..].iter().position(|b| *b == b'\r') {
            Some(idx)
                if self.buf.len() > self.pos + idx + 1 && self.buf[self.pos + idx + 1] == b'\n' =>
            {
                let slice = &self.buf[self.pos..self.pos + idx];
                self.pos += idx + 2;
                Ok(slice)
            }
            _ => eof(),
        }
    }

    #[inline(always)]
    fn parse_integer<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        self.expect_byte(INTEGER_TAG)?;
        self.parse_raw_number()
    }

    #[inline(always)]
    fn parse_float<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        self.expect_byte(DOUBLE_TAG)?;
        self.parse_raw_number()
    }

    fn parse_raw_number<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        let next_line = self.next_line()?;
        let str = str::from_utf8(next_line)?;
        str.parse::<T>()
            .map_err(|_| Error::Client("Cannot parse number".to_owned()))
    }

    fn parse_bulk_string(&mut self) -> Result<&'de [u8]> {
        self.expect_byte(BULK_STRING_TAG)?;

        match self.parse_raw_number::<i64>()? {
            -1 => Ok(&[]),
            len => {
                let len = usize::try_from(len)
                    .map_err(|_| Error::Client("Malformed bulk string len".to_owned()))?;
                if self.buf.len() - self.pos < len + 2 {
                    eof()
                } else if self.buf[self.pos + len] != b'\r' || self.buf[self.pos + len + 1] != b'\n'
                {
                    Err(Error::Client(format!(
                        "Expected \\r\\n after bulk string. Got '{}''{}'",
                        self.buf[self.pos + len] as char,
                        self.buf[self.pos + len + 1] as char
                    )))
                } else {
                    let result = &self.buf[self.pos..self.pos + len];
                    self.pos += len + 2;
                    Ok(result)
                }
            }
        }
    }

    fn try_parse_null_bulkstring(&mut self) -> bool {
        if self.buf.len() >= self.pos + 5 && &self.buf[self.pos..self.pos + 5] == b"$-1\r\n" {
            self.pos += 5;
            true
        } else {
            false
        }
    }

    fn parse_string(&mut self) -> Result<&'de str> {
        self.expect_byte(SIMPLE_STRING_TAG)?;
        let next_line = self.next_line()?;
        let str = str::from_utf8(next_line)?;
        Ok(str)
    }

    fn parse_null(&mut self) -> Result<()> {
        self.expect_byte(NULL_TAG)?;
        let next_line = self.next_line()?;
        if next_line.is_empty() {
            Ok(())
        } else {
            Err(Error::Client(format!(
                "Expected \\r\\n after null. Got '{}'",
                String::from_utf8_lossy(next_line)
            )))
        }
    }

    fn parse_boolean(&mut self) -> Result<bool> {
        self.expect_byte(BOOL_TAG)?;
        let next_line = self.next_line()?;

        match next_line {
            b"t" => Ok(true),
            b"f" => Ok(false),
            _ => Err(Error::Client(format!(
                "Expected boolean. Got '{}'",
                String::from_utf8_lossy(next_line)
            ))),
        }
    }

    fn parse_integer_ex<T>(&mut self) -> Result<T>
    where
        T: FromStr + Default,
    {
        let first_byte = self.peek()?;

        match first_byte {
            INTEGER_TAG => self.parse_integer::<T>(),
            NULL_TAG => {
                self.parse_null()?;
                Ok(Default::default())
            }
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                let str = str::from_utf8(bs)?;
                if str.is_empty() {
                    Ok(Default::default())
                } else {
                    str.parse::<T>()
                        .map_err(|_| Error::Client("Cannot parse number".to_owned()))
                }
            }
            SIMPLE_STRING_TAG => self
                .parse_string()?
                .parse::<T>()
                .map_err(|_| Error::Client("Cannot number".to_owned())),
            _ => Err(Error::Client("Cannot parse number".to_owned())),
        }
    }

    fn parse_float_ex<T>(&mut self) -> Result<T>
    where
        T: FromStr + Default,
    {
        let first_byte = self.peek()?;

        match first_byte {
            INTEGER_TAG => self.parse_integer::<T>(),
            DOUBLE_TAG => self.parse_float::<T>(),
            NULL_TAG => {
                self.parse_null()?;
                Ok(Default::default())
            }
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                let str = str::from_utf8(bs)?;
                if str.is_empty() {
                    Ok(Default::default())
                } else {
                    str.parse::<T>()
                        .map_err(|_| Error::Client("Cannot parse number".to_owned()))
                }
            }
            SIMPLE_STRING_TAG => self
                .parse_string()?
                .parse::<T>()
                .map_err(|_| Error::Client("Cannot number".to_owned())),
            _ => Err(Error::Client("Cannot parse number".to_owned())),
        }
    }
}

impl<'de, 'a> Deserializer<'de> for &'a mut RespDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let first_byte = self.peek()?;

        match first_byte {
            BULK_STRING_TAG => self.deserialize_byte_buf(visitor),
            ARRAY_TAG => self.deserialize_seq(visitor),
            MAP_TAG => self.deserialize_map(visitor),
            SET_TAG => self.deserialize_seq(visitor),
            INTEGER_TAG => self.deserialize_i64(visitor),
            DOUBLE_TAG => self.deserialize_f64(visitor),
            SIMPLE_STRING_TAG => self.deserialize_str(visitor),
            NULL_TAG => self.deserialize_unit(visitor),
            BOOL_TAG => self.deserialize_bool(visitor),
            VERBATIM_STRING_TAG => self.deserialize_byte_buf(visitor),
            PUSH_TAG => visitor.visit_map(PushMapAccess::new(self)),
            _ => Err(Error::Client(format!(
                "Unknown data type '{}' (0x{:02x})",
                first_byte as char, first_byte
            ))),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let first_byte = self.peek()?;

        let result: bool = match first_byte {
            INTEGER_TAG => self.parse_integer::<i64>()? != 0,
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                match bs {
                    b"1" | b"true" => true,
                    b"0" | b"false" => false,
                    _ => return Err(Error::Client("Cannot parse to bool".to_owned())),
                }
            }
            SIMPLE_STRING_TAG => self.parse_string()? == "OK",
            BOOL_TAG => self.parse_boolean()?,
            _ => return Err(Error::Client("Cannot parse to bool".to_owned())),
        };

        visitor.visit_bool(result)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_integer_ex()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_integer_ex()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_integer_ex()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_integer_ex()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_integer_ex()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_integer_ex()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_integer_ex()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_integer_ex()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_float_ex()?)
    }

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
        let first_byte = self.peek()?;

        let result: char = match first_byte {
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                let str = str::from_utf8(bs)?;
                if str.len() == 1 {
                    str.chars().next().unwrap()
                } else {
                    return Err(Error::Client("Cannot parse to char".to_owned()));
                }
            }
            SIMPLE_STRING_TAG => {
                let str = self.parse_string()?;
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
        let first_byte = self.peek()?;

        let result = match first_byte {
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                str::from_utf8(bs)?
            }
            SIMPLE_STRING_TAG => self.parse_string()?,
            _ => return Err(Error::Client("Cannot parse to str".to_owned())),
        };

        visitor.visit_borrowed_str(result)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let first_byte = self.peek()?;

        let result = match first_byte {
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                str::from_utf8(bs)?
            }
            SIMPLE_STRING_TAG => self.parse_string()?,
            _ => return Err(Error::Client("Cannot parse to String".to_owned())),
        };

        visitor.visit_string(result.to_owned())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.parse_bulk_string()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.parse_bulk_string()?;
        if value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_byte_buf(value.to_vec())
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let first_byte = self.peek()?;

        match first_byte {
            BULK_STRING_TAG => {
                if self.try_parse_null_bulkstring() {
                    visitor.visit_none()
                } else {
                    visitor.visit_some(self)
                }
            }
            NULL_TAG => {
                self.parse_null()?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let first_byte = self.peek()?;

        match first_byte {
            BULK_STRING_TAG => {
                if self.try_parse_null_bulkstring() {
                    visitor.visit_unit()
                } else {
                    Err(Error::Client("Expected null".to_owned()))
                }
            }
            NULL_TAG => {
                self.parse_null()?;
                visitor.visit_unit()
            }
            _ => Err(Error::Client("Expected null".to_owned())),
        }
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
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
        let first_byte = self.peek()?;

        match first_byte {
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                self.advance()?;
                let array_len = self.parse_raw_number()?;

                visitor.visit_seq(SeqMapAccess {
                    de: self,
                    len: array_len,
                })
            }
            _ => Err(Error::Client("Cannot parse to Sequence".to_owned())),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

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
        let first_byte = self.peek()?;

        let len = match first_byte {
            ARRAY_TAG => {
                self.advance()?;
                let len: usize = self.parse_raw_number()?;
                if len % 2 == 0 {
                    len / 2
                } else {
                    return Err(Error::Client(
                        "Array len must be even to be able to parse a map".to_owned(),
                    ));
                }
            }
            MAP_TAG => {
                self.advance()?;
                self.parse_raw_number()?
            }
            _ => return Err(Error::Client("Cannot parse map".to_owned())),
        };

        visitor.visit_map(SeqMapAccess { de: self, len })
    }

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
        let first_byte = self.peek()?;

        match first_byte {
            BULK_STRING_TAG => {
                // Visit a unit variant.
                let bs = self.parse_bulk_string()?;
                let str = str::from_utf8(bs)?;
                visitor.visit_enum(str.into_deserializer())
            }
            SIMPLE_STRING_TAG => {
                // Visit a unit variant.
                let str = self.parse_string()?;
                visitor.visit_enum(str.into_deserializer())
            }
            ARRAY_TAG => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as an array of 2 elements
                self.advance()?;
                let len: usize = self.parse_raw_number()?;
                if len == 2 {
                    visitor.visit_enum(Enum { de: self })
                } else {
                    Err(Error::Client(
                        "Array len must be 2 to parse an enum".to_owned(),
                    ))
                }
            }
            MAP_TAG => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as a map of 1 element
                self.advance()?;
                let len: usize = self.parse_raw_number()?;
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

pub struct SeqMapAccess<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
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

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

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
    fn unit_variant(self) -> Result<()> {
        Err(Error::Client("Expected string or bulk string".to_owned()))
    }

    // Newtype variants are represented as map so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented as map of array so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_seq(visitor)
    }

    // Struct variants are represented as map of map so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_map(visitor)
    }
}

struct PushMapAccess<'de, 'a> {
    de: &'a mut RespDeserializer<'de>,
    visited: bool,
}

impl<'de, 'a> PushMapAccess<'de, 'a> {
    fn new(de: &'a mut RespDeserializer<'de>) -> Self {
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
    de: &'a mut RespDeserializer<'de>,
}

impl<'de, 'a> Deserializer<'de> for PushDeserializer<'de, 'a> {
    type Error = Error;

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
