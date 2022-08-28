use crate::{Error, Result};
use std::fmt;

//#[derive(Clone)]
pub enum BulkString {
    Str(&'static str),
    String(String),
    Binary(Vec<u8>),
    Integer(i64),
    Nil,
}

impl BulkString {
    pub fn len(&self) -> usize {
        match self {
            BulkString::Str(s) => s.len(),
            BulkString::String(s) => s.len(),
            BulkString::Binary(s) => s.len(),
            BulkString::Integer(_) => unimplemented!(),
            BulkString::Nil => unimplemented!(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            BulkString::Str(s) => s.as_bytes(),
            BulkString::String(s) => s.as_bytes(),
            BulkString::Binary(s) => &s,
            BulkString::Integer(_) => unimplemented!(),
            BulkString::Nil => unimplemented!(),
        }
    }
}

impl From<&'static str> for BulkString {
    fn from(str: &'static str) -> Self {
        Self::Str(str)
    }
}

impl From<String> for BulkString {
    fn from(string: String) -> Self {
        Self::String(string)
    }
}

impl From<Vec<u8>> for BulkString {
    fn from(binary: Vec<u8>) -> Self {
        Self::Binary(binary)
    }
}

impl From<i64> for BulkString {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<u64> for BulkString {
    fn from(u: u64) -> Self {
        Self::Integer(u as i64)
    }
}

impl From<i32> for BulkString {
    fn from(i: i32) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<u32> for BulkString {
    fn from(u: u32) -> Self {
        Self::Integer(u as i64)
    }
}

impl From<isize> for BulkString {
    fn from(i: isize) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<usize> for BulkString {
    fn from(u: usize) -> Self {
        Self::Integer(u as i64)
    }
}

impl From<f32> for BulkString {
    fn from(f: f32) -> Self {
        Self::String(f.to_string())
    }
}

impl From<f64> for BulkString {
    fn from(f: f64) -> Self {
        Self::String(f.to_string())
    }
}

impl From<BulkString> for Result<String> {
    fn from(bs: BulkString) -> Self {
        match bs {
            BulkString::Str(s) => Ok(s.to_owned()),
            BulkString::String(s) => Ok(s),
            BulkString::Binary(s) => String::from_utf8(s).map_err(|e| Error::Parse(e.to_string())),
            BulkString::Integer(i) => Ok(i.to_string()),
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
            Self::Nil => write!(f, "Nil"),
        }
    }
}

/// Initialize a Bulksring as [BulkString::Nil](crate::resp::BulkString::Nil)
impl Default for BulkString {
    fn default() -> Self {
        BulkString::Nil
    }
}
