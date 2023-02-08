use super::RespDeserializer;
use crate::{resp::RespBuf, Error, Result};
use bytes::BytesMut;
use serde::{de::IgnoredAny, Deserialize};
use tokio_util::codec::Decoder;

pub struct BufferDecoder;

impl Decoder for BufferDecoder {
    type Item = RespBuf;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        let bytes = src.as_ref();
        let mut deserializer = RespDeserializer::new(bytes);
        let result = IgnoredAny::deserialize(&mut deserializer);
        match result {
            Ok(_) => Ok(Some(RespBuf(src.split_to(deserializer.get_pos()).freeze()))),
            Err(Error::Client(e)) if e == "EOF" => Ok(None),
            Err(e) => Err(e),
        }
    }
}
