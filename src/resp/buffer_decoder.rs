use crate::{Error, Result};
use bytes::{BytesMut, Bytes};
use serde::{de::IgnoredAny, Deserialize};
use tokio_util::codec::Decoder;
use super::RespDeserializer;

pub struct BufferDecoder;

impl Decoder for BufferDecoder {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        let bytes = src.as_ref();
        let mut deserializer = RespDeserializer::from_bytes(bytes);
        let result = IgnoredAny::deserialize(&mut deserializer);
        match result {
            Ok(_) => Ok(Some(src.split_to(deserializer.get_pos()).freeze())),
            Err(Error::Client(e)) if e == "EOF" => Ok(None),
            Err(e) => Err(e),
        }
    }
}
