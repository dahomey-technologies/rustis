use crate::resp::deserialize_byte_buf;
use serde::Deserialize;
use std::{fmt, ops::Deref};

#[derive(Deserialize)]
pub struct BulkString(#[serde(deserialize_with = "deserialize_byte_buf")] Vec<u8>);

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

impl fmt::Debug for BulkString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BulkString").field(&self.0).finish()
    }
}
