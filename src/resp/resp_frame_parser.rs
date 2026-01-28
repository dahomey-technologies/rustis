use crate::{ClientError, Error, Result, resp::RespFrame};
use memchr::memchr;
use std::ops::Range;

pub(crate) const SIMPLE_STRING_TAG: u8 = b'+';
pub(crate) const SIMPLE_ERROR_TAG: u8 = b'-';
pub(crate) const INTEGER_TAG: u8 = b':';
pub(crate) const BULK_STRING_TAG: u8 = b'$';
pub(crate) const ARRAY_TAG: u8 = b'*';
pub(crate) const NULL_TAG: u8 = b'_';
pub(crate) const BOOL_TAG: u8 = b'#';
pub(crate) const DOUBLE_TAG: u8 = b',';
pub(crate) const BULK_ERROR_TAG: u8 = b'!';
pub(crate) const VERBATIM_STRING_TAG: u8 = b'=';
pub(crate) const MAP_TAG: u8 = b'%';
pub(crate) const SET_TAG: u8 = b'~';
pub(crate) const PUSH_TAG: u8 = b'>';

pub struct RespFrameParser<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> RespFrameParser<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    #[inline(always)]
    pub fn parse(&mut self) -> Result<(RespFrame, usize)> {
        let tag = self.buf[self.pos];
        self.pos += 1;

        let frame = match tag {
            SIMPLE_STRING_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                RespFrame::SimpleString(start..self.pos - 2)
            }
            SIMPLE_ERROR_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                RespFrame::Error(start..self.pos - 2)
            }
            INTEGER_TAG => {
                let val = self.parse_integer()?; // Parsing direct
                RespFrame::Integer(val)
            }
            DOUBLE_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                let val = fast_float2::parse(&self.buf[start..self.pos - 2])
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?;
                RespFrame::Double(val)
            }
            NULL_TAG => {
                self.parse_crlf()?;
                RespFrame::Null
            }
            BOOL_TAG => {
                let b = match self.buf[self.pos] {
                    b't' => true,
                    b'f' => false,
                    _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
                };
                self.pos += 3; // t/f + \r\n
                RespFrame::Boolean(b)
            }
            BULK_STRING_TAG | VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    let start = self.pos;
                    let need = self.pos + len as usize + 2;
                    if self.buf.len() < need {
                        return Err(Error::EOF);
                    }
                    self.pos = need;
                    RespFrame::BulkString(start..need - 2)
                }
            }
            BULK_ERROR_TAG => {
                let len = self.parse_integer()?;
                let start = self.pos;
                let need = self.pos + len as usize + 2;
                if self.buf.len() < need {
                    return Err(Error::EOF);
                }
                self.pos = need;
                RespFrame::Error(start..need - 2)
            }
            ARRAY_TAG => {
                let (len, ranges) = self.parse_collection(1)?;
                RespFrame::Array { len, ranges }
            }
            MAP_TAG => {
                let (len, ranges) = self.parse_collection(2)?;
                RespFrame::Map { len, ranges }
            }
            SET_TAG => {
                let (len, ranges) = self.parse_collection(1)?;
                RespFrame::Set { len, ranges }
            }
            PUSH_TAG => {
                let (len, ranges) = self.parse_collection(1)?;
                RespFrame::Push { len, ranges }
            }
            _ => return Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        };

        Ok((frame, self.pos))
    }

    pub fn parse_range(&mut self, range: Range<usize>) -> Result<RespFrame> {
        self.pos = range.start;
        let tag = self.buf[self.pos];
        self.pos += 1;

        let frame = match tag {
            SIMPLE_STRING_TAG => RespFrame::SimpleString(self.pos..range.end - 2),
            SIMPLE_ERROR_TAG => RespFrame::Error(self.pos..range.end - 2),
            INTEGER_TAG => {
                let val = atoi::atoi(&self.buf[self.pos..range.end - 2])
                    .ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?;
                RespFrame::Integer(val)
            }
            DOUBLE_TAG => {
                let val = fast_float2::parse(&self.buf[self.pos..range.end - 2])
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?;
                RespFrame::Double(val)
            }
            NULL_TAG => RespFrame::Null,
            BOOL_TAG => {
                let b = match self.buf[self.pos] {
                    b't' => true,
                    b'f' => false,
                    _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
                };
                RespFrame::Boolean(b)
            }
            BULK_STRING_TAG | VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    RespFrame::BulkString(self.pos..self.pos + len as usize)
                }
            }
            BULK_ERROR_TAG => {
                let len = self.parse_integer()?;
                RespFrame::Error(self.pos..self.pos + len as usize)
            }
            ARRAY_TAG => {
                let (len, ranges) = self.parse_collection(1)?;
                RespFrame::Array { len, ranges }
            }
            MAP_TAG => {
                let (len, ranges) = self.parse_collection(2)?;
                RespFrame::Map { len, ranges }
            }
            SET_TAG => {
                let (len, ranges) = self.parse_collection(1)?;
                RespFrame::Set { len, ranges }
            }
            PUSH_TAG => {
                let (len, ranges) = self.parse_collection(1)?;
                RespFrame::Push { len, ranges }
            }
            _ => return Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        };

        self.pos = range.end;

        Ok(frame)
    }

    #[inline]
    fn parse_crlf(&mut self) -> Result<()> {
        let rem = &self.buf[self.pos..];
        let i = memchr(b'\r', rem).ok_or(Error::EOF)?;
        if i + 1 >= rem.len() || rem[i + 1] != b'\n' {
            return Err(Error::EOF);
        }
        self.pos += i + 2;
        Ok(())
    }

    #[inline]
    fn parse_integer(&mut self) -> Result<i64> {
        let start = self.pos;
        self.parse_crlf()?;
        atoi::atoi(&self.buf[start..(self.pos - 2)])
            .ok_or_else(|| Error::Client(ClientError::CannotParseInteger))
    }

    #[inline]
    fn parse_collection(&mut self, multiplier: usize) -> Result<(usize, [Range<u32>; 5])> {
        let len = self.parse_integer()? as usize * multiplier;
        let mut ranges = [0..0, 0..0, 0..0, 0..0, 0..0];
        let range_len = std::cmp::min(len, ranges.len());

        for range in ranges.iter_mut().take(range_len) {
            let start = self.pos;
            self.parse_value()?;
            *range = (start as u32)..(self.pos as u32);
        }

        for _ in range_len..len {
            self.parse_value()?;
        }

        Ok((len, ranges))
    }

    fn parse_value(&mut self) -> Result<()> {
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }

        let tag = self.buf[self.pos];
        self.pos += 1;

        match tag {
            SIMPLE_STRING_TAG | SIMPLE_ERROR_TAG | INTEGER_TAG | DOUBLE_TAG | NULL_TAG
            | BOOL_TAG => self.parse_crlf(),

            BULK_STRING_TAG | BULK_ERROR_TAG | VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    // Null bulk string
                    return Ok(());
                }
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseBulkString));
                }
                let need = self.pos + len as usize + 2;
                if self.buf.len() < need {
                    return Err(Error::EOF);
                }
                if &self.buf[self.pos + len as usize..need] != b"\r\n" {
                    return Err(Error::Client(ClientError::CannotParseBulkString));
                }
                self.pos = need;
                Ok(())
            }
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.parse_integer()? as usize;
                for _ in 0..len {
                    self.parse_value()?;
                }
                Ok(())
            }
            MAP_TAG => {
                let len = self.parse_integer()? as usize * 2;
                for _ in 0..len {
                    self.parse_value()?;
                }
                Ok(())
            }

            tag => Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        }
    }
}
