use crate::{Error, Result};
use std::{fmt, ops::Deref, str::from_utf8_unchecked};

use super::BulkString;

/// Argument of a [`Command`](crate::resp::Command).
///
/// This enum is meant to hold direct native type until their conversion to RESP
/// without transforming them in a intermeditate representation like `String` or `Vec<u8>`
/// in the objective to avoid temporary allocations
#[derive(Clone, PartialEq)]
pub enum CommandArg {
    /// Static str, most of the time, the name of a key or a sub command
    Str(&'static str),
    /// String
    String(String),
    /// Binary buffer
    Binary(Vec<u8>),
    /// Signed integer
    Signed(i64),
    /// Unsigned integer
    Unsigned(u64),
    /// 32 bit floating value
    F32(f32),
    /// 64 bit floating value
    F64(f64),
    /// Null representation
    Nil,
}

impl CommandArg {
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            CommandArg::Str(s) => s.len(),
            CommandArg::String(s) => s.len(),
            CommandArg::Binary(s) => s.len(),
            CommandArg::Nil => 0,
            CommandArg::Signed(_)
            | CommandArg::Unsigned(_)
            | CommandArg::F32(_)
            | CommandArg::F64(_) => {
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
            CommandArg::Str(s) => s.as_bytes(),
            CommandArg::String(s) => s.as_bytes(),
            CommandArg::Binary(s) => s,
            CommandArg::Signed(_)
            | CommandArg::Unsigned(_)
            | CommandArg::F32(_)
            | CommandArg::F64(_)
            | CommandArg::Nil => {
                unimplemented!()
            }
        }
    }

    pub fn to_usize(&self) -> Result<usize> {
        match self {
            CommandArg::Str(s) => match s.parse::<usize>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            CommandArg::String(s) => match s.parse::<usize>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            CommandArg::Binary(b) => unsafe {
                match from_utf8_unchecked(b).parse::<usize>() {
                    Ok(u) => Ok(u),
                    Err(e) => Err(Error::Client(e.to_string())),
                }
            },
            CommandArg::Signed(i) => Ok(*i as usize),
            CommandArg::Unsigned(u) => Ok(*u as usize),
            CommandArg::F32(f) => Ok(*f as usize),
            CommandArg::F64(f) => Ok(*f as usize),
            CommandArg::Nil => Ok(0),
        }
    }
}

impl From<bool> for CommandArg {
    #[inline]
    fn from(b: bool) -> Self {
        Self::Unsigned(u64::from(b))
    }
}

impl From<char> for CommandArg {
    #[inline]
    fn from(ch: char) -> Self {
        Self::String(ch.to_string())
    }
}

impl From<&'static str> for CommandArg {
    #[inline]
    fn from(str: &'static str) -> Self {
        Self::Str(str)
    }
}

impl From<String> for CommandArg {
    #[inline]
    fn from(string: String) -> Self {
        Self::String(string)
    }
}

impl From<i64> for CommandArg {
    #[inline]
    fn from(i: i64) -> Self {
        Self::Signed(i)
    }
}

impl From<u64> for CommandArg {
    #[inline]
    fn from(u: u64) -> Self {
        Self::Unsigned(u)
    }
}

impl From<i32> for CommandArg {
    #[inline]
    fn from(i: i32) -> Self {
        Self::Signed(i64::from(i))
    }
}

impl From<u32> for CommandArg {
    #[inline]
    fn from(u: u32) -> Self {
        Self::Unsigned(u64::from(u))
    }
}

impl From<i16> for CommandArg {
    #[inline]
    fn from(i: i16) -> Self {
        Self::Signed(i64::from(i))
    }
}

impl From<u16> for CommandArg {
    #[inline]
    fn from(u: u16) -> Self {
        Self::Unsigned(u64::from(u))
    }
}

impl From<isize> for CommandArg {
    #[inline]
    fn from(i: isize) -> Self {
        Self::Signed(i as i64)
    }
}

impl From<usize> for CommandArg {
    #[inline]
    fn from(u: usize) -> Self {
        Self::Unsigned(u as u64)
    }
}

impl From<f32> for CommandArg {
    #[inline]
    fn from(f: f32) -> Self {
        Self::F32(f)
    }
}

impl From<f64> for CommandArg {
    #[inline]
    fn from(f: f64) -> Self {
        Self::F64(f)
    }
}

impl From<BulkString> for CommandArg {
    #[inline]
    fn from(bs: BulkString) -> Self {
        Self::Binary(bs.0)
    }
}

impl Deref for CommandArg {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            CommandArg::Str(s) => s,
            CommandArg::String(s) => s,
            CommandArg::Nil => "",
            _ => unimplemented!(),
        }
    }
}

impl fmt::Debug for CommandArg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Str(arg0) => f.debug_tuple("Str").field(arg0).finish(),
            Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
            Self::Binary(arg0) => f
                .debug_tuple("Binary")
                .field(&String::from_utf8_lossy(arg0).into_owned())
                .finish(),
            Self::Signed(arg0) => f.debug_tuple("Signed").field(arg0).finish(),
            Self::Unsigned(arg0) => f.debug_tuple("Unsigned").field(arg0).finish(),
            Self::F32(arg0) => f.debug_tuple("F32").field(arg0).finish(),
            Self::F64(arg0) => f.debug_tuple("F64").field(arg0).finish(),
            Self::Nil => write!(f, "Nil"),
        }
    }
}

impl PartialEq<String> for CommandArg {
    fn eq(&self, other: &String) -> bool {
        match self {
            CommandArg::Str(s) => *other == *s,
            CommandArg::String(s) => *other == *s,
            CommandArg::Binary(s) => unsafe { *other == core::str::from_utf8_unchecked(s) },
            CommandArg::Signed(i) => *other == i.to_string(),
            CommandArg::Unsigned(u) => *other == u.to_string(),
            CommandArg::F32(f) => *other == f.to_string(),
            CommandArg::F64(f) => *other == f.to_string(),
            CommandArg::Nil => other.is_empty(),
        }
    }
}

impl PartialEq<&str> for CommandArg {
    fn eq(&self, other: &&str) -> bool {
        match self {
            CommandArg::Str(s) => *other == *s,
            CommandArg::String(s) => *other == s,
            CommandArg::Binary(s) => unsafe { *other == core::str::from_utf8_unchecked(s) },
            CommandArg::Signed(i) => {
                let mut temp = itoa::Buffer::new();
                let str = temp.format(*i);
                *other == str
            }
            CommandArg::Unsigned(u) => {
                let mut temp = itoa::Buffer::new();
                let str = temp.format(*u);
                *other == str
            }
            CommandArg::F32(f) => *other == f.to_string(),
            CommandArg::F64(f) => *other == f.to_string(),
            CommandArg::Nil => other.is_empty(),
        }
    }
}
