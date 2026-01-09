use crate::{Error, RedisError, Result, resp::PUSH_FAKE_FIELD};
use memchr::memchr;
use serde::{
    Deserializer,
    de::{DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor},
    forward_to_deserialize_any,
};
use std::str::{self, FromStr};

pub(crate) const SIMPLE_STRING_TAG: u8 = b'+';
pub(crate) const ERROR_TAG: u8 = b'-';
pub(crate) const INTEGER_TAG: u8 = b':';
pub(crate) const BULK_STRING_TAG: u8 = b'$';
pub(crate) const ARRAY_TAG: u8 = b'*';
pub(crate) const MAP_TAG: u8 = b'%';
pub(crate) const SET_TAG: u8 = b'~';
pub(crate) const DOUBLE_TAG: u8 = b',';
pub(crate) const NIL_TAG: u8 = b'_';
pub(crate) const BOOL_TAG: u8 = b'#';
pub(crate) const VERBATIM_STRING_TAG: u8 = b'=';
pub(crate) const PUSH_TAG: u8 = b'>';
pub(crate) const BLOB_ERROR_TAG: u8 = b'!';

#[inline(always)]
const fn eof<T>() -> Result<T> {
    Err(Error::EOF)
}

/// Serde deserializer for [`RESP3`](https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md)
pub struct RespDeserializer<'de> {
    buf: &'de [u8],
    pos: usize,
    eat_error: bool,
}

impl<'de> RespDeserializer<'de> {
    /// Creates a new `RespDeserializer`
    #[inline]
    pub const fn new(buf: &'de [u8]) -> Self {
        RespDeserializer {
            buf,
            pos: 0,
            eat_error: true,
        }
    }

    /// Get current position in the input byte buffer
    #[inline]
    pub const fn get_pos(&self) -> usize {
        self.pos
    }

    /// Returns remaining buffer length for bounds checking
    #[inline(always)]
    const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Check if we have at least n bytes remaining
    #[inline(always)]
    const fn has_bytes(&self, n: usize) -> bool {
        self.remaining() >= n
    }

    // Look at the first byte in the input without consuming it.
    #[inline]
    fn peek(&mut self) -> Result<u8> {
        let byte = *self.buf.get(self.pos).ok_or(Error::EOF)?;

        if self.eat_error {
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
            Ok(byte)
        }
    }

    #[inline(always)]
    fn next(&mut self) -> Result<u8> {
        let byte = self.peek()?;
        self.advance();
        Ok(byte)
    }

    #[inline(always)]
    fn advance(&mut self) {
        self.pos += 1;
    }

    #[inline]
    fn next_line(&mut self) -> Result<&'de [u8]> {
        let remaining = &self.buf[self.pos..];
        let idx = memchr(b'\r', remaining).ok_or(Error::EOF)?;

        // Bounds check optimized
        if idx + 1 >= remaining.len() || remaining[idx + 1] != b'\n' {
            return eof();
        }

