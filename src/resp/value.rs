use crate::{
    resp::{Command, FromValue},
    Error, RedisError, Result,
};
use std::fmt;

/// Generic Redis Object Model
///
/// This enum is a direct mapping to [`Redis serialization protocol`](https://redis.io/docs/reference/protocol-spec/) (RESP)
#[derive(PartialEq)]
pub enum Value {
    /// [RESP Simple String](https://redis.io/docs/reference/protocol-spec/#resp-simple-strings)
    SimpleString(String),
    /// [RESP Integer](https://redis.io/docs/reference/protocol-spec/#resp-integers)
    Integer(i64),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Double
    Double(f64),
    /// [RESP Bulk String](https://redis.io/docs/reference/protocol-spec/#resp-bulk-strings)
    BulkString(Vec<u8>),
    /// [RESP Array](https://redis.io/docs/reference/protocol-spec/#resp-arrays)
    Array(Vec<Value>),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Map type
    Map(Vec<(Value, Value)>),
    /// [RESP3](https://github.com/antirez/RESP3/blob/master/spec.md) Push
    Push(Vec<Value>),
    /// [RESP Error](https://redis.io/docs/reference/protocol-spec/#resp-errors)
    Error(RedisError),
    /// [RESP Null](https://redis.io/docs/reference/protocol-spec/#resp-bulk-strings)
    Nil,
}

impl Value {
    /// A [`Value`](crate::resp::Value) to user type conversion that consumes the input value.
    ///
    /// # Errors
    /// Any parsing error ([`Error::Client`](crate::Error::Client)) due to incompatibility between Value variant and taget type
    pub fn into<T>(self) -> Result<T>
    where
        T: FromValue,
    {
        T::from_value(self)
    }

    pub fn into_with_command<T>(self, command: &Command) -> Result<T>
    where
        T: FromValue,
    {
        T::from_value_with_command(self, command)
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match &self {
            Value::SimpleString(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Double(f) => f.to_string(),
            Value::BulkString(s) => String::from_utf8_lossy(s).into_owned(),
            Value::Array(v) => format!(
                "[{}]",
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Map(v) => format!(
                "[{}]",
                v.iter()
                    .map(|(k, v)| format!("({}, {})", k.to_string(), v.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Push(v) => format!(
                "Push[{}]",
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Error(e) => e.to_string(),
            Value::Nil => String::from(""),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f.debug_tuple("SimpleString").field(arg0).finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::BulkString(arg0) => f
                .debug_tuple("BulkString")
                .field(&String::from_utf8_lossy(arg0).into_owned())
                .finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Map(arg0) => f.debug_tuple("Map").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
            Self::Nil => write!(f, "Nil"),
        }
    }
}

pub(crate) trait ResultValueExt {
    fn into_result(self) -> Result<Value>;
    fn map_into_result<T, F>(self, op: F) -> Result<T>
    where
        F: FnOnce(Value) -> T;
}

impl ResultValueExt for Result<Value> {
    #[inline]
    fn into_result(self) -> Result<Value> {
        match self {
            Ok(value) => match value {
                Value::Error(e) => Err(Error::Redis(e)),
                _ => Ok(value),
            },
            Err(e) => Err(e),
        }
    }

    #[inline]
    fn map_into_result<T, F>(self, op: F) -> Result<T>
    where
        F: FnOnce(Value) -> T,
    {
        match self {
            Ok(value) => match value {
                Value::Error(e) => Err(Error::Redis(e)),
                _ => Ok(op(value)),
            },
            Err(e) => Err(e),
        }
    }
}

pub(crate) trait IntoValueIterator<I: Iterator<Item = Value>>: Sized {
    fn into_value_iter<T>(self) -> ValueIterator<T, I>
    where
        T: FromValue;
}

impl IntoValueIterator<std::vec::IntoIter<Value>> for Vec<Value> {
    fn into_value_iter<T>(self) -> ValueIterator<T, std::vec::IntoIter<Value>>
    where
        T: FromValue,
    {
        ValueIterator::new(self.into_iter())
    }
}

pub(crate) struct ValueIterator<T, I>
where
    T: FromValue,
    I: Iterator<Item = Value>,
{
    iter: I,
    phantom: std::marker::PhantomData<T>,
    #[allow(clippy::complexity)]
    next_functor: Box<dyn FnMut(&mut I) -> Option<Result<T>>>,
}

impl<T, I> ValueIterator<T, I>
where
    T: FromValue,
    I: Iterator<Item = Value>,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            phantom: std::marker::PhantomData,
            next_functor: T::next_functor(),
        }
    }
}

impl<T, I> Iterator for ValueIterator<T, I>
where
    T: FromValue,
    I: Iterator<Item = Value>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.next_functor)(&mut self.iter)
    }
}
