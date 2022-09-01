use crate::{
    resp::{Array, BulkString},
    Error, Result,
};
use std::{collections::HashSet, hash::Hash};

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
            Value::Array(Array::Vec(v)) => {
                let mut result = Vec::<T>::with_capacity(v.len());
                for value in v {
                    let e = T::from_value(value)?;
                    result.push(e);
                }
                Ok(result)
            }
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
            Value::Array(Array::Vec(v)) => {
                let mut result = HashSet::<T>::with_capacity(v.len());
                for value in v {
                    let v = T::from_value(value)?;
                    result.insert(v);
                }
                Ok(result)
            }
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
