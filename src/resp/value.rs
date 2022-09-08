use crate::{
    resp::{Array, BulkString},
    Error, Result,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::Hash,
};

#[derive(Debug)]
pub enum Value {
    SimpleString(String),
    Integer(i64),
    BulkString(BulkString),
    Array(Array),
    Error(String),
}

impl Value {
    pub fn into<T>(self) -> Result<T>
    where
        T: FromValue,
    {
        T::from_value(self)
    }
}

pub trait FromValue: Sized {
    fn from_value(value: Value) -> Result<Self>;
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        Box::new(|iter| {
            let value = iter.next()?;
            Some(value.into())
        })
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Ok(value),
        }
    }
}

impl FromValue for () {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::SimpleString(_) => Ok(()),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to ())",
                value
            ))),
        }
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Nil) => Ok(Vec::new()),
            Value::Array(Array::Vec(v)) => v.from_value_array().collect(),
            _ => Err(Error::Parse("Unexpected result value type".to_owned())),
        }
    }
}

impl<T> FromValue for HashSet<T>
where
    T: FromValue + Eq + Hash,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Nil) => Ok(HashSet::new()),
            Value::Array(Array::Vec(v)) => v.from_value_array().collect(),
            _ => Err(Error::Parse("Unexpected result value type".to_owned())),
        }
    }
}

impl<T> FromValue for BTreeSet<T>
where
    T: FromValue + Ord,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Nil) => Ok(BTreeSet::new()),
            Value::Array(Array::Vec(v)) => v.from_value_array().collect(),
            _ => Err(Error::Parse("Unexpected result value type".to_owned())),
        }
    }
}

impl<K, V> FromValue for HashMap<K, V>
where
    K: FromValue + Eq + Hash + Default,
    V: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Nil) => Ok(HashMap::new()),
            Value::Array(Array::Vec(v)) => v.from_value_array().collect(),
            _ => Err(Error::Parse("Unexpected result value type".to_owned())),
        }
    }
}

impl<K, V> FromValue for BTreeMap<K, V>
where
    K: FromValue + Ord + Default,
    V: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Nil) => Ok(BTreeMap::new()),
            Value::Array(Array::Vec(v)) => v.from_value_array().collect(),
            _ => Err(Error::Parse("Unexpected result value type".to_owned())),
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) => Ok(None),
            _ => T::from_value(value).map(|v| Some(v)),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i != 0),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to bool",
                value
            ))),
        }
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to i64",
                value
            ))),
        }
    }
}

impl FromValue for u64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as u64),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Parse(e.to_string())) {
                    Ok(s) => match s.parse::<u64>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Parse(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to u64",
                value
            ))),
        }
    }
}

impl FromValue for i32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as i32),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to i32",
                value
            ))),
        }
    }
}

impl FromValue for u32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as u32),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to u32",
                value
            ))),
        }
    }
}

impl FromValue for i16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as i16),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to i16",
                value
            ))),
        }
    }
}

impl FromValue for u16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as u16),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to u16",
                value
            ))),
        }
    }
}

impl FromValue for i8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as i8),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to i8",
                value
            ))),
        }
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as u8),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to u8",
                value
            ))),
        }
    }
}

impl FromValue for isize {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as isize),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to isize",
                value
            ))),
        }
    }
}

impl FromValue for usize {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i as usize),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Parse(e.to_string())) {
                    Ok(s) => match s.parse::<usize>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Parse(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to usize",
                value
            ))),
        }
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Binary(b)) => {
                Ok(String::from_utf8_lossy(&b).parse::<f32>().unwrap())
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to f32",
                value
            ))),
        }
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Binary(b)) => {
                Ok(String::from_utf8_lossy(&b).parse::<f64>().unwrap())
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to f64",
                value
            ))),
        }
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(s) => Result::<String>::from(s),
            Value::SimpleString(s) => Ok(s),
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to String",
                value
            ))),
        }
    }
}

impl<T, U> FromValue for (T, U)
where
    T: FromValue + Default,
    U: FromValue + Default,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(Array::Vec(mut values)) => {
                match (values.pop(), values.pop(), values.pop()) {
                    (Some(right), Some(left), None) => Ok((left.into()?, right.into()?)),
                    (None, None, None) => Ok((Default::default(), Default::default())),
                    _ => Err(Error::Parse("Cannot parse result to Tuple".to_owned())),
                }
            }
            _ => Err(Error::Parse(format!(
                "Cannot parse result {:?} to Tuple",
                value
            ))),
        }
    }

    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        Box::new(|iter| {
            let first = iter.next()?;

            match first {
                Value::Array(_) => Some(Self::from_value(first)),
                _ => {
                    let second = iter.next()?;
                    Some(
                        first
                            .into()
                            .and_then(|f| second.into().and_then(|s| Ok((f, s)))),
                    )
                }
            }
        })
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match &self {
            Value::SimpleString(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            Value::BulkString(s) => s.to_string(),
            Value::Array(a) => match a {
                Array::Vec(v) => format!(
                    "[{}]",
                    v.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Array::Nil => "[]".to_string(),
            },
            Value::Error(e) => e.clone(),
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

pub(crate) trait ValueVecExt: Sized {
    fn from_value_array<T>(self) -> FromArrayIterator<T>
    where
        T: FromValue;
}

impl ValueVecExt for Vec<Value> {
    fn from_value_array<T>(self) -> FromArrayIterator<T>
    where
        T: FromValue,
    {
        FromArrayIterator::new(self.into_iter())
    }
}

pub(crate) struct FromArrayIterator<T>
where
    T: FromValue,
{
    iter: std::vec::IntoIter<Value>,
    phantom: std::marker::PhantomData<T>,
    next_functor: Box<dyn FnMut(&mut std::vec::IntoIter<Value>) -> Option<Result<T>>>,
}

impl<T> FromArrayIterator<T>
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

impl<T> Iterator for FromArrayIterator<T>
where
    T: FromValue,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.next_functor)(&mut self.iter)
    }
}

/// Marker for single value array
pub trait FromSingleValueArray<T>: FromValue
where
    T: FromValue,
{
}

impl<T> FromSingleValueArray<T> for Vec<T> where T: FromValue {}
impl<T> FromSingleValueArray<T> for HashSet<T> where T: FromValue + Eq + Hash {}
impl<T> FromSingleValueArray<T> for BTreeSet<T> where T: FromValue + Ord {}

/// Marker for key/value array
pub trait FromKeyValueValueArray<K, V>: FromValue
where
    K: FromValue,
    V: FromValue,
{
}

impl<K, V> FromKeyValueValueArray<K, V> for Vec<(K, V)>
where
    K: FromValue + Default,
    V: FromValue + Default,
{
}

impl<K, V> FromKeyValueValueArray<K, V> for HashMap<K, V>
where
    K: FromValue + Eq + Hash + Default,
    V: FromValue + Default,
{
}

impl<K, V> FromKeyValueValueArray<K, V> for BTreeMap<K, V>
where
    K: FromValue + Ord + Default,
    V: FromValue + Default,
{
}
