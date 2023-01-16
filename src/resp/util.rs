use serde::{
    de::{self, DeserializeOwned, Visitor, DeserializeSeed},
    Deserializer,
};
use std::{fmt, marker::PhantomData};

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

pub fn deserialize_byte_buf<'de, D>(deserializer: D) -> std::result::Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ByteBufVisitor;

    impl<'de> Visitor<'de> for ByteBufVisitor {
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

pub struct ByteBufSeed;

impl<'de> DeserializeSeed<'de> for ByteBufSeed {
    type Value = Vec<u8>;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_byte_buf(deserializer)
    }
}

#[derive(Default)]
pub struct VecOfPairsSeed<T1, T2>
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
