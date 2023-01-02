use crate::{
    resp::{Command, IntoValueIterator, Value, ValueIterator},
    Error, Result,
};
use smallvec::{smallvec, SmallVec};
use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::Display,
    hash::{BuildHasher, Hash},
};

/// Used to do [`Value`](crate::resp::Value) to user type conversion
/// while consuming the input [`Value`](crate::resp::Value)
pub trait FromValue: Sized {
    /// Converts to this type from the input [`Value`](crate::resp::Value).
    ///
    /// # Errors
    ///
    /// Any parsing error ([`Error::Client`](crate::Error::Client)) due to incompatibility between Value variant and taget type
    fn from_value(value: Value) -> Result<Self>;

    #[inline]
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        Ok(value)
    }
}

impl FromValue for () {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::SimpleString(_) => Ok(()),
            Value::BulkString(bs) if bs.is_empty() => Ok(()),
            Value::Array(a) if a.is_empty() => Ok(()),
            Value::Set(s) if s.is_empty() => Ok(()),
            Value::Map(m) if m.is_empty() => Ok(()),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to ())",
                value
            ))),
        }
    }
}

impl<T, const N: usize> FromValue for [T; N]
where
    T: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Array(v) if v.len() == N => v
                .into_value_iter()
                .collect::<Result<Vec<T>>>()?
                .try_into()
                .map_err(|_| Error::Client("Cannot convert vec to array".to_owned())),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(
                "Cannot convert Nil into static array".to_owned(),
            )),
        }
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(Vec::new()),
            Value::Array(v) => v.into_value_iter().collect(),
            Value::Set(v) => v.into_iter().map(Value::into).collect(),
            Value::Map(v) => ValueIterator::new(v.into_iter().flat_map(|(k, v)| [k, v])).collect(),
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(SmallVec::new()),
            Value::Array(v) => v.into_value_iter().collect(),
            Value::Set(v) => v.into_iter().map(Value::into).collect(),
            Value::Map(v) => ValueIterator::new(v.into_iter().flat_map(|(k, v)| [k, v])).collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Ok(smallvec![value.into()?]),
        }
    }
}

impl<T, S: BuildHasher + Default> FromValue for HashSet<T, S>
where
    T: FromValue + Eq + Hash,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(HashSet::default()),
            Value::Array(v) => v.into_iter().map(Value::into).collect(),
            Value::Set(v) => v.into_iter().map(Value::into).collect(),
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(BTreeSet::new()),
            Value::Array(v) => v.into_iter().map(Value::into).collect(),
            Value::Set(v) => v.into_iter().map(Value::into).collect(),
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(HashMap::default()),
            Value::Array(v) => v.into_value_iter().collect(),
            Value::Map(v) => v
                .into_iter()
                .map(|(k, v)| Ok((k.into()?, v.into()?)))
                .collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to HashMap",
                value
            ))),
        }
    }
}

impl<K, V> FromValue for BTreeMap<K, V>
where
    K: FromValue + Ord,
    V: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(BTreeMap::new()),
            Value::Array(v) => v.into_value_iter().collect(),
            Value::Map(v) => v
                .into_iter()
                .map(|(k, v)| Ok((k.into()?, v.into()?)))
                .collect(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to BTreeMap",
                value
            ))),
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Nil => Ok(None),
            Value::Array(a) if a.is_empty() => Ok(None),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => T::from_value(value).map(|v| Some(v)),
        }
    }
}

impl FromValue for bool {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i != 0),
            Value::Double(d) => Ok(d != 0.),
            Value::SimpleString(s) if s == "OK" => Ok(true),
            Value::Nil => Ok(false),
            Value::BulkString(s) if s == b"0" || s == b"false" => Ok(false),
            Value::BulkString(s) if s == b"1" || s == b"true" => Ok(true),
            Value::Boolean(b) => Ok(b),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to bool",
                value
            ))),
        }
    }
}

impl FromValue for i64 {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => Ok(i),
            Value::Double(d) => Ok(d as i64),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                u64::try_from(i).map_err(|_| Error::Client("Cannot parse result to u64".to_owned()))
            }
            Value::Double(d) => Ok(d as u64),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                i32::try_from(i).map_err(|_| Error::Client("Cannot parse result to i32".to_owned()))
            }
            Value::Double(d) => Ok(d as i32),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                u32::try_from(i).map_err(|_| Error::Client("Cannot parse result to u32".to_owned()))
            }
            Value::Double(d) => Ok(d as u32),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                i16::try_from(i).map_err(|_| Error::Client("Cannot parse result to i16".to_owned()))
            }
            Value::Double(d) => Ok(d as i16),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                u16::try_from(i).map_err(|_| Error::Client("Cannot parse result to u16".to_owned()))
            }
            Value::Double(d) => Ok(d as u16),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => {
                i8::try_from(i).map_err(|_| Error::Client("Cannot parse result i8 u64".to_owned()))
            }
            Value::Double(d) => Ok(d as i8),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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

