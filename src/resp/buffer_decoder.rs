use super::RespDeserializer;
use crate::{Error, Result, resp::RespBuf};
use bytes::BytesMut;
use serde::{Deserialize, de::IgnoredAny};
use tokio_util::codec::Decoder;

pub(crate) struct BufferDecoder;

impl Decoder for BufferDecoder {
    type Item = RespBuf;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        let bytes = src.as_ref();
        let tag = bytes[0];

        match tag {
            // CATEGORY 1 : Simple scan of the next CRLF
            b'+' | b'-' | b':' | b'_' | b',' | b'#' => {
                if let Some(pos) = memchr::memchr(b'\n', bytes) {
                    return Ok(Some(RespBuf::new(src.split_to(pos + 1).freeze())));
                }
            }
            // CATEGORY 2 : Size reading + jump
            b'$' | b'!' | b'=' => {
                if let Some(pos) = memchr::memchr(b'\n', bytes) {
                    let line = &bytes[1..pos];
                    if let Ok(len) = parse_integer(line) {
                        if len == -1 {
                            // Null bulk strings
                            return Ok(Some(RespBuf::new(src.split_to(pos + 1).freeze())));
                        }
                        let total_size = pos + 1 + (len as usize) + 2;
                        if bytes.len() >= total_size {
                            return Ok(Some(RespBuf::new(src.split_to(total_size).freeze())));
                        }
                    }
                }
            }
            // CATEGORY 3 : Containers -> Fallback To Serde
            b'*' | b'%' | b'~' | b'>' => {
                let mut deserializer = RespDeserializer::new(bytes);
                let result = IgnoredAny::deserialize(&mut deserializer);
                match result {
                    Ok(_) => {
                        return Ok(Some(RespBuf::new(
                            src.split_to(deserializer.get_pos()).freeze(),
                        )));
                    }
                    Err(Error::EOF) => return Ok(None),
                    Err(e) => return Err(e),
                }
            }
            _ => return Err(Error::Client("Invalid TAG".to_string())),
        }

        Ok(None)
    }
}

fn parse_integer(input: &[u8]) -> Result<isize> {
    atoi::atoi(input).ok_or_else(|| {
        Error::Client(format!(
            "Cannot parse integer from {}",
            String::from_utf8_lossy(input)
        ))
    })
}
