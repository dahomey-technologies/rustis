use crate::{
    ClientError, Error, RedisError, Result,
    resp::{RespBuf, RespDeserializer, RespFrameParser},
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::{fmt, ops::Range};

#[derive(Debug, Clone, PartialEq)]
pub enum RespFrame {
    SimpleString(Range<usize>),
    Integer(i64),
    Double(f64),
    BulkString(Range<usize>),
    Boolean(bool),
    Array { len: usize, ranges: [Range<u32>; 5] },
    Map { len: usize, ranges: [Range<u32>; 5] },
    Set { len: usize, ranges: [Range<u32>; 5] },
    Push { len: usize, ranges: [Range<u32>; 5] },
    Error(Range<usize>),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RespResponse {
    IntegerArray(Vec<i64>),
    OwnedArray(Vec<RespResponse>),
    Frame(RespBuf, RespFrame),
}

impl RespResponse {
    #[inline(always)]
    pub fn new(buf: RespBuf, frame: RespFrame) -> Self {
        Self::Frame(buf, frame)
    }

    #[inline(always)]
    pub fn view(&self) -> RespView<'_> {
        match self {
            RespResponse::IntegerArray(a) => RespView::IntegerArray(a),
            RespResponse::OwnedArray(a) => RespView::OwnedArray(a),
            RespResponse::Frame(buf, frame) => (buf.as_ref(), frame.clone()).into(),
        }
    }

    /// Returns `true` if the RESP Response is a push message
    #[inline(always)]
    pub fn is_push(&self) -> bool {
        matches!(self, RespResponse::Frame(_, RespFrame::Push { .. }))
    }

    /// Returns `true` if the RESP Response is a monitor message
    #[inline(always)]
    pub fn is_monitor(&self) -> bool {
        matches!(self, RespResponse::Frame(buf, RespFrame::SimpleString(r)) if buf.as_ref().get(r.start).is_some_and(|f| f.is_ascii_digit()))
    }

    /// Returns `true` if the RESP Response is a Redis error
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        matches!(self, RespResponse::Frame(_, RespFrame::Error(_)))
    }

    #[inline(always)]
    pub fn null() -> RespResponse {
        Self::Frame(RespBuf::default(), RespFrame::Null)
    }

    #[inline(always)]
    pub fn integer(i: i64) -> RespResponse {
        Self::Frame(RespBuf::default(), RespFrame::Integer(i))
    }

    #[inline(always)]
    pub fn integer_array(a: Vec<i64>) -> RespResponse {
        Self::IntegerArray(a)
    }

    #[inline(always)]
    pub fn owned_array(a: Vec<RespResponse>) -> RespResponse {
        Self::OwnedArray(a)
    }

    /// Constructs a new `Response` as a RESP Ok message (+OK\r\n)
    #[inline(always)]
    pub fn ok() -> RespResponse {
        Self::Frame(
            RespBuf::from(Bytes::from_static(b"+OK\r\n")),
            RespFrame::SimpleString(1..3),
        )
    }

    /// Convert the RESP Response to a Rust type `T` by using serde deserialization
    #[inline]
    pub fn to<T: DeserializeOwned>(&self) -> Result<T> {
        T::deserialize(RespDeserializer::new(self.view()))
    }

    pub fn into_array_iter(self) -> Result<RespResponseIter> {
        match self {
            RespResponse::Frame(buf, RespFrame::Array { len, ranges } | RespFrame::Set { len, ranges }) => {
                Ok(RespResponseIter::new(buf, len, ranges))
            }
            RespResponse::Frame(buf, RespFrame::Error(r)) => Err(Error::Redis(RedisError::try_from(buf.slice(r).as_ref())?)),
            _ => Err(Error::Client(ClientError::Unexpected)),
        }
    }
}

