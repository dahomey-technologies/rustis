use crate::{
    Error, Result,
    resp::{RespBuf, RespFrameScanner},
};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

pub(crate) struct BufferDecoder;

impl Decoder for BufferDecoder {
    type Item = RespBuf;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        match RespFrameScanner::new(src.as_ref()).scan() {
            Ok(len) => Ok(Some(RespBuf::new(src.split_to(len).freeze()))),
            Err(Error::EOF) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