impl FromValue for isize {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => isize::try_from(i)
                .map_err(|_| Error::Client("Cannot parse result to isize".to_owned())),
            Value::Double(d) => Ok(d as isize),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Integer(i) => usize::try_from(i)
                .map_err(|_| Error::Client("Cannot parse result to usize".to_owned())),
            Value::Double(d) => Ok(d as usize),
            Value::Nil => Ok(0),
            Value::BulkString(s) => {
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(b) => Ok(String::from_utf8_lossy(&b).parse::<f32>()?),
            Value::Nil => Ok(0f32),
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(b) => Ok(String::from_utf8_lossy(&b).parse::<f64>()?),
            Value::Nil => Ok(0f64),
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
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Double(d) => Ok(d.to_string()),
            Value::BulkString(s) => String::from_utf8(s).map_err(|e| Error::Client(e.to_string())),
            Value::Nil => Ok(String::from("")),
            Value::SimpleString(s) => Ok(s),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to String",
                value
            ))),
        }
    }
}

impl FromValue for Vec<u8> {
    #[inline]
    fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::BulkString(s) => Ok(s),
            Value::Nil => Ok(Vec::new()),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to Bytes",
                value
            ))),
        }
    }
}

/// Marker for single value
pub trait FromSingleValue: FromValue {}

impl FromSingleValue for Value {}
impl FromSingleValue for () {}
impl FromSingleValue for i8 {}
impl FromSingleValue for u16 {}
impl FromSingleValue for i16 {}
impl FromSingleValue for u32 {}
impl FromSingleValue for i32 {}
impl FromSingleValue for u64 {}
impl FromSingleValue for i64 {}
impl FromSingleValue for usize {}
impl FromSingleValue for isize {}
impl FromSingleValue for f32 {}
impl FromSingleValue for f64 {}
impl FromSingleValue for bool {}
impl FromSingleValue for String {}
impl FromSingleValue for Vec<u8> {}
impl<T: FromSingleValue> FromSingleValue for Option<T> {}

/// Marker for a collection of values
pub trait FromValueArray<T>: FromValue
where
    T: FromValue,
{
}

impl<T> FromValueArray<T> for () where T: FromValue {}
impl<T, const N: usize> FromValueArray<T> for [T; N] where T: FromValue {}
impl<T> FromValueArray<T> for Vec<T> where T: FromValue {}
impl<T, A> FromValueArray<T> for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: FromValue,
{
}
impl<T, S: BuildHasher + Default> FromValueArray<T> for HashSet<T, S> where T: FromValue + Eq + Hash {}
impl<T> FromValueArray<T> for BTreeSet<T> where T: FromValue + Ord {}

/// Marker for key/value collections
pub trait FromKeyValueArray<K, V>: FromValue
where
    K: FromSingleValue,
    V: FromValue,
{
}

impl<K, V> FromKeyValueArray<K, V> for ()
where
    K: FromSingleValue,
    V: FromValue,
{
}

impl<K, V> FromKeyValueArray<K, V> for Vec<(K, V)>
where
    K: FromSingleValue,
    V: FromValue,
{
}

impl<K, V, A> FromKeyValueArray<K, V> for SmallVec<A>
where
    A: smallvec::Array<Item = (K, V)>,
    K: FromSingleValue,
    V: FromValue,
{
}

impl<K, V, S: BuildHasher + Default> FromKeyValueArray<K, V> for HashMap<K, V, S>
where
    K: FromSingleValue + Eq + Hash,
    V: FromValue,
{
}

impl<K, V> FromKeyValueArray<K, V> for BTreeMap<K, V>
where
    K: FromSingleValue + Ord,
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
    #[inline]
    fn remove_with_result<Q: ?Sized>(&mut self, k: &Q) -> Result<V>
    where
        K: Borrow<Q> + Hash + Eq,
        Q: Hash + Eq + Display,
        S: BuildHasher,
    {
        self.remove(k)
            .ok_or_else(|| Error::Client(format!("Cannot parse field '{}'", k)))
    }

    #[inline]
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
