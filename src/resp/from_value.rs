use crate::{
    resp::{Command, Value},
    Error, Result,
};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
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
    fn from_value(_value: Value) -> Result<Self> {
        unimplemented!()
    }

    #[inline]
    fn from_value_with_command(value: Value, _command: &Command) -> Result<Self> {
        Self::from_value(value)
    }
}

pub trait DeserializeWithCommand<'de>: Sized {
    fn deserialize<D>(deserializer: D, command: Command) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>;
}

impl<T> FromValue for T
where
    T: DeserializeOwned,
{
    fn from_value(value: Value) -> Result<Self> {
        T::deserialize(&value)
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
impl<T: FromSingleValue + DeserializeOwned> FromSingleValue for Option<T> {}

/// Marker for a collection of values
pub trait FromValueArray<T>: FromValue
where
    T: FromValue + DeserializeOwned,
{
}

impl<T> FromValueArray<T> for () where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 2] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 3] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 4] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 5] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 6] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 7] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 8] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 9] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 10] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 11] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 12] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 13] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 14] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for [T; 15] where T: FromValue + DeserializeOwned {}
impl<T> FromValueArray<T> for Vec<T> where T: FromValue + DeserializeOwned {}
impl<T, A> FromValueArray<T> for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: FromValue + DeserializeOwned,
{
}
impl<T, S: BuildHasher + Default> FromValueArray<T> for HashSet<T, S> where
    T: FromValue + Eq + Hash + DeserializeOwned
{
}
impl<T> FromValueArray<T> for BTreeSet<T> where T: FromValue + Ord + DeserializeOwned {}

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
    K: FromSingleValue + DeserializeOwned,
    V: FromValue + DeserializeOwned,
{
}

impl<K, V, A> FromKeyValueArray<K, V> for SmallVec<A>
where
    A: smallvec::Array<Item = (K, V)>,
    K: FromSingleValue + DeserializeOwned,
    V: FromValue + DeserializeOwned,
{
}

impl<K, V, S: BuildHasher + Default> FromKeyValueArray<K, V> for HashMap<K, V, S>
where
    K: FromSingleValue + Eq + Hash + DeserializeOwned,
    V: FromValue + DeserializeOwned,
{
}

impl<K, V> FromKeyValueArray<K, V> for BTreeMap<K, V>
where
    K: FromSingleValue + Ord + DeserializeOwned,
    V: FromValue + DeserializeOwned,
{
}

pub trait HashMapExt<K, V, S> {
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