        let slice = &remaining[..idx];
        self.pos += idx + 2;
        Ok(slice)
    }

    #[inline]
    fn peek_line(&self) -> Result<&'de [u8]> {
        let remaining = &self.buf[self.pos..];
        let idx = memchr(b'\r', remaining).ok_or(Error::EOF)?;

        if idx + 1 >= remaining.len() || remaining[idx + 1] != b'\n' {
            return eof();
        }

        Ok(&remaining[..idx])
    }

    #[inline]
    fn parse_float<T>(&mut self) -> Result<T>
    where
        T: fast_float2::FastFloat,
    {
        let line = self.next_line()?;
        fast_float2::parse(line).map_err(|_| {
            Error::Client(format!(
                "Cannot parse number from {}",
                String::from_utf8_lossy(line)
            ))
        })
    }

    #[inline]
    fn parse_integer<T>(&mut self) -> Result<T>
    where
        T: atoi::FromRadix10SignedChecked,
    {
        let line = self.next_line()?;
        atoi::atoi(line).ok_or_else(|| {
            Error::Client(format!(
                "Cannot parse integer from {}",
                String::from_utf8_lossy(line)
            ))
        })
    }

    #[inline]
    fn peek_integer<T>(&self) -> Result<T>
    where
        T: atoi::FromRadix10SignedChecked,
    {
        let line = self.peek_line()?;
        atoi::atoi(&line[1..]).ok_or_else(|| {
            Error::Client(format!(
                "Cannot parse integer from {}",
                String::from_utf8_lossy(line)
            ))
        })
    }

    #[inline]
    fn parse_bulk_string(&mut self) -> Result<&'de [u8]> {
        let len = self.parse_integer::<usize>()?;

        // Optimized bounds check
        if !self.has_bytes(len + 2) {
            return eof();
        }

        let end = self.pos + len;

        // Validate \r\n terminator
        if self.buf[end] != b'\r' || self.buf[end + 1] != b'\n' {
            return Err(Error::Client(format!(
                "Expected \\r\\n after bulk string. Got '{}''{}'",
                self.buf[end] as char,
                self.buf[end + 1] as char
            )));
        }

        let result = &self.buf[self.pos..end];
        self.pos = end + 2;
        Ok(result)
    }

    /// The first three bytes provide information about the format of the following string,
    /// which can be txt for plain text, or mkd for markdown.
    /// The fourth byte is always :. Then the real string follows.
    #[inline]
    fn parse_verbatim_string(&mut self) -> Result<&'de [u8]> {
        let full = self.parse_bulk_string()?;
        if full.len() < 4 {
            return Err(Error::Client("Verbatim string too short".to_owned()));
        }
        Ok(&full[4..])
    }

    #[inline(always)]
    fn parse_string(&mut self) -> Result<&'de str> {
        let line = self.next_line()?;
        str::from_utf8(line).map_err(Into::into)
    }

    #[inline(always)]
    fn peek_string(&self) -> Result<Option<&'de str>> {
        let line = self.peek_line()?;
        if line.first() == Some(&SIMPLE_STRING_TAG) {
            Ok(Some(str::from_utf8(&line[1..])?))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn parse_nil(&mut self) -> Result<()> {
        let line = self.next_line()?;
        if line.is_empty() {
            Ok(())
        } else {
            Err(Error::Client(format!(
                "Expected \\r\\n after null. Got '{}'",
                String::from_utf8_lossy(line)
            )))
        }
    }

    #[inline]
    fn parse_boolean(&mut self) -> Result<bool> {
        let line = self.next_line()?;
        match line {
            b"t" => Ok(true),
            b"f" => Ok(false),
            _ => Err(Error::Client(format!(
                "Expected boolean. Got '{}'",
                String::from_utf8_lossy(line)
            ))),
        }
    }

    #[inline]
    fn parse_integer_ex<T>(&mut self) -> Result<T>
    where
        T: atoi::FromRadix10SignedChecked + Default,
    {
        match self.next()? {
            INTEGER_TAG | DOUBLE_TAG => self.parse_integer::<T>(),
            NIL_TAG => {
                self.parse_nil()?;
                Ok(T::default())
            }
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                if bs.is_empty() {
                    Ok(T::default())
                } else {
                    atoi::atoi(bs).ok_or_else(|| {
                        Error::Client(format!(
                            "Cannot parse number from {}",
                            String::from_utf8_lossy(bs)
                        ))
                    })
                }
            }
            SIMPLE_STRING_TAG => {
                let line = self.next_line()?;
                atoi::atoi(line).ok_or_else(|| {
                    Error::Client(format!(
                        "Cannot parse number from {}",
                        String::from_utf8_lossy(line)
                    ))
                })
            }
            ARRAY_TAG => {
                let len = self.parse_integer::<usize>()?;
                if len == 1 && self.next()? == INTEGER_TAG {
                    self.parse_integer::<T>()
                } else {
                    Err(Error::Client("Cannot parse number from array".to_owned()))
                }
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            tag => Err(Error::Client(format!(
                "Cannot parse number from `{}`",
                tag as char
            ))),
        }
    }

    #[inline]
    fn parse_float_ex<T>(&mut self) -> Result<T>
    where
        T: fast_float2::FastFloat + Default,
    {
        match self.next()? {
            INTEGER_TAG | DOUBLE_TAG => self.parse_float::<T>(),
            NIL_TAG => {
                self.parse_nil()?;
                Ok(T::default())
            }
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                if bs.is_empty() {
                    Ok(T::default())
                } else {
                    fast_float2::parse(bs)
                        .map_err(|_| Error::Client("Cannot parse number".to_owned()))
                }
            }
            SIMPLE_STRING_TAG => {
                let line = self.next_line()?;
                fast_float2::parse(line)
                    .map_err(|_| Error::Client("Cannot parse number".to_owned()))
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            _ => Err(Error::Client("Cannot parse number".to_owned())),
        }
    }

    #[inline]
    fn parse_error(&mut self) -> Result<RedisError> {
        let str = self.parse_string()?;
        RedisError::from_str(str)
    }

    #[inline]
    fn parse_blob_error(&mut self) -> Result<RedisError> {
        let bs = self.parse_bulk_string()?;
        let str = str::from_utf8(bs)?;
        RedisError::from_str(str)
    }

    #[inline]
    fn ignore_line(&mut self) -> Result<()> {
        let remaining = &self.buf[self.pos..];
        let idx = memchr(b'\r', remaining).ok_or(Error::EOF)?;

        if idx + 1 >= remaining.len() || remaining[idx + 1] != b'\n' {
            return eof();
        }

        self.pos += idx + 2;
        Ok(())
    }

    #[inline]
    fn ignore_bulk_string(&mut self) -> Result<()> {
        let len = self.parse_integer::<usize>()?;

        if !self.has_bytes(len + 2) {
            return eof();
        }

        let end = self.pos + len;

        if self.buf[end] != b'\r' || self.buf[end + 1] != b'\n' {
            return Err(Error::Client(format!(
                "Expected \\r\\n after bulk string. Got '{}''{}'",
                self.buf[end] as char,
                self.buf[end + 1] as char
            )));
        }

        self.pos = end + 2;
        Ok(())
    }

    #[inline]
    fn ignore_value(&mut self) -> Result<()> {
        self.eat_error = false;
        match self.next()? {
            SIMPLE_STRING_TAG | ERROR_TAG | INTEGER_TAG | DOUBLE_TAG | NIL_TAG | BOOL_TAG => {
                self.ignore_line()
            }
            BULK_STRING_TAG | BLOB_ERROR_TAG | VERBATIM_STRING_TAG => self.ignore_bulk_string(),
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_integer::<usize>()?;
                for _ in 0..len {
                    self.ignore_value()?;
                }
                Ok(())
            }
            MAP_TAG => {
                let len = self.parse_integer::<usize>()? * 2;
                for _ in 0..len {
                    self.ignore_value()?;
                }
                Ok(())
            }
            _ => Err(Error::Client("Cannot parse tag".to_owned())),
        }
    }

    /// Returns an iterator over a RESP Array in byte slices
    pub fn array_chunks(&mut self) -> Result<RespArrayChunks<'de, '_>> {
        match self.next()? {
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_integer::<usize>()?;
                Ok(RespArrayChunks::new(self, len))
            }
            _ => Err(Error::Client("Cannot parse sequence".to_owned())),
        }
    }
}

