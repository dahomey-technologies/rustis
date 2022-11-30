use crate::{Error, Result};
use std::{fmt, str::from_utf8_unchecked};

use super::BulkString;

#[derive(Clone, PartialEq)]
pub enum CommandArg {
    Str(&'static str),
    String(String),
    Binary(Vec<u8>),
    Integer(i64),
    F32(f32),
    F64(f64),
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
            CommandArg::Integer(_) | CommandArg::F32(_) | CommandArg::F64(_) | CommandArg::Nil => {
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
            CommandArg::Integer(_) | CommandArg::F32(_) | CommandArg::F64(_) | CommandArg::Nil => {
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
            CommandArg::Integer(i) => Ok(*i as usize),
            CommandArg::F32(f) => Ok(*f as usize),
            CommandArg::F64(f) => Ok(*f as usize),
            CommandArg::Nil => Ok(0),
        }
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
        Self::Integer(i)
    }
}

impl From<u64> for CommandArg {
    #[inline]
    fn from(u: u64) -> Self {
        Self::Integer(u as i64)
    }
}

impl From<i32> for CommandArg {
    #[inline]
    fn from(i: i32) -> Self {
        Self::Integer(i64::from(i))
    }
}

impl From<u32> for CommandArg {
    #[inline]
    fn from(u: u32) -> Self {
        Self::Integer(i64::from(u))
    }
}

impl From<i16> for CommandArg {
    #[inline]
    fn from(i: i16) -> Self {
        Self::Integer(i64::from(i))
    }
}

impl From<u16> for CommandArg {
    #[inline]
    fn from(u: u16) -> Self {
        Self::Integer(i64::from(u))
    }
}

impl From<isize> for CommandArg {
    #[inline]
    fn from(i: isize) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<usize> for CommandArg {
    #[inline]
    fn from(u: usize) -> Self {
        Self::Integer(u as i64)
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

impl From<CommandArg> for Result<String> {
    #[inline]
    fn from(bs: CommandArg) -> Self {
        match bs {
            CommandArg::Str(s) => Ok(s.to_owned()),
            CommandArg::String(s) => Ok(s),
            CommandArg::Binary(s) => String::from_utf8(s).map_err(|e| Error::Client(e.to_string())),
            CommandArg::Integer(i) => Ok(i.to_string()),
            CommandArg::F32(f) => Ok(f.to_string()),
            CommandArg::F64(f) => Ok(f.to_string()),
            CommandArg::Nil => Ok(String::from("")),
        }
    }
}

impl ToString for CommandArg {
    fn to_string(&self) -> String {
        match self {
            CommandArg::Str(s) => (*s).to_owned(),
            CommandArg::String(s) => s.clone(),
            CommandArg::Binary(s) => String::from_utf8_lossy(s).into_owned(),
            CommandArg::Integer(i) => i.to_string(),
            CommandArg::F32(f) => f.to_string(),
            CommandArg::F64(f) => f.to_string(),
            CommandArg::Nil => String::from(""),
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
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
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
            CommandArg::Integer(i) => *other == i.to_string(),
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
            CommandArg::Integer(i) => *other == i.to_string(),
            CommandArg::F32(f) => *other == f.to_string(),
            CommandArg::F64(f) => *other == f.to_string(),
            CommandArg::Nil => other.is_empty(),
        }
    }
}