#[derive(PartialEq)]
pub enum RespView<'a> {
    SimpleString(&'a [u8]),
    Integer(i64),
    Double(f64),
    BulkString(&'a [u8]),
    Boolean(bool),
    IntegerArray(&'a [i64]),
    OwnedArray(&'a [RespResponse]),
    Array(RespArrayView<'a>),
    Map(RespArrayView<'a>),
    Set(RespArrayView<'a>),
    Push(RespArrayView<'a>),
    Error(&'a [u8]),
    Null,
}

impl<'a> From<(&'a [u8], RespFrame)> for RespView<'a> {
    fn from((bytes, frame): (&'a [u8], RespFrame)) -> Self {
        match frame {
            RespFrame::SimpleString(r) => RespView::SimpleString(&bytes[r]),
            RespFrame::Integer(i) => RespView::Integer(i),
            RespFrame::Double(f) => RespView::Double(f),
            RespFrame::BulkString(r) => RespView::BulkString(&bytes[r]),
            RespFrame::Boolean(b) => RespView::Boolean(b),
            RespFrame::Array { len, ranges } => {
                RespView::Array(RespArrayView::new(bytes, len, ranges))
            }
            RespFrame::Map { len, ranges } => {
                RespView::Array(RespArrayView::new(bytes, len, ranges))
            }
            RespFrame::Set { len, ranges } => {
                RespView::Array(RespArrayView::new(bytes, len, ranges))
            }
            RespFrame::Push { len, ranges } => {
                RespView::Array(RespArrayView::new(bytes, len, ranges))
            }
            RespFrame::Error(r) => RespView::Error(&bytes[r]),
            RespFrame::Null => RespView::Null,
        }
    }
}

impl<'a> fmt::Debug for RespView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f.debug_tuple("SimpleString").field(&String::from_utf8_lossy(arg0)).finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::BulkString(arg0) => f.debug_tuple("BulkString").field(&String::from_utf8_lossy(arg0)).finish(),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::IntegerArray(arg0) => f.debug_tuple("IntegerArray").field(arg0).finish(),
            Self::OwnedArray(arg0) => f.debug_tuple("OwnedArray").field(arg0).finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Map(arg0) => f.debug_tuple("Map").field(arg0).finish(),
            Self::Set(arg0) => f.debug_tuple("Set").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(&String::from_utf8_lossy(arg0)).finish(),
            Self::Null => write!(f, "Null"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RespArrayView<'a> {
    buf: &'a [u8],
    len: usize,
    ranges: [Range<u32>; 5],
}

impl<'a> RespArrayView<'a> {
    #[inline(always)]
    pub fn new(buf: &'a [u8], len: usize, ranges: [Range<u32>; 5]) -> Self {
        Self { buf, len, ranges }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a> IntoIterator for RespArrayView<'a> {
    type Item = RespView<'a>;
    type IntoIter = RespArrayIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RespArrayIter::new(self.buf, self.len, self.ranges)
    }
}

pub struct RespArrayIter<'a> {
    bytes: &'a [u8],
    len: usize,
    ranges: [Range<u32>; 5],
    current: usize,
    parser: RespFrameParser<'a>,
}

impl<'a> RespArrayIter<'a> {
    #[inline(always)]
    pub fn new(bytes: &'a [u8], len: usize, ranges: [Range<u32>; 5]) -> Self {
        Self {
            bytes,
            len,
            ranges,
            current: 0,
            parser: RespFrameParser::new(bytes),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn has_next(&self) -> bool {
        self.current < self.len
    }
}

impl<'a> Iterator for RespArrayIter<'a> {
    type Item = RespView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.len {
            return None;
        }

        if self.current < self.ranges.len() {
            let range = &self.ranges[self.current];
            let range = range.start as usize..range.end as usize;
            let frame = self.parser.parse_range(range.clone()).ok()?;
            self.current += 1;
            Some((self.bytes, frame).into())
        } else {
            let (frame, _) = self.parser.parse().ok()?;
            self.current += 1;
            Some((self.bytes, frame).into())
        }
    }
}

pub struct RespResponseIter {
    buf: RespBuf,
    len: usize,
    ranges: [Range<u32>; 5],
    current: usize,
    pos: usize,
}

impl RespResponseIter {
    pub fn new(buf: RespBuf, len: usize, ranges: [Range<u32>; 5]) -> Self {
        Self {
            buf,
            len,
            ranges,
            current: 0,
            pos: 0,
        }
    }
}

impl Iterator for RespResponseIter {
    type Item = RespResponse;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.len {
            return None;
        }

        if self.current < self.ranges.len() {
            let range = &self.ranges[self.current];
            let range = range.start as usize..range.end as usize;
            let mut parser = RespFrameParser::new(self.buf.as_ref());
            let frame = parser.parse_range(range.clone()).ok()?;
            self.pos = range.end;
            self.current += 1;
            let sub_buf = RespBuf::from(self.buf.clone().slice(range));
            Some(RespResponse::new(sub_buf, frame))
        } else {
            let mut parser = RespFrameParser::new(&self.buf.as_ref()[self.pos..]);
            let (frame, len) = parser.parse().ok()?;
            let range = self.pos..self.pos + len;
            self.pos += len;
            self.current += 1;
            let sub_buf = RespBuf::from(self.buf.clone().slice(range));
            Some(RespResponse::new(sub_buf, frame))
        }
    }
}
