use crate::{
    resp::{Array, BulkString, FromValue},
    Error, Result,
};

#[derive(Debug)]
pub enum Value {
    SimpleString(String),
    Integer(i64),
    Double(f64),
    BulkString(BulkString),
    Array(Array),
    Error(String),
}

impl Value {
    /// A [`Value`](crate::resp::Value) to user type conversion that consumes the input value.
    /// 
    /// # Errors
    /// Any parsing error ([`Error::Parse`](crate::Error::Parse)) due to incompatibility between Value variant and tagert type
    pub fn into<T>(self) -> Result<T>
    where
        T: FromValue,
    {
        T::from_value(self)
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::BulkString(BulkString::Nil)
    }
}

pub(crate) trait ResultValueExt {
    fn into_result(self) -> Result<Value>;
    fn map_into_result<T, F>(self, op: F) -> Result<T>
    where
        F: FnOnce(Value) -> T;
}

impl ResultValueExt for Result<Value> {
    fn into_result(self) -> Result<Value> {
        match self {
            Ok(value) => match value {
                Value::Error(e) => Err(Error::Redis(e)),
                _ => Ok(value),
            },
            Err(e) => Err(e),
        }
    }

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