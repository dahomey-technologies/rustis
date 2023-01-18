use crate::{resp::PUSH_FAKE_FIELD, Error, RedisError, Result};
use memchr::memchr;
use serde::{
    de::{DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};
use std::str::{self, FromStr};

const SIMPLE_STRING_TAG: u8 = b'+';
const ERROR_TAG: u8 = b'-';
const INTEGER_TAG: u8 = b':';
const BULK_STRING_TAG: u8 = b'$';
const ARRAY_TAG: u8 = b'*';
const MAP_TAG: u8 = b'%';
const SET_TAG: u8 = b'~';
const DOUBLE_TAG: u8 = b',';
const NIL_TAG: u8 = b'_';
const BOOL_TAG: u8 = b'#';
const VERBATIM_STRING_TAG: u8 = b'=';
const PUSH_TAG: u8 = b'>';
const BLOB_ERROR_TAG: u8 = b'!';

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

    #[inline]
    pub fn get_pos(&self) -> usize {
        self.pos
    }

    // Look at the first byte in the input without consuming it.
    fn peek(&mut self) -> Result<u8> {
        if self.pos < self.buf.len() {
            let byte = self.buf[self.pos];

            match byte {
                ERROR_TAG => {
                    self.advance();
                    let str = self.parse_string()?;
                    Err(Error::Redis(RedisError::from_str(str)?))
                }
                BLOB_ERROR_TAG => {
                    self.advance();
                    let bs = self.parse_bulk_string()?;
                    let str = str::from_utf8(bs)?;
                    Err(Error::Redis(RedisError::from_str(str)?))
                }
                _ => Ok(byte),
            }
        } else {
            eof()
        }
    }

    #[inline(always)]
    fn next(&mut self) -> Result<u8> {
        self.peek().map(|v| {
            self.advance();
            v
        })
    }

    #[inline(always)]
    fn advance(&mut self) {
        self.pos += 1;
    }

    fn next_line(&mut self) -> Result<&'de [u8]> {
        match memchr(b'\r', &self.buf[self.pos..]) {
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

    fn peek_line(&self) -> Result<&'de [u8]> {
        match memchr(b'\r', &self.buf[self.pos..]) {
            Some(idx)
                if self.buf.len() > self.pos + idx + 1 && self.buf[self.pos + idx + 1] == b'\n' =>
            {
                let slice = &self.buf[self.pos..self.pos + idx];
                Ok(slice)
            }
            _ => eof(),
        }
    }

    fn parse_number<T>(&mut self) -> Result<T>
    where
        T: FromStr,
    {
        let next_line = self.next_line()?;
        let str = str::from_utf8(next_line)?;
        str.parse::<T>()
            .map_err(|_| Error::Client("Cannot parse number".to_owned()))
    }

    fn peek_number<T>(&self) -> Result<T>
    where
        T: FromStr,
    {
        let next_line = self.peek_line()?;
        let str = str::from_utf8(&next_line[1..])?;
        str.parse::<T>()
            .map_err(|_| Error::Client(format!("Cannot parse number from {str}")))
    }

    fn parse_bulk_string(&mut self) -> Result<&'de [u8]> {
        let len = self.parse_number::<usize>()?;
        if self.buf.len() - self.pos < len + 2 {
            eof()
        } else if self.buf[self.pos + len] != b'\r' || self.buf[self.pos + len + 1] != b'\n' {
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

    #[inline(always)]
    fn parse_string(&mut self) -> Result<&'de str> {
        let next_line = self.next_line()?;
        let str = str::from_utf8(next_line)?;
        Ok(str)
    }

    #[inline(always)]
    fn peek_string(&self) -> Result<Option<&'de str>> {
        let next_line = self.peek_line()?;
        if let Some(&SIMPLE_STRING_TAG) = next_line.first() {
            let str = str::from_utf8(&next_line[1..])?;
            Ok(Some(str))
        } else {
            Ok(None)
        }
    }

    fn parse_nil(&mut self) -> Result<()> {
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
        match self.next()? {
            INTEGER_TAG => self.parse_number::<T>(),
            DOUBLE_TAG => self.parse_number::<T>(),
            NIL_TAG => {
                self.parse_nil()?;
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
            ARRAY_TAG => {
                let len = self.parse_number::<usize>()?;
                if len == 1 && self.next()? == INTEGER_TAG {
                    self.parse_number::<T>()
                } else {
                    Err(Error::Client("Cannot parse number".to_owned()))
                }
            }
            _ => Err(Error::Client("Cannot parse number".to_owned())),
        }
    }

    fn parse_float_ex<T>(&mut self) -> Result<T>
    where
        T: FromStr + Default,
    {
        match self.next()? {
            INTEGER_TAG | DOUBLE_TAG => self.parse_number::<T>(),
            NIL_TAG => {
                self.parse_nil()?;
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

    fn ignore_value(&mut self) -> Result<()> {
        match self.next()? {
            SIMPLE_STRING_TAG | ERROR_TAG | INTEGER_TAG | DOUBLE_TAG | NIL_TAG | BOOL_TAG => {
                self.next_line()?;
                Ok(())
            }
            BULK_STRING_TAG | BLOB_ERROR_TAG | VERBATIM_STRING_TAG => {
                self.parse_bulk_string()?;
                Ok(())
            }
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_number::<usize>()?;
                for _ in 0..len {
                    self.ignore_value()?;
                }
                Ok(())
            }
            MAP_TAG => {
                let len = self.parse_number::<usize>()? * 2;
                for _ in 0..len {
                    self.ignore_value()?;
                }
                Ok(())
            }
            _ => Err(Error::Client("Cannot parse tag".to_owned())),
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
            BULK_STRING_TAG => self.deserialize_bytes(visitor),
            ARRAY_TAG => self.deserialize_seq(visitor),
            MAP_TAG => self.deserialize_map(visitor),
            SET_TAG => self.deserialize_seq(visitor),
            INTEGER_TAG => self.deserialize_i64(visitor),
            DOUBLE_TAG => self.deserialize_f64(visitor),
            SIMPLE_STRING_TAG => self.deserialize_str(visitor),
            NIL_TAG => self.deserialize_option(visitor),
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
        let result: bool = match self.next()? {
            INTEGER_TAG => self.parse_number::<i64>()? != 0,
            DOUBLE_TAG => self.parse_number::<f64>()? != 0.,
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
            NIL_TAG => false,
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
        let result: char = match self.next()? {
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
            NIL_TAG => '\0',
            _ => return Err(Error::Client("Cannot parse to char".to_owned())),
        };

        visitor.visit_char(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                str::from_utf8(bs)?
            }
            SIMPLE_STRING_TAG => self.parse_string()?,
            NIL_TAG => "",
            _ => return Err(Error::Client("Cannot parse to str".to_owned())),
        };

        visitor.visit_borrowed_str(result)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            DOUBLE_TAG => self.parse_number::<f64>()?.to_string(),
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                str::from_utf8(bs)?.to_owned()
            }
            NIL_TAG => String::from(""),
            SIMPLE_STRING_TAG => self.parse_string()?.to_owned(),
            _ => return Err(Error::Client("Cannot parse to String".to_owned())),
        };

        visitor.visit_string(result)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            BULK_STRING_TAG => self.parse_bulk_string()?,
            NIL_TAG => &[],
            SIMPLE_STRING_TAG => self.parse_string()?.as_bytes(),
            _ => return Err(Error::Client("Cannot parse to bytes".to_owned())),
        };

        visitor.visit_borrowed_bytes(result)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            BULK_STRING_TAG => self.parse_bulk_string()?.to_vec(),
            NIL_TAG => vec![],
            SIMPLE_STRING_TAG => self.parse_string()?.as_bytes().to_vec(),
            _ => return Err(Error::Client("Cannot parse to byte buffer".to_owned())),
        };

        visitor.visit_byte_buf(result)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            NIL_TAG => {
                self.advance();
                self.parse_nil()?;
                visitor.visit_none()
            }
            ARRAY_TAG => {
                let len = self.peek_number::<usize>()?;
                if len == 0 {
                    visitor.visit_none()
                } else {
                    visitor.visit_some(self)
                }
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.next()? {
            NIL_TAG => {
                self.parse_nil()?;
                visitor.visit_unit()
            }
            INTEGER_TAG => {
                self.parse_number::<i64>()?;
                visitor.visit_unit()
            }
            SIMPLE_STRING_TAG => {
                self.parse_string()?;
                visitor.visit_unit()
            }
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                if bs.len() == 0 {
                    visitor.visit_unit()
                } else {
                    return Err(Error::Client("Expected nil".to_owned()));
                }
            }
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_number::<usize>()?;
                if len == 0 {
                    visitor.visit_unit()
                } else {
                    return Err(Error::Client("Expected nil".to_owned()));
                }
            }
            _ => Err(Error::Client("Expected nil".to_owned())),
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
        match self.next()? {
            NIL_TAG => visitor.visit_seq(NilSeqAccess),
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_number()?;
                visitor.visit_seq(SeqAccess { de: self, len })
            }
            MAP_TAG => {
                let len = self.parse_number()?;
                visitor.visit_seq(MapAccess { de: self, len })
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
        match self.next()? {
            ARRAY_TAG => {
                let len: usize = self.parse_number()?;
                if len % 2 != 0 {
                    return Err(Error::Client(
                        "Array len must be even to be able to parse a map".to_owned(),
                    ));
                }
                visitor.visit_map(SeqAccess { de: self, len })
            }
            MAP_TAG => {
                let len = self.parse_number()?;
                visitor.visit_map(MapAccess { de: self, len })
            }
            _ => Err(Error::Client("Cannot parse map".to_owned())),
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
        fn check_resp2_array(
            de: &mut RespDeserializer,
            array_len: usize,
            fields: &'static [&'static str],
        ) -> Result<bool> {
            Ok(if array_len > fields.len() {
                true
            } else if let Some(s) = de.peek_string()? {
                fields.iter().any(|f| s == *f)
            } else {
                false
            })
        }

        match self.next()? {
            ARRAY_TAG => {
                let len: usize = self.parse_number()?;
                if check_resp2_array(self, len, fields)? {
                    visitor.visit_map(SeqAccess { de: self, len })
                } else {
                    visitor.visit_seq(SeqAccess { de: self, len })
                }
            }
            MAP_TAG => {
                let len = self.parse_number()?;
                visitor.visit_map(MapAccess { de: self, len })
            }
            _ => Err(Error::Client("Cannot parse struct".to_owned())),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.next()? {
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
                let len: usize = self.parse_number()?;
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
                let len: usize = self.parse_number()?;
                if len == 1 {
                    visitor.visit_enum(Enum { de: self })
                } else {
                    Err(Error::Client(
                        "Map len must be 1 to parse an enum".to_owned(),
                    ))
                }
            }
            _ => Err(Error::Client(format!("Cannot parse enum `{name}`"))),
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
        self.ignore_value()?;
        visitor.visit_unit()
    }
}

pub struct NilSeqAccess;

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

struct SeqAccess<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
    len: usize,
}

