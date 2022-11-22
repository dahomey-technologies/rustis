use crate::{
    resp::{Array, BulkString, Command, IntoValueIterator, Value},
    Error, Result,
};
use smallvec::{smallvec, SmallVec};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Display,
    hash::{BuildHasher, Hash},
};

pub trait FromValue: Sized {
    /// Used to do [`Value`](crate::resp::Value) to user type conversion
    ///
    /// # Errors
    ///
    /// Any parsing error ([`Error::Client`](crate::Error::Client)) due to incompatibility between Value variant and taget type
    fn from_value(value: Value) -> Result<Self>;

    fn from_value_with_command(value: Value, _command: &Command) -> Result<Self> {
        Self::from_value(value)
    }

    #[must_use]
    #[allow(clippy::complexity)]
    fn next_functor<I: Iterator<Item = Value>>() -> Box<dyn FnMut(&mut I) -> Option<Result<Self>>> {
        Box::new(|iter| {
            let value = iter.next()?;
            Some(value.into())
        })
    }
}

impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self> {
        Ok(value)
    }
}

impl FromValue for () {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::SimpleString(_) => Ok(()),
            _ => Err(Error::Client(format!(
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
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(Vec::new()),
            Value::Array(Array::Vec(v)) => v.into_value_iter().collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Ok(vec![value.into()?]),
        }
    }
}

impl<T, A> FromValue for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(SmallVec::new()),
            Value::Array(Array::Vec(v)) => v.into_value_iter().collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Ok(smallvec![value.into()?]),
        }
    }
}

impl<T, S: BuildHasher + Default> FromValue for HashSet<T, S>
where
    T: FromValue + Eq + Hash,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(HashSet::default()),
            Value::Array(Array::Vec(v)) => v.into_value_iter().collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => {
                let mut hash_set = HashSet::default();
                hash_set.insert(value.into()?);
                Ok(hash_set)
            }
        }
    }
}

impl<T> FromValue for BTreeSet<T>
where
    T: FromValue + Ord,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(BTreeSet::new()),
            Value::Array(Array::Vec(v)) => v.into_value_iter().collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Ok(BTreeSet::from([value.into()?])),
        }
    }
}

impl<K, V, S: BuildHasher + Default> FromValue for HashMap<K, V, S>
where
    K: FromValue + Eq + Hash,
    V: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(HashMap::default()),
            Value::Array(Array::Vec(v)) => v.into_value_iter().collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Unexpected result value type".to_owned())),
        }
    }
}

impl<K, V> FromValue for BTreeMap<K, V>
where
    K: FromValue + Ord,
    V: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(BTreeMap::new()),
            Value::Array(Array::Vec(v)) => v.into_value_iter().collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Unexpected result value type".to_owned())),
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(None),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => T::from_value(value).map(|v| Some(v)),
        }
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i != 0),
            Value::SimpleString(s) if s == "OK" => Ok(true),
            Value::BulkString(BulkString::Nil) => Ok(false),
            Value::BulkString(BulkString::Binary(s)) if s == b"0" => Ok(false),
            Value::BulkString(BulkString::Binary(s)) if s == b"1" => Ok(true),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
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
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<i64>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<i64>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to i64",
                value
            ))),
        }
    }
}

impl FromValue for u64 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                u64::try_from(i).map_err(|_| Error::Client("Cannot parse result to u64".to_owned()))
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<u64>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<u64>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to u64",
                value
            ))),
        }
    }
}

impl FromValue for i32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                i32::try_from(i).map_err(|_| Error::Client("Cannot parse result to i32".to_owned()))
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<i32>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<i32>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to i32",
                value
            ))),
        }
    }
}

impl FromValue for u32 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                u32::try_from(i).map_err(|_| Error::Client("Cannot parse result to u32".to_owned()))
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<u32>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<u32>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to u32",
                value
            ))),
        }
    }
}

impl FromValue for i16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                i16::try_from(i).map_err(|_| Error::Client("Cannot parse result to i16".to_owned()))
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<i16>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<i16>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to i16",
                value
            ))),
        }
    }
}

