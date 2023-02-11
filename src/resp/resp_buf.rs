use crate::{
    resp::{RespDeserializer, ARRAY_TAG, BLOB_ERROR_TAG, ERROR_TAG, PUSH_TAG, SIMPLE_STRING_TAG},
    Result,
};
use bytes::{Bytes, BytesMut, BufMut};
use serde::Deserialize;
use std::{fmt, ops::Deref};

/// Represents a [RESP](https://redis.io/docs/reference/protocol-spec/) Buffer incoming from the network
#[derive(Clone)]
pub struct RespBuf(Bytes);

impl RespBuf {
    /// Constructs a new `RespBuf` from a `Bytes` buffer
    pub fn new(bytes: Bytes) -> Self {
        Self(bytes)
    }

    /// Constructs a new `RespBuf` as a RESP Array from a collection of chunks (byte slices)
    pub fn from_chunks(chunks: &Vec::<&[u8]>) -> Self {
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
        self.0.len() > 1 && (self.0[0] == ERROR_TAG || self.0[0] == BLOB_ERROR_TAG)
    }

    /// Convert the RESP Buffer to a Rust type `T` by using serde deserialization
    #[inline]
    pub fn to<'de, T: Deserialize<'de>>(&'de self) -> Result<T> {
        let mut deserializer = RespDeserializer::new(&self.0);
        T::deserialize(&mut deserializer)
    }

    /// Returns the internal buffer as a byte slice
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
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
    type Target = [u8];

    
    #[inline]fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for RespBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = if self.0.len() > 1000 {
            format!(
                "{}...",
                String::from_utf8_lossy(&self.0[..1000]).replace("\r\n", "\\r\\n")
            )
        } else {
            String::from_utf8_lossy(&self.0).replace("\r\n", "\\r\\n")
        };

        f.write_str(&str)
    }
}

impl fmt::Debug for RespBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}
