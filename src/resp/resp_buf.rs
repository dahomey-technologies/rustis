use crate::{
    Result,
    resp::{
        ARRAY_TAG, BULK_ERROR_TAG, PUSH_TAG, RespDeserializer, RespFrameParser, RespResponse,
        SIMPLE_ERROR_TAG, SIMPLE_STRING_TAG, Value,
    },
};
use bytes::{BufMut, Bytes, BytesMut};
use serde::de::DeserializeOwned;
use std::{fmt, ops::Deref};

/// Represents a [RESP](https://redis.io/docs/reference/protocol-spec/) Buffer incoming from the network
#[derive(Clone, Default, PartialEq)]
pub struct RespBuf(Bytes);

impl RespBuf {
    /// Constructs a new `RespBuf` from a `Bytes` buffer
    #[inline(always)]
    pub fn new() -> Self {
        Self(Bytes::new())
    }

    /// Constructs a new `RespBuf` as a RESP Array from a collection of chunks (byte slices)
    pub fn from_chunks(chunks: &Vec<&[u8]>) -> Self {
        let mut bytes = BytesMut::new();

        bytes.put_u8(ARRAY_TAG);

        let mut temp = itoa::Buffer::new();
        let str = temp.format(chunks.len());
        bytes.put_slice(str.as_bytes());
        bytes.put_slice(b"\r\n");

        for chunk in chunks {
            bytes.put_slice(chunk)
        }

        Self(bytes.freeze())
    }

    /// Constructs a new `RespBuf` from a byte slice
    #[inline]
    pub fn from_slice(data: &[u8]) -> RespBuf {
        RespBuf(Bytes::copy_from_slice(data))
    }

    /// Returns `true` if the RESP Buffer is a push message
    #[inline]
    pub fn is_push_message(&self) -> bool {
        (!self.0.is_empty() && self.0[0] == PUSH_TAG) || self.is_monitor_message()
    }

    /// Returns `true` if the RESP Buffer is a monitor message
    #[inline]
    pub fn is_monitor_message(&self) -> bool {
        self.0.len() > 1 && self.0[0] == SIMPLE_STRING_TAG && (self.0[1] as char).is_numeric()
    }

    /// Returns `true` if the RESP Buffer is a Redis error
    #[inline]
    pub fn is_error(&self) -> bool {
        self.0.len() > 1 && (self.0[0] == SIMPLE_ERROR_TAG || self.0[0] == BULK_ERROR_TAG)
    }

    /// Convert the RESP Buffer to a Rust type `T` by using serde deserialization
    #[inline]
    pub fn to<T: DeserializeOwned>(&self) -> Result<T> {
        let (frame, _) = RespFrameParser::new(&self.0).parse()?;
        let response = RespResponse::new(self.clone(), frame);
        T::deserialize(RespDeserializer::new(response.view()))
    }

    #[inline(always)]
    pub fn bytes(&self) -> &Bytes {
        &self.0
    }

    /// Transform into Bytes
    #[inline(always)]
    pub fn into_bytes(self) -> Bytes {
        self.0
    }

    /// Constructs a new `RespBuf` as a RESP Ok message (+OK\r\n)
    #[inline]
    pub fn ok() -> RespBuf {
        RespBuf(Bytes::from_static(b"+OK\r\n"))
    }

    /// Constructs a new `RespBuf` as a RESP Nil message (_\r\n)
    #[inline]
    pub fn nil() -> RespBuf {
        RespBuf(Bytes::from_static(b"_\r\n"))
    }
}

impl Deref for RespBuf {
    type Target = Bytes;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for RespBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //f.write_str(&String::from_utf8_lossy(&self.0))
        match self.to::<Value>() {
            Ok(value) => {
                let str = format!("{value:?}");
                if str.len() > 1000 {
                    f.write_str(str.get(..1000).unwrap_or("<can't slice to display>"))
                } else {
                    f.write_str(&str)
                }
            }
            Err(e) => f.write_fmt(format_args!("RESP buffer error: {e:?}")),
        }
    }
}

impl fmt::Debug for RespBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl From<Bytes> for RespBuf {
    #[inline(always)]
    fn from(value: Bytes) -> Self {
        RespBuf(value)
    }
}