impl<'de> Deserializer<'de> for &mut RespDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            BULK_STRING_TAG => self.deserialize_bytes(visitor),
            ARRAY_TAG => self.deserialize_seq(visitor),
            MAP_TAG => self.deserialize_map(visitor),
            SET_TAG => self.deserialize_seq(visitor),
            INTEGER_TAG => self.deserialize_i64(visitor),
            DOUBLE_TAG => self.deserialize_f64(visitor),
            SIMPLE_STRING_TAG => self.deserialize_str(visitor),
            NIL_TAG => self.deserialize_option(visitor),
            BOOL_TAG => self.deserialize_bool(visitor),
            VERBATIM_STRING_TAG => self.deserialize_bytes(visitor),
            PUSH_TAG => visitor.visit_map(PushMapAccess::new(self)),
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            byte => Err(Error::Client(format!(
                "Unknown data type '{}' (0x{:02x})",
                byte as char, byte
            ))),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result: bool = match self.next()? {
            INTEGER_TAG => self.parse_integer::<i64>()? != 0,
            DOUBLE_TAG => self.parse_float::<f64>()? != 0.,
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                matches!(bs, b"1" | b"true")
            }
            SIMPLE_STRING_TAG => self.parse_string()? == "OK",
            BOOL_TAG => self.parse_boolean()?,
            NIL_TAG => {
                self.parse_nil()?;
                false
            }
            ERROR_TAG => return Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => return Err(Error::Redis(self.parse_blob_error()?)),
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
            BULK_STRING_TAG => {
                let bs = self.parse_bulk_string()?;
                let str = str::from_utf8(bs)?;
                let mut chars = str.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => c,
                    _ => return Err(Error::Client("Cannot parse to char".to_owned())),
                }
            }
            SIMPLE_STRING_TAG => {
                let str = self.parse_string()?;
                let mut chars = str.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => c,
                    _ => return Err(Error::Client("Cannot parse to char".to_owned())),
                }
            }
            NIL_TAG => {
                self.parse_nil()?;
                '\0'
            }
            ERROR_TAG => return Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => return Err(Error::Redis(self.parse_blob_error()?)),
            _ => return Err(Error::Client("Cannot parse to char".to_owned())),
        };

        visitor.visit_char(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            BULK_STRING_TAG => str::from_utf8(self.parse_bulk_string()?)?,
            VERBATIM_STRING_TAG => str::from_utf8(self.parse_verbatim_string()?)?,
            SIMPLE_STRING_TAG => self.parse_string()?,
            NIL_TAG => {
                self.parse_nil()?;
                ""
            }
            ERROR_TAG => return Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => return Err(Error::Redis(self.parse_blob_error()?)),
            tag => {
                return Err(Error::Client(format!(
                    "Cannot parse to str a RESP value starting with `{}`",
                    tag as char
                )));
            }
        };

        visitor.visit_borrowed_str(result)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            DOUBLE_TAG => self.parse_float::<f64>()?.to_string(),
            BULK_STRING_TAG => str::from_utf8(self.parse_bulk_string()?)?.to_owned(),
            VERBATIM_STRING_TAG => str::from_utf8(self.parse_verbatim_string()?)?.to_owned(),
            NIL_TAG => {
                self.parse_nil()?;
                String::new()
            }
            SIMPLE_STRING_TAG => self.parse_string()?.to_owned(),
            ERROR_TAG => return Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => return Err(Error::Redis(self.parse_blob_error()?)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse to string: `{}`",
                    String::from_utf8_lossy(self.next_line()?).replace("\r\n", "\\r\\n")
                )));
            }
        };

        visitor.visit_string(result)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self.next()? {
            BULK_STRING_TAG => self.parse_bulk_string()?,
            VERBATIM_STRING_TAG => self.parse_verbatim_string()?,
            NIL_TAG => {
                self.parse_nil()?;
                &[]
            }
            SIMPLE_STRING_TAG => self.parse_string()?.as_bytes(),
            ERROR_TAG => return Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => return Err(Error::Redis(self.parse_blob_error()?)),
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
            VERBATIM_STRING_TAG => self.parse_verbatim_string()?.to_vec(),
            NIL_TAG => {
                self.parse_nil()?;
                Vec::new()
            }
            SIMPLE_STRING_TAG => self.parse_string()?.as_bytes().to_vec(),
            ERROR_TAG => return Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => return Err(Error::Redis(self.parse_blob_error()?)),
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
                let len = self.peek_integer::<usize>()?;
                if len == 0 {
                    visitor.visit_none()
                } else {
                    visitor.visit_some(self)
                }
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            _ => visitor.visit_some(self),
        }
    }

    /// deserialize_unit basically means the next value should be ignored
    ///  expect if it is an error.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek()? {
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            _ => {
                self.ignore_value()?;
                visitor.visit_unit()
            }
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
        match self.next()? {
            NIL_TAG => {
                self.parse_nil()?;
                visitor.visit_seq(NilSeqAccess)
            }
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_integer()?;
                visitor.visit_seq(SeqAccess { de: self, len })
            }
            MAP_TAG => {
                let len = self.parse_integer()?;
                visitor.visit_seq(MapAccess { de: self, len })
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            tag => Err(Error::Client(format!(
                "Cannot parse to sequence a RESP value starting with {}",
                tag as char
            ))),
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
        match self.next()? {
            ARRAY_TAG => {
                let len = self.parse_integer()?;
                visitor.visit_map(SeqAccess { de: self, len })
            }
            MAP_TAG => {
                let len = self.parse_integer()?;
                visitor.visit_map(MapAccess { de: self, len })
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            SIMPLE_STRING_TAG => {
                let str = self.parse_string()?;
                Err(Error::Client(format!(
                    "Cannot parse map from simple string `{str}`"
                )))
            }
            c => Err(Error::Client(format!(
                "Cannot parse map from {}",
                c as char
            ))),
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
            de: &mut RespDeserializer,
            array_len: usize,
            fields: &'static [&'static str],
        ) -> Result<bool> {
            Ok(if array_len > fields.len() {
                true
            } else if let Some(s) = de.peek_string()? {
                fields.contains(&s)
            } else {
                false
            })
        }

        match self.next()? {
            ARRAY_TAG => {
                let len = self.parse_integer()?;
                if check_resp2_array(self, len, fields)? {
                    visitor.visit_map(SeqAccess { de: self, len })
                } else {
                    visitor.visit_seq(SeqAccess { de: self, len })
                }
            }
            MAP_TAG => {
                let len = self.parse_integer()?;
                visitor.visit_map(MapAccess { de: self, len })
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
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
                let len: usize = self.parse_integer()?;
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
                let len: usize = self.parse_integer()?;
                if len == 1 {
                    visitor.visit_enum(Enum { de: self })
                } else {
                    Err(Error::Client(
                        "Map len must be 1 to parse an enum".to_owned(),
                    ))
                }
            }
            ERROR_TAG => Err(Error::Redis(self.parse_error()?)),
            BLOB_ERROR_TAG => Err(Error::Redis(self.parse_blob_error()?)),
            _ => Err(Error::Client(format!("Cannot parse enum `{name}`"))),
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
        self.ignore_value()?;
        visitor.visit_unit()
    }
}

