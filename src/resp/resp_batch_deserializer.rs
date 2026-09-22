use crate::{
    Error, Result,
    resp::{RespDeserializer, RespResponse},
};
use serde::{
    Deserializer,
    de::{DeserializeSeed, Visitor},
};
use std::slice;

/// Reads a batch of replies -- a pipeline's, or the replies `EXEC` returns for a
/// transaction -- as one value.
///
/// # A batch of one
///
/// One reply is the size at which two readings meet: the batch is a sequence of
/// one, and it is also that reply. A caller looping over a variable number of
/// commands reads `Vec<T>` and means the batch; a caller who queued one command
/// reads `T` and means the reply. serde already tells the two apart -- `Vec<T>`
/// asks for a sequence, `Option<i64>` for an option, `i64` for an integer -- so
/// [`Self::deserialize_seq`] answers for the batch and every other form is
/// handed to the lone reply's own deserializer.
///
/// Tuples go to the batch, not to the reply: a tuple is how a caller reads a
/// pipeline whose length it knows, one element per retained command, and that
/// stays true of the pipeline holding one -- `(String,)` is the reading
/// `examples/pipelining.rs` documents for it.
///
/// Above one reply nothing is ambiguous: every form is the batch.
pub(crate) struct RespBatchDeserializer<'de> {
    responses: &'de [RespResponse],
}

impl<'de> RespBatchDeserializer<'de> {
    pub(crate) fn new(responses: &'de [RespResponse]) -> RespBatchDeserializer<'de> {
        RespBatchDeserializer { responses }
    }

    /// The lone reply of a one-reply batch, which answers for the batch on every
    /// form but the sequence ones. See the type's docs.
    #[inline]
    fn single_reply(&self) -> Option<&'de RespResponse> {
        match self.responses {
            [response] => Some(response),
            _ => None,
        }
    }
}

/// Implements the listed methods as: the lone reply of a one-reply batch answers
/// them, any other batch reads as a sequence.
macro_rules! forward_to_single_reply {
    ($($method:ident)*) => {
        $(
            #[inline]
            fn $method<V>(self, visitor: V) -> Result<V::Value>
            where
                V: Visitor<'de>,
            {
                match self.single_reply() {
                    Some(response) => RespDeserializer::new(response.view()?).$method(visitor),
                    None => self.deserialize_seq(visitor),
                }
            }
        )*
    };
}

impl<'de> Deserializer<'de> for &'de RespBatchDeserializer<'de> {
    type Error = Error;

    forward_to_single_reply! {
        deserialize_any deserialize_bool deserialize_i8 deserialize_i16 deserialize_i32
        deserialize_i64 deserialize_i128 deserialize_u8 deserialize_u16 deserialize_u32
        deserialize_u64 deserialize_u128 deserialize_f32 deserialize_f64 deserialize_char
        deserialize_str deserialize_string deserialize_bytes deserialize_byte_buf
        deserialize_option deserialize_map deserialize_identifier deserialize_ignored_any
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess::new(self.responses))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.responses {
            [] => visitor.visit_unit(),
            [response] => RespDeserializer::new(response.view()?).deserialize_unit(visitor),
            _ => self.deserialize_seq(visitor),
        }
    }

    /// A unit struct is a named unit, and reads as one -- an empty batch included.
    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.single_reply() {
            Some(response) => {
                RespDeserializer::new(response.view()?).deserialize_newtype_struct(name, visitor)
            }
            None => self.deserialize_seq(visitor),
        }
    }

    /// A tuple is the batch of a caller who knows its length, whatever that
    /// length is. See the type's docs.
    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.single_reply() {
            Some(response) => {
                RespDeserializer::new(response.view()?).deserialize_struct(name, fields, visitor)
            }
            None => self.deserialize_seq(visitor),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.single_reply() {
            Some(response) => {
                RespDeserializer::new(response.view()?).deserialize_enum(name, variants, visitor)
            }
            None => self.deserialize_seq(visitor),
        }
    }
}

struct SeqAccess<'de> {
    iter: slice::Iter<'de, RespResponse>,
    len: usize,
}

impl<'de> SeqAccess<'de> {
    pub(crate) fn new(bufs: &'de [RespResponse]) -> Self {
        Self {
            len: bufs.len(),
            iter: bufs.iter(),
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(response) => seed
                .deserialize(RespDeserializer::new(response.view()?))
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}