impl<'de, 'a> serde::de::SeqAccess<'de> for SeqAccess<'a, 'de> {
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

impl<'de, 'a> serde::de::MapAccess<'de> for SeqAccess<'a, 'de> {
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
        self.len -= 1;
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct MapAccess<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
    len: usize,
}

impl<'de, 'a> serde::de::MapAccess<'de> for MapAccess<'a, 'de> {
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

impl<'de, 'a> serde::de::SeqAccess<'de> for MapAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            self.len -= 1;
            seed.deserialize(PairDeserializer { de: self.de }).map(Some)
        } else {
            Ok(None)
        }
    }
}

struct PairDeserializer<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
}

impl<'de, 'a> Deserializer<'de> for PairDeserializer<'a, 'de> {
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
        len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        pub struct PairSeqAccess<'a, 'de: 'a> {
            de: &'a mut RespDeserializer<'de>,
            len: usize,
        }

        impl<'de, 'a> serde::de::SeqAccess<'de> for PairSeqAccess<'a, 'de> {
            type Error = Error;

            fn next_element_seed<T>(
                &mut self,
                seed: T,
            ) -> std::result::Result<Option<T::Value>, Self::Error>
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
        }

        visitor.visit_seq(PairSeqAccess { de: self.de, len })
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

impl<'de, 'a> serde::de::MapAccess<'de> for PushMapAccess<'de, 'a> {
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
