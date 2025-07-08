use crate::resp::{BulkString, Value};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::{BuildHasher, Hash},
};

/// Marker for a RESP Response
pub trait Response {}

impl<T> Response for T where T: DeserializeOwned {}

/// Marker for a primitive response
pub trait PrimitiveResponse: Response {}

impl PrimitiveResponse for Value {}
impl PrimitiveResponse for () {}
impl PrimitiveResponse for i8 {}
impl PrimitiveResponse for u16 {}
impl PrimitiveResponse for i16 {}
impl PrimitiveResponse for u32 {}
impl PrimitiveResponse for i32 {}
impl PrimitiveResponse for u64 {}
impl PrimitiveResponse for i64 {}
impl PrimitiveResponse for usize {}
impl PrimitiveResponse for isize {}
impl PrimitiveResponse for f32 {}
impl PrimitiveResponse for f64 {}
impl PrimitiveResponse for bool {}
impl PrimitiveResponse for String {}
impl PrimitiveResponse for BulkString {}
impl<T: PrimitiveResponse + DeserializeOwned> PrimitiveResponse for Option<T> {}

/// Marker for a collection response
pub trait CollectionResponse<T>: Response
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self;
}

impl<T> CollectionResponse<T> for ()
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(_iter: I) -> Self {}
}

impl<T> CollectionResponse<T> for [T; 1]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 2]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 3]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 4]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 5]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 6]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 7]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 8]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 9]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 10]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 11]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 12]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 13]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 14]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}
impl<T> CollectionResponse<T> for [T; 15]
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut it = iter.into_iter();
        std::array::from_fn(|_| {
            it.next().expect("Not enough elements in iterator")
        })
    }
}

impl<T> CollectionResponse<T> for Vec<T>
where
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect()
    }
}
impl<T, A> CollectionResponse<T> for SmallVec<A>
where
    A: smallvec::Array<Item = T>,
    T: Response + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect()
    }
}
impl<T, S: BuildHasher + Default> CollectionResponse<T> for HashSet<T, S>
where
    T: Response + Eq + Hash + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect()
    }
}
impl<T> CollectionResponse<T> for BTreeSet<T>
where
    T: Response + Ord + DeserializeOwned,
{
    fn from_collection<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect()
    }
}

/// Marker for key/value collection response
pub trait KeyValueCollectionResponse<K, V>: Response
where
    K: PrimitiveResponse,
    V: Response,
{
}

impl<K, V> KeyValueCollectionResponse<K, V> for ()
where
    K: PrimitiveResponse,
    V: Response,
{
}

impl<K, V> KeyValueCollectionResponse<K, V> for Vec<(K, V)>
where
    K: PrimitiveResponse + DeserializeOwned,
    V: Response + DeserializeOwned,
{
}

impl<K, V, A> KeyValueCollectionResponse<K, V> for SmallVec<A>
where
    A: smallvec::Array<Item = (K, V)>,
    K: PrimitiveResponse + DeserializeOwned,
    V: Response + DeserializeOwned,
{
}

impl<K, V, S: BuildHasher + Default> KeyValueCollectionResponse<K, V> for HashMap<K, V, S>
where
    K: PrimitiveResponse + Eq + Hash + DeserializeOwned,
    V: Response + DeserializeOwned,
{
}

impl<K, V> KeyValueCollectionResponse<K, V> for BTreeMap<K, V>
where
    K: PrimitiveResponse + Ord + DeserializeOwned,
    V: Response + DeserializeOwned,
{
}
