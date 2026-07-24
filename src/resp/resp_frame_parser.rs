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
pub(crate) const ATTRIBUTE_TAG: u8 = b'|';
pub(crate) const BIG_NUMBER_TAG: u8 = b'(';

/// Maximum collection-nesting depth the parser will descend into before
/// rejecting a frame. RESP replies are shallow in practice (a handful of levels
/// for the deepest cluster/stream introspection commands), so this bound is
/// generous for legitimate traffic while stopping a crafted `*1\r\n*1\r\n…`
/// reply from recursing `parse_value` into a stack overflow (HARD-01), which —
/// unlike a panic — is not catchable and aborts the whole process.
pub(crate) const MAX_NESTING_DEPTH: usize = 128;

/// Maximum byte length the parser accepts for a single bulk string, bulk error
/// or verbatim string, checked against the declared header before the payload
/// is trusted. Matches Redis's default `proto-max-bulk-len` (512 MiB): generous
/// for any legitimate reply, while stopping a crafted `$999999999999\r\n` header
/// from making the streaming decoder accumulate an unbounded buffer (HARD-02).
pub(crate) const MAX_BULK_LENGTH: usize = 512 * 1024 * 1024;

/// Maximum number of elements the parser accepts in a single collection (array,
/// set, push, or map — counted after the map key/value doubling). Bounds an
/// attacker-controlled loop count and any future pre-reservation (HARD-02);
/// generous for real replies.
pub(crate) const MAX_COLLECTION_LENGTH: usize = 128 * 1024 * 1024;

pub struct RespFrameParser<'a> {
    buf: &'a [u8],
    pos: usize,
    /// Current collection-nesting depth, bounded by [`MAX_NESTING_DEPTH`].
    depth: usize,
}

