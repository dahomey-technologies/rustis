use crate::{
    Error, Result,
    resp::{RespBuf, RespFrameParser, RespResponse},
};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

pub(crate) struct BufferDecoder;

impl Decoder for BufferDecoder {
    type Item = RespResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        match RespFrameParser::new(src.as_ref()).parse() {
            Ok((frame, frame_len)) => {
                let bytes = src.split_to(frame_len).freeze();
                Ok(Some(RespResponse::new(RespBuf::from(bytes), frame)))
            },
            Err(Error::EOF) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
