use crate::{
    Error, Result,
    resp::{RespDeserializer, RespResponse},
};
use serde::{Deserializer, de::DeserializeSeed, forward_to_deserialize_any};
use std::slice;

pub(crate) struct RespBatchDeserializer<'de> {
    responses: &'de [RespResponse],
}

impl<'de> RespBatchDeserializer<'de> {
    pub fn new(responses: &'de [RespResponse]) -> RespBatchDeserializer<'de> {
        RespBatchDeserializer { responses }
    }
}

impl<'de> Deserializer<'de> for &'de RespBatchDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit_struct newtype_struct tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess::new(self.responses))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.responses.is_empty() {
            visitor.visit_unit()
        } else {
            self.deserialize_seq(visitor)
        }
    }
}

struct SeqAccess<'de> {
    iter: slice::Iter<'de, RespResponse>,
    len: usize,
}

impl<'de> SeqAccess<'de> {
    pub fn new(bufs: &'de [RespResponse]) -> Self {
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
                .deserialize(RespDeserializer::new(response.view()))
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}