impl<'a> RespFrameParser<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            depth: 0,
        }
    }

    /// Creates a parser over `buf` positioned at `pos`, so parsed frames carry
    /// ranges absolute to `buf` rather than to a sub-slice.
    pub fn new_at(buf: &'a [u8], pos: usize) -> Self {
        Self { buf, pos, depth: 0 }
    }

    /// Enters one collection-nesting level, rejecting the frame once the bound
    /// is crossed. Paired with [`Self::leave`] on the success path; error paths
    /// abort parsing outright, so the counter need not be unwound there.
    #[inline]
    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
        }
        Ok(())
    }

    #[inline]
    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Rejects a bulk-family length that exceeds [`MAX_BULK_LENGTH`]. `len` must
    /// already be known non-negative.
    #[inline]
    fn check_bulk_len(len: i64) -> Result<()> {
        if len as usize > MAX_BULK_LENGTH {
            return Err(Error::Client(ClientError::BulkLengthTooLarge));
        }
        Ok(())
    }

    /// Rejects a collection cardinality that exceeds [`MAX_COLLECTION_LENGTH`].
    #[inline]
    fn check_collection_len(len: usize) -> Result<()> {
        if len > MAX_COLLECTION_LENGTH {
            return Err(Error::Client(ClientError::CollectionLengthTooLarge));
        }
        Ok(())
    }

    /// Consumes any leading RESP3 attribute frames (`|<n>\r\n` followed by `2n`
    /// values). Attributes are out-of-band metadata that may legally precede
    /// *any* reply, so they are skipped at frame-dispatch level and never
    /// surfaced as a frame (RESP-02). Element values are consumed with
    /// [`Self::parse_value`], which itself skips nested attributes.
    #[inline]
    fn skip_attributes(&mut self) -> Result<()> {
        while self.pos < self.buf.len() && self.buf[self.pos] == ATTRIBUTE_TAG {
            self.pos += 1;
            let len = self.parse_integer()?;
            if len < 0 {
                return Err(Error::Client(ClientError::CannotParseMap));
            }
            let len = len as usize * 2;
            Self::check_collection_len(len)?;
            self.enter()?;
            for _ in 0..len {
                self.parse_value()?;
            }
            self.leave();
        }
        Ok(())
    }

    #[inline(always)]
    pub fn parse(&mut self) -> Result<(RespFrame, usize)> {
        self.skip_attributes()?;
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }
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
                // 't' or 'f' + \r\n
                if self.pos + 3 > self.buf.len() {
                    return Err(Error::EOF);
                }
                let b = match self.buf[self.pos] {
                    b't' => true,
                    b'f' => false,
                    _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
                };
                if &self.buf[self.pos + 1..self.pos + 3] != b"\r\n" {
                    return Err(Error::Client(ClientError::CannotParseBoolean));
                }
                self.pos += 3;
                RespFrame::Boolean(b)
            }
            BULK_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    if len < 0 {
                        return Err(Error::Client(ClientError::CannotParseBulkString));
                    }
                    Self::check_bulk_len(len)?;
                    let start = self.pos;
                    let need = self.pos + len as usize + 2;
                    if self.buf.len() < need {
                        return Err(Error::EOF);
                    }
                    if &self.buf[need - 2..need] != b"\r\n" {
                        return Err(Error::Client(ClientError::CannotParseBulkString));
                    }
                    self.pos = need;
                    RespFrame::BulkString(start..need - 2)
                }
            }
            // The first three bytes provide information about the format of the following string,
            // which can be txt for plain text, or mkd for markdown.
            // The fourth byte is always :. Then the real string follows.
            VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    if len < 4 {
                        return Err(Error::Client(ClientError::VerbatimStringTooShort));
                    }
                    Self::check_bulk_len(len)?;
                    let start = self.pos;
                    let need = self.pos + len as usize + 2;
                    if self.buf.len() < need {
                        return Err(Error::EOF);
                    }
                    if &self.buf[need - 2..need] != b"\r\n" {
                        return Err(Error::Client(ClientError::CannotParseVerbatimString));
                    }
                    self.pos = need;
                    RespFrame::BulkString(start + 4..need - 2)
                }
            }
            BULK_ERROR_TAG => {
                let len = self.parse_integer()?;
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseBulkError));
                }
                Self::check_bulk_len(len)?;
                let start = self.pos;
                let need = self.pos + len as usize + 2;
                if self.buf.len() < need {
                    return Err(Error::EOF);
                }
                if &self.buf[need - 2..need] != b"\r\n" {
                    return Err(Error::Client(ClientError::CannotParseBulkError));
                }
                self.pos = need;
                RespFrame::Error(start..need - 2)
            }
            ARRAY_TAG => match self.parse_collection(1)? {
                Some((len, ranges)) => RespFrame::Array { len, ranges },
                None => RespFrame::Null,
            },
            MAP_TAG => match self.parse_collection(2)? {
                Some((len, ranges)) => RespFrame::Map { len, ranges },
                None => RespFrame::Null,
            },
            SET_TAG => match self.parse_collection(1)? {
                Some((len, ranges)) => RespFrame::Set { len, ranges },
                None => RespFrame::Null,
            },
            PUSH_TAG => match self.parse_collection(1)? {
                Some((len, ranges)) => RespFrame::Push { len, ranges },
                None => RespFrame::Null,
            },
            // A big number is an arbitrary-precision integer that does not fit
            // in an `i64`; it is surfaced as its decimal-string payload so the
            // caller can read it as a string (RESP-02).
            BIG_NUMBER_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                RespFrame::BulkString(start..self.pos - 2)
            }
            _ => return Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        };

        Ok((frame, self.pos))
    }

    pub fn parse_range(&mut self, range: Range<usize>) -> Result<RespFrame> {
        self.pos = range.start;
        self.skip_attributes()?;
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }
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
            BULK_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    if len < 0 {
                        return Err(Error::Client(ClientError::CannotParseBulkString));
                    }
                    Self::check_bulk_len(len)?;
                    RespFrame::BulkString(self.pos..self.pos + len as usize)
                }
            }
            // The first three bytes provide information about the format of the following string,
            // which can be txt for plain text, or mkd for markdown.
            // The fourth byte is always :. Then the real string follows.
            VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    if len < 4 {
                        return Err(Error::Client(ClientError::VerbatimStringTooShort));
                    }
                    Self::check_bulk_len(len)?;
                    RespFrame::BulkString(self.pos + 4..self.pos + 4 + len as usize)
                }
            }
            BULK_ERROR_TAG => {
                let len = self.parse_integer()?;
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseBulkError));
                }
                Self::check_bulk_len(len)?;
                RespFrame::Error(self.pos..self.pos + len as usize)
            }
            ARRAY_TAG => match self.parse_collection(1)? {
                Some((len, ranges)) => RespFrame::Array { len, ranges },
                None => RespFrame::Null,
            },
            MAP_TAG => match self.parse_collection(2)? {
                Some((len, ranges)) => RespFrame::Map { len, ranges },
                None => RespFrame::Null,
            },
            SET_TAG => match self.parse_collection(1)? {
                Some((len, ranges)) => RespFrame::Set { len, ranges },
                None => RespFrame::Null,
            },
            PUSH_TAG => match self.parse_collection(1)? {
                Some((len, ranges)) => RespFrame::Push { len, ranges },
                None => RespFrame::Null,
            },
            BIG_NUMBER_TAG => RespFrame::BulkString(self.pos..range.end - 2),
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
        let mut n = 0i64;
        let slice = &self.buf[self.pos..];
        let mut i = 0;

        let sign = if let Some(&b'-') = slice.first() {
            i += 1;
            -1
        } else {
            1
        };

        while i < slice.len() {
            let b = slice[i];
            match b {
                b'0'..=b'9' => {
                    n = n
                        .checked_mul(10)
                        .and_then(|n| n.checked_add((b - b'0') as i64))
                        .ok_or(Error::Client(ClientError::CannotParseInteger))?;
                    i += 1;
                }
                b'\r' => match slice.get(i + 1) {
                    Some(&b'\n') => {
                        self.pos += i + 2;
                        return Ok(n * sign);
                    }
                    Some(_) => return Err(Error::Client(ClientError::CannotParseInteger)),
                    None => return Err(Error::EOF),
                },
                _ => return Err(Error::Client(ClientError::CannotParseInteger)),
            }
        }
        Err(Error::EOF)
    }

    #[inline]
    fn parse_collection(&mut self, multiplier: usize) -> Result<Option<(usize, [Range<u32>; 5])>> {
        let len = self.parse_integer()?;
        if len == -1 {
            // RESP2 null array/map
            return Ok(None);
        }
        if len < 0 {
            return Err(Error::Client(ClientError::CannotParseSequence));
        }
        let len = len as usize * multiplier;
        Self::check_collection_len(len)?;
        let mut ranges = [0..0, 0..0, 0..0, 0..0, 0..0];
        let range_len = std::cmp::min(len, ranges.len());

        self.enter()?;

        for range in ranges.iter_mut().take(range_len) {
            let start = self.pos;
            self.parse_value()?;
            *range = (start as u32)..(self.pos as u32);
        }

        for _ in range_len..len {
            self.parse_value()?;
        }

        self.leave();

        Ok(Some((len, ranges)))
    }

    fn parse_value(&mut self) -> Result<()> {
        self.skip_attributes()?;
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }

        let tag = self.buf[self.pos];
        self.pos += 1;

        match tag {
            SIMPLE_STRING_TAG | SIMPLE_ERROR_TAG | INTEGER_TAG | DOUBLE_TAG | NULL_TAG
            | BOOL_TAG | BIG_NUMBER_TAG => self.parse_crlf(),

            BULK_STRING_TAG | BULK_ERROR_TAG | VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    // Null bulk string
                    return Ok(());
                }
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseBulkString));
                }
                Self::check_bulk_len(len)?;
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
                let len = self.parse_integer()?;
                if len == -1 {
                    // RESP2 null array
                    return Ok(());
                }
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseSequence));
                }
                Self::check_collection_len(len as usize)?;
                self.enter()?;
                for _ in 0..len as usize {
                    self.parse_value()?;
                }
                self.leave();
                Ok(())
            }
            MAP_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    // RESP2 null map
                    return Ok(());
                }
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseMap));
                }
                Self::check_collection_len(len as usize * 2)?;
                self.enter()?;
                for _ in 0..len as usize * 2 {
                    self.parse_value()?;
                }
                self.leave();
                Ok(())
            }

            tag => Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        }
    }
}
