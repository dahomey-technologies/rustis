use crate::resp::ArgCounter;
use serde::{
    Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned, DeserializeSeed, Visitor},
};
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

/// Serialize a byte buffer (&\[u8\]) option
pub fn serialize_byte_buf_option<S>(bytes: &Option<&[u8]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(bytes) = bytes {
        serializer.serialize_bytes(bytes)
    } else {
        serializer.serialize_none()
    }
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
    pub(crate) fn new() -> Self {
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

/// Tells whether a flat array holds a struct's fields as field/value pairs
/// rather than as a positional tuple. Both deserializers share this single rule
/// so that the same reply decodes identically through either one.
///
/// RESP3 maps carry the answer in the protocol and never come here; a flat
/// array does not, and the shape has to be guessed from `len` and the first
/// element. It is read as pairs when the length is even *and* the first element
/// is a string naming one of the struct's fields — `first_key` is `None` for any
/// other kind of first element.
///
/// The rule is deliberately blind to `fields.len()`, because the server's field
/// list moves between Redis versions while the struct's does not:
///
/// * a field added to a pair array keeps the length even and the first element
///   unchanged, so it stays pairs and serde ignores the unknown key;
/// * an element appended to a positional array stays positional, and the
///   deserializers' `SeqAccess` stops after the last field, ignoring the extra;
/// * a field the server no longer sends yields a `missing field` error naming
///   it, instead of a shape mismatch.
///
/// It accepts one false positive in exchange: a positional array of even length
/// whose first element happens to equal a field name is read as pairs.
pub(crate) fn is_field_value_array(
    len: usize,
    first_key: Option<&[u8]>,
    fields: &'static [&'static str],
) -> bool {
    len.is_multiple_of(2)
        && first_key.is_some_and(|key| fields.iter().any(|field| field.as_bytes() == key))
}

/// Serialize field name only and skip the boolean value
pub(crate) fn serialize_flag<S: serde::Serializer>(
    _: &bool,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_unit()
}

/// Serializes a slice prefixed by the number of command arguments it produces.
/// Use with #[serde(serialize_with = "serialize_slice_with_arg_count")]
///
/// Clauses such as `LOAD count field [field ...]` or `PARAMS nargs name value
/// [name value ...]` count the *arguments* that follow, not the elements of the
/// collection they came from — one element can produce several arguments, as
/// `identifier AS property` and `name value` do. The count therefore comes from
/// an [`ArgCounter`] dry run rather than from `slice.len()`.
///
/// The dry run is independent of `S`, so this works whichever serializer the
/// value is being written to. When the enclosing value is itself passed to a
/// counting builder method the slice is walked twice, which costs nothing but
/// CPU on a few elements.
pub(crate) fn serialize_slice_with_arg_count<S, T>(
    slice: &[T],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    // The tuple `(usize, &[T])` is serialized sequentially.
    (count_args::<_, S::Error>(&slice)?, slice).serialize(serializer)
}

/// Counts the command arguments a value produces, for the clauses that must
/// declare that number to the server.
pub(crate) fn count_args<T, E>(value: &T) -> Result<usize, E>
where
    T: Serialize + ?Sized,
    E: serde::ser::Error,
{
    let mut counter = ArgCounter::default();
    value.serialize(&mut counter).map_err(E::custom)?;
    Ok(counter.count)
}