struct NilSeqAccess;

impl<'de> serde::de::SeqAccess<'de> for NilSeqAccess {
    type Error = Error;

    #[inline]
    fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        Ok(None)
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
}

struct SeqAccess<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
    len: usize,
}

impl<'de> serde::de::SeqAccess<'de> for SeqAccess<'_, 'de> {
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

impl<'de> serde::de::MapAccess<'de> for SeqAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.len > 0 {
            if self.de.peek()? == ARRAY_TAG {
                let tuple_len = self.de.peek_integer::<usize>()?;
                if tuple_len == 2 {
                    self.de.next_line()?;
                } else {
                    self.len -= 1;
                }
            } else {
                self.len -= 1;
            }
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

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct MapAccess<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
    len: usize,
}

impl<'de> serde::de::MapAccess<'de> for MapAccess<'_, 'de> {
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

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> serde::de::SeqAccess<'de> for MapAccess<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
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

impl<'de> Deserializer<'de> for PairDeserializer<'_, 'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
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

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        struct PairSeqAccess<'a, 'de: 'a> {
            de: &'a mut RespDeserializer<'de>,
            len: usize,
        }

        impl<'de> serde::de::SeqAccess<'de> for PairSeqAccess<'_, 'de> {
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
        }

        visitor.visit_seq(PairSeqAccess { de: self.de, len })
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut RespDeserializer<'de>,
}

