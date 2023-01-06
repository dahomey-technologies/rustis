use crate::{resp::RespDeserializer, Error};
use bytes::BytesMut;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use tokio_util::codec::Decoder;

pub struct RespDecoder<T> {
    phantom: PhantomData<T>,
}

impl<T> Default for RespDecoder<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> Decoder for RespDecoder<T>
where
    T: DeserializeOwned,
{
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Error> {
        let mut deserializer = RespDeserializer::from_bytes(src);
        let result = T::deserialize(&mut deserializer);
        match result {
            Ok(data) => Ok(Some(data)),
            Err(Error::Client(e)) if e == "EOF" => Ok(None),
            Err(e) => Err(e),
        }
    }
}
