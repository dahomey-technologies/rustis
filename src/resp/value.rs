use crate::{
    resp::{Array, BulkString, Command, FromValue},
    Error, RedisError, Result,
};

#[derive(Debug)]
pub enum Value {
    SimpleString(String),
    Integer(i64),
    Double(f64),
    BulkString(BulkString),
    Array(Array),
    Push(Array),
    Error(RedisError),
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
        Value::BulkString(BulkString::Nil)
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match &self {
            Value::SimpleString(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::Double(f) => f.to_string(),
            Value::BulkString(s) => s.to_string(),
            Value::Array(Array::Vec(v)) => format!(
                "[{}]",
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Array(Array::Nil) => "[]".to_string(),
            Value::Push(Array::Vec(v)) => format!(
                "Push[{}]",
                v.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Push(Array::Nil) => "Push[]".to_string(),
            Value::Error(e) => e.to_string(),
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
