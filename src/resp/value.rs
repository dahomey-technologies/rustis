use crate::{
    resp::{Command, FromValue},
    Error, RedisError, Result,
};
use std::fmt;

#[derive(PartialEq)]
pub enum Value {
    SimpleString(String),
    Integer(i64),
    Double(f64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<Value>>),
    Push(Option<Vec<Value>>),
    Error(RedisError),
}

pub struct BulkString(pub Vec<u8>);

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
        Value::BulkString(None)
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match &self {
            Value::SimpleString(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Double(f) => f.to_string(),
            Value::BulkString(s) => match s {
                Some(s) => String::from_utf8_lossy(s).into_owned(),
                None => String::from(""),
            },
            Value::Array(Some(v)) => format!(
                "[{}]",
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Array(None) => "[]".to_string(),
            Value::Push(Some(v)) => format!(
                "Push[{}]",
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Push(None) => "Push[]".to_string(),
            Value::Error(e) => e.to_string(),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f.debug_tuple("SimpleString").field(arg0).finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::BulkString(Some(arg0)) => f.debug_tuple("CommandArg").field(&String::from_utf8_lossy(arg0).into_owned()).finish(),
            Self::BulkString(None) => f.debug_tuple("CommandArg").field(&None::<Vec<u8>>).finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
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

pub(crate) trait IntoValueIterator: Sized {
    fn into_value_iter<T>(self) -> ValueIterator<T>
    where
        T: FromValue;
}

impl IntoValueIterator for Vec<Value> {
    fn into_value_iter<T>(self) -> ValueIterator<T>
    where
        T: FromValue,
    {
        ValueIterator::new(self.into_iter())
    }
}

pub(crate) struct ValueIterator<T>
where
    T: FromValue,
{
    iter: std::vec::IntoIter<Value>,
    phantom: std::marker::PhantomData<T>,
    #[allow(clippy::complexity)]
    next_functor: Box<dyn FnMut(&mut std::vec::IntoIter<Value>) -> Option<Result<T>>>,
}

impl<T> ValueIterator<T>
where
    T: FromValue,
{
    pub fn new(iter: std::vec::IntoIter<Value>) -> Self {
        Self {
            iter,
            phantom: std::marker::PhantomData,
            next_functor: T::next_functor(),
        }
    }
}

impl<T> Iterator for ValueIterator<T>
where
    T: FromValue,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.next_functor)(&mut self.iter)
    }
}