impl<'de> EnumAccess<'de> for Enum<'_, 'de> {
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

impl<'de> VariantAccess<'de> for Enum<'_, 'de> {
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
    de: &'a mut RespDeserializer<'de>,
    visited: bool,
}

impl<'de, 'a> PushMapAccess<'de, 'a> {
    #[inline]
    const fn new(de: &'a mut RespDeserializer<'de>) -> Self {
        Self { de, visited: false }
    }
}

impl<'de> serde::de::MapAccess<'de> for PushMapAccess<'de, '_> {
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

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

struct PushDeserializer<'de, 'a> {
    de: &'a mut RespDeserializer<'de>,
}

impl<'de> Deserializer<'de> for PushDeserializer<'de, '_> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.deserialize_seq(visitor)
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

/// An iterator over a RESP Array in byte slices
///
/// # See
/// [`RespDeserializer::array_chunks`](RespDeserializer::array_chunks)
pub struct RespArrayChunks<'de, 'a> {
    de: &'a mut RespDeserializer<'de>,
    len: usize,
    idx: usize,
    pos: usize,
}

impl<'de, 'a> RespArrayChunks<'de, 'a> {
    #[inline]
    pub(crate) const fn new(de: &'a mut RespDeserializer<'de>, len: usize) -> Self {
        let pos = de.get_pos();
        Self {
            de,
            len,
            idx: 0,
            pos,
        }
    }
}

impl<'de> Iterator for RespArrayChunks<'de, '_> {
    type Item = &'de [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            return None;
        }

        self.idx += 1;
        if self.de.ignore_value().is_ok() {
            let pos = self.de.get_pos();
            let chunk = &self.de.buf[self.pos..pos];
            self.pos = pos;
            Some(chunk)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for RespArrayChunks<'_, '_> {
    #[inline]
    fn len(&self) -> usize {
        self.len
    }
}
