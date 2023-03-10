use crate::{RedisError, Result};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter, Write},
    hash::{Hash, Hasher},
};

/// Generic Redis Object Model
///
/// This enum is a direct mapping to [`Redis serialization protocol`](https://redis.io/docs/reference/protocol-spec/) (RESP)
#[derive(Default)]
pub enum Value {
    /// [RESP Simple String](https://redis.io/docs/reference/protocol-spec/#resp-simple-strings)
    SimpleString(String),
    /// [RESP Integer](https://redis.io/docs/reference/protocol-spec/#resp-integers)
    Integer(i64),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Double
    Double(f64),
    /// [RESP Bulk String](https://redis.io/docs/reference/protocol-spec/#resp-bulk-strings)
    BulkString(Vec<u8>),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Boolean
    Boolean(bool),
    /// [RESP Array](https://redis.io/docs/reference/protocol-spec/#resp-arrays)
    Array(Vec<Value>),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Map type
    Map(HashMap<Value, Value>),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Push
    Set(Vec<Value>),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Set reply
    Push(Vec<Value>),
    /// [RESP Error](https://redis.io/docs/reference/protocol-spec/#resp-errors)
    Error(RedisError),
    /// [RESP Null](https://redis.io/docs/reference/protocol-spec/#resp-bulk-strings)
    #[default]
    Nil,
}

impl Value {
    /// A [`Value`](crate::resp::Value) to user type conversion that consumes the input value.
    ///
    /// # Errors
    /// Any parsing error ([`Error::Client`](crate::Error::Client)) due to incompatibility between Value variant and taget type
    #[inline]
    pub fn into<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(&self)
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::SimpleString(s) => s.hash(state),
            Value::Integer(i) => i.hash(state),
            Value::Double(d) => d.to_string().hash(state),
            Value::BulkString(bs) => bs.hash(state),
            Value::Error(e) => e.hash(state),
            Value::Nil => "_\r\n".hash(state),
            _ => unimplemented!("Hash not implemented for {self}"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SimpleString(l0), Self::SimpleString(r0)) => l0 == r0,
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Double(l0), Self::Double(r0)) => l0 == r0,
            (Self::BulkString(l0), Self::BulkString(r0)) => l0 == r0,
            (Self::Array(l0), Self::Array(r0)) => l0 == r0,
            (Self::Map(l0), Self::Map(r0)) => l0 == r0,
            (Self::Set(l0), Self::Set(r0)) => l0 == r0,
            (Self::Push(l0), Self::Push(r0)) => l0 == r0,
            (Self::Error(l0), Self::Error(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            Value::SimpleString(s) => s.fmt(f),
            Value::Integer(i) => i.fmt(f),
            Value::Double(d) => d.fmt(f),
            Value::BulkString(s) => String::from_utf8_lossy(s).fmt(f),
            Value::Boolean(b) => b.fmt(f),
            Value::Array(v) => {
                f.write_char('[')?;
                let mut first = true;
                for value in v {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    value.fmt(f)?;
                }
                f.write_char(']')
            }
            Value::Map(m) => {
                f.write_char('{')?;
                let mut first = true;
                for (key, value) in m {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    key.fmt(f)?;
                    f.write_str(": ")?;
                    value.fmt(f)?;
                }
                f.write_char('}')
            }
            Value::Set(v) => {
                f.write_char('[')?;
                let mut first = true;
                for value in v {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    value.fmt(f)?;
                }
                f.write_char(']')
            }
            Value::Push(v) => {
                f.write_char('[')?;
                let mut first = true;
                for value in v {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    value.fmt(f)?;
                }
                f.write_char(']')
            }
            Value::Error(e) => e.fmt(f),
            Value::Nil => f.write_str("Nil"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f.debug_tuple("SimpleString").field(arg0).finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::BulkString(arg0) => f
                .debug_tuple("BulkString")
                .field(&String::from_utf8_lossy(arg0).into_owned())
                .finish(),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Map(arg0) => f.debug_tuple("Map").field(arg0).finish(),
            Self::Set(arg0) => f.debug_tuple("Set").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
            Self::Nil => write!(f, "Nil"),
        }
    }
}