impl FromValue for u16 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                u16::try_from(i).map_err(|_| Error::Client("Cannot parse result to u16".to_owned()))
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<u16>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<u16>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to u16",
                value
            ))),
        }
    }
}

impl FromValue for i8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                i8::try_from(i).map_err(|_| Error::Client("Cannot parse result i8 u64".to_owned()))
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<i8>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<i8>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to i8",
                value
            ))),
        }
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => u8::try_from(i)
                .map_err(|_| Error::Client("Cannot parse result tu8o u64".to_owned())),
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<u8>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<u8>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to u8",
                value
            ))),
        }
    }
}

impl FromValue for isize {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => isize::try_from(i)
                .map_err(|_| Error::Client("Cannot parse result to isize".to_owned())),
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<isize>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<isize>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to isize",
                value
            ))),
        }
    }
}

impl FromValue for usize {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => usize::try_from(i)
                .map_err(|_| Error::Client("Cannot parse result to usize".to_owned())),
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0),
            Value::BulkString(BulkString::Binary(s)) => {
                match String::from_utf8(s).map_err(|e| Error::Client(e.to_string())) {
                    Ok(s) => match s.parse::<usize>() {
                        Ok(u) => Ok(u),
                        Err(e) => Err(Error::Client(e.to_string())),
                    },
                    Err(e) => Err(e),
                }
            }
            Value::SimpleString(s) => match s.parse::<usize>() {
                Ok(u) => Ok(u),
                Err(e) => Err(Error::Client(e.to_string())),
            },
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
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
                Ok(String::from_utf8_lossy(&b).parse::<f32>()?)
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0f32),
            Value::SimpleString(s) => Ok(s.parse::<f32>()?),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
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
                Ok(String::from_utf8_lossy(&b).parse::<f64>()?)
            }
            Value::BulkString(BulkString::Nil) | Value::Array(Array::Nil) => Ok(0f64),
            Value::SimpleString(s) => Ok(s.parse::<f64>()?),
            Value::Double(d) => Ok(d),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
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
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to String",
                value
            ))),
        }
    }
}

impl FromValue for BulkString {
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(s) => Ok(s),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to BulkString",
                value
            ))),
        }
    }
}

/// Marker for single value array
pub trait FromSingleValueArray<T>: FromValue
where
    T: FromValue,
{
}

impl<T> FromSingleValueArray<T> for Vec<T> where T: FromValue {}
impl<T, A> FromSingleValueArray<T> for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: FromValue,
{
}
impl<T, S: BuildHasher + Default> FromSingleValueArray<T> for HashSet<T, S> where
    T: FromValue + Eq + Hash
{
}
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
    K: FromValue,
    V: FromValue,
{
}

impl<K, V, A> FromKeyValueValueArray<K, V> for SmallVec<A>
where
    A: smallvec::Array<Item = (K, V)>,
    K: FromValue,
    V: FromValue,
{
}

impl<K, V, S: BuildHasher + Default> FromKeyValueValueArray<K, V> for HashMap<K, V, S>
where
    K: FromValue + Eq + Hash,
    V: FromValue,
{
}

impl<K, V> FromKeyValueValueArray<K, V> for BTreeMap<K, V>
where
    K: FromValue + Ord,
    V: FromValue,
{
}

pub(crate) trait HashMapExt<K, V, S> {
    fn remove_with_result<Q: ?Sized>(&mut self, k: &Q) -> Result<V>
    where
        K: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq + Display,
        S: BuildHasher;

    fn remove_or_default<Q: ?Sized>(&mut self, k: &Q) -> V
    where
        K: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq + Display,
        S: BuildHasher,
        V: Default;
}

impl<K, V, S> HashMapExt<K, V, S> for HashMap<K, V, S> {
    fn remove_with_result<Q: ?Sized>(&mut self, k: &Q) -> Result<V>
    where
        K: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq + Display,
        S: BuildHasher,
    {
        self.remove(k)
            .ok_or_else(|| Error::Client(format!("Cannot parse field '{}'", k)))
    }

    fn remove_or_default<Q: ?Sized>(&mut self, k: &Q) -> V
    where
        K: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq + Display,
        S: BuildHasher,
        V: Default,
    {
        self.remove(k).unwrap_or_default()
    }
}
