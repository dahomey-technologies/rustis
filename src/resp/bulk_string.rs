use crate::{Error, Result};
use std::fmt;

pub enum BulkString {
    Str(&'static str),
    String(String),
    Binary(Vec<u8>),
    Integer(i64),
    F32(f32),
    F64(f64),
    Nil,
}

impl BulkString {
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            BulkString::Str(s) => s.len(),
            BulkString::String(s) => s.len(),
            BulkString::Binary(s) => s.len(),
            BulkString::Integer(_) | BulkString::F32(_) | BulkString::F64(_) | BulkString::Nil => {
                unimplemented!()
            }
        }
    }

    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            BulkString::Str(s) => s.as_bytes(),
            BulkString::String(s) => s.as_bytes(),
            BulkString::Binary(s) => s,
            BulkString::Integer(_) | BulkString::F32(_) | BulkString::F64(_) | BulkString::Nil => {
                unimplemented!()
            }
        }
    }
}

impl From<&'static str> for BulkString {
    #[inline]
    fn from(str: &'static str) -> Self {
        Self::Str(str)
    }
}

impl From<String> for BulkString {
    #[inline]
    fn from(string: String) -> Self {
        Self::String(string)
    }
}

impl From<i64> for BulkString {
    #[inline]
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<u64> for BulkString {
    #[inline]
    fn from(u: u64) -> Self {
        Self::Integer(i64::try_from(u).unwrap())
    }
}

impl From<i32> for BulkString {
    #[inline]
    fn from(i: i32) -> Self {
        Self::Integer(i64::from(i))
    }
}

impl From<u32> for BulkString {
    #[inline]
    fn from(u: u32) -> Self {
        Self::Integer(i64::from(u))
    }
}

impl From<i16> for BulkString {
    #[inline]
    fn from(i: i16) -> Self {
        Self::Integer(i64::from(i))
    }
}

impl From<u16> for BulkString {
    #[inline]
    fn from(u: u16) -> Self {
        Self::Integer(i64::from(u))
    }
}

impl From<isize> for BulkString {
    #[inline]
    fn from(i: isize) -> Self {
        Self::Integer(i64::try_from(i).unwrap())
    }
}

impl From<usize> for BulkString {
    #[inline]
    fn from(u: usize) -> Self {
        Self::Integer(u as i64)
    }
}

impl From<f32> for BulkString {
    #[inline]
    fn from(f: f32) -> Self {
        Self::F32(f)
    }
}

impl From<f64> for BulkString {
    #[inline]
    fn from(f: f64) -> Self {
        Self::F64(f)
    }
}

impl From<BulkString> for Result<String> {
    #[inline]
    fn from(bs: BulkString) -> Self {
        match bs {
            BulkString::Str(s) => Ok(s.to_owned()),
            BulkString::String(s) => Ok(s),
            BulkString::Binary(s) => String::from_utf8(s).map_err(|e| Error::Client(e.to_string())),
            BulkString::Integer(i) => Ok(i.to_string()),
            BulkString::F32(f) => Ok(f.to_string()),
            BulkString::F64(f) => Ok(f.to_string()),
            BulkString::Nil => Ok(String::from("")),
        }
    }
}

impl ToString for BulkString {
    fn to_string(&self) -> String {
        match self {
            BulkString::Str(s) => (*s).to_owned(),
            BulkString::String(s) => s.clone(),
            BulkString::Binary(s) => String::from_utf8_lossy(s).into_owned(),
            BulkString::Integer(i) => i.to_string(),
            BulkString::F32(f) => f.to_string(),
            BulkString::F64(f) => f.to_string(),
            BulkString::Nil => String::from(""),
        }
    }
}

impl fmt::Debug for BulkString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Str(arg0) => f.debug_tuple("Str").field(arg0).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Binary(arg0) => f
                .debug_tuple("Binary")
                .field(&String::from_utf8_lossy(arg0).into_owned())
                .finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::F32(arg0) => f.debug_tuple("F32").field(arg0).finish(),
            Self::F64(arg0) => f.debug_tuple("F64").field(arg0).finish(),
            Self::Nil => write!(f, "Nil"),
        }
    }
}

/// Initialize a Bulksring as [`BulkString::Nil`](crate::resp::BulkString::Nil)
impl Default for BulkString {
    fn default() -> Self {
        BulkString::Nil
    }
}
