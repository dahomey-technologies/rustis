use crate::resp::{deserialize_byte_buf, serialize_byte_buf};
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref};

/// Represents the [Bulk String](https://redis.io/docs/reference/protocol-spec/#resp-bulk-strings) RESP type
#[derive(Deserialize, Serialize, Hash, PartialEq, Eq, Clone)]
pub struct BulkString(
    #[serde(
        deserialize_with = "deserialize_byte_buf",
        serialize_with = "serialize_byte_buf"
    )]
    Vec<u8>,
);

impl BulkString {
    /// Constructs a new `BulkString` from a bytes buffer
    #[inline]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Returns the internal buffer as a byte slice
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for BulkString {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BulkString> for Vec<u8> {
    #[inline]
    fn from(bs: BulkString) -> Self {
        bs.0
    }
}

impl From<Vec<u8>> for BulkString {
    #[inline]
    fn from(bytes: Vec<u8>) -> Self {
        BulkString(bytes)
    }
}

impl<const N: usize> From<&[u8; N]> for BulkString {
    #[inline]
    fn from(bytes: &[u8; N]) -> Self {
        BulkString(bytes.to_vec())
    }
}

impl fmt::Debug for BulkString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BulkString").field(&self.0).finish()
    }
}

impl fmt::Display for BulkString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(String::from_utf8_lossy(&self.0).as_ref())
    }
}
