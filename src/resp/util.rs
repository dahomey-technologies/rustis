use serde::{
    Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned, DeserializeSeed, Visitor},
};
use smallvec::SmallVec;
use std::{fmt, marker::PhantomData};

/// Deserialize a Vec of pairs from a sequence
pub fn deserialize_vec_of_pairs<'de, D, T1, T2>(
    deserializer: D,
) -> std::result::Result<Vec<(T1, T2)>, D::Error>
where
    D: Deserializer<'de>,
    T1: DeserializeOwned,
    T2: DeserializeOwned,
{
    struct VecOfPairsVisitor<T1, T2>
    where
        T1: DeserializeOwned,
        T2: DeserializeOwned,
    {
        phantom: PhantomData<(T1, T2)>,
    }

    impl<'de, T1, T2> Visitor<'de> for VecOfPairsVisitor<T1, T2>
    where
        T1: DeserializeOwned,
        T2: DeserializeOwned,
    {
        type Value = Vec<(T1, T2)>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Vec<(T1, T2)>")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut v = if let Some(size) = seq.size_hint() {
                Vec::with_capacity(size / 2)
            } else {
                Vec::new()
            };

            while let Some(first) = seq.next_element()? {
                let Some(second) = seq.next_element()? else {
                    return Err(de::Error::custom("invalid length"));
                };

                v.push((first, second));
            }

            Ok(v)
        }
    }

    deserializer.deserialize_seq(VecOfPairsVisitor {
        phantom: PhantomData,
    })
}

/// Deserialize a Vec of triplets from a sequence
pub fn deserialize_vec_of_triplets<'de, D, T1, T2, T3>(
    deserializer: D,
) -> std::result::Result<Vec<(T1, T2, T3)>, D::Error>
where
    D: Deserializer<'de>,
    T1: DeserializeOwned,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
{
    struct VecOfTripletVisitor<T1, T2, T3>
    where
        T1: DeserializeOwned,
        T2: DeserializeOwned,
        T3: DeserializeOwned,
    {
        phantom: PhantomData<(T1, T2, T3)>,
    }

    impl<'de, T1, T2, T3> Visitor<'de> for VecOfTripletVisitor<T1, T2, T3>
    where
        T1: DeserializeOwned,
        T2: DeserializeOwned,
        T3: DeserializeOwned,
    {
        type Value = Vec<(T1, T2, T3)>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Vec<(T1, T2, T3)>")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut v = if let Some(size) = seq.size_hint() {
                Vec::with_capacity(size / 3)
            } else {
                Vec::new()
            };

            while let Some(first) = seq.next_element()? {
                let Some(second) = seq.next_element()? else {
                    return Err(de::Error::custom("invalid length"));
                };

                let Some(third) = seq.next_element()? else {
                    return Err(de::Error::custom("invalid length"));
                };

                v.push((first, second, third));
            }

            Ok(v)
        }
    }

    deserializer.deserialize_seq(VecOfTripletVisitor {
        phantom: PhantomData,
    })
}

/// Deserialize a byte buffer (Vec\<u8\>)
pub fn deserialize_byte_buf<'de, D>(deserializer: D) -> std::result::Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ByteBufVisitor;

    impl Visitor<'_> for ByteBufVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Vec<u8>")
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }
    }

    deserializer.deserialize_byte_buf(ByteBufVisitor)
}

/// Serialize a byte buffer (&\[u8\])
pub fn serialize_byte_buf<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(bytes)
}

pub(crate) struct ByteBufSeed;

impl<'de> DeserializeSeed<'de> for ByteBufSeed {
    type Value = Vec<u8>;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_byte_buf(deserializer)
    }
}

/// Deserialize a byte slice (&\[u8\])
pub fn deserialize_bytes<'de, D>(deserializer: D) -> std::result::Result<&'de [u8], D::Error>
where
    D: Deserializer<'de>,
{
    struct ByteBufVisitor;

    impl<'de> Visitor<'de> for ByteBufVisitor {
        type Value = &'de [u8];

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("&'de [u8]")
        }

        fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v)
        }
    }

    deserializer.deserialize_bytes(ByteBufVisitor)
}

#[derive(Default)]
pub(crate) struct VecOfPairsSeed<T1, T2>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
{
    phatom: PhantomData<(T1, T2)>,
}

impl<T1, T2> VecOfPairsSeed<T1, T2>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            phatom: PhantomData,
        }
    }
}

impl<'de, T1, T2> DeserializeSeed<'de> for VecOfPairsSeed<T1, T2>
where
    T1: DeserializeOwned,
    T2: DeserializeOwned,
{
    type Value = Vec<(T1, T2)>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_vec_of_pairs(deserializer)
    }
}

/// Serialize field name only and skip the boolean value
pub(crate) fn serialize_flag<S: serde::Serializer>(
    _: &bool,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_unit()
}

/// Serializes a slice prefixed by its length.
/// Use with #[serde(serialize_with = "serialize_slice_with_len")]
pub(crate) fn serialize_slice_with_len<S, T>(
    slice: &[T],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    // Astuce : Le tuple (usize, &[T]) est sérialisé séquentiellement
    (slice.len(), slice).serialize(serializer)
}

pub struct SmallVecWithCounter<T, const N: usize>(usize, SmallVec<[T; N]>);

impl<T, const N: usize> SmallVecWithCounter<T, N> {
    pub fn push(&mut self, value: T) {
        self.0 += 1;
        self.1.push(value);
    }
}
