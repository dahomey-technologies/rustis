use crate::{Error, Result};
use bytes::{BytesMut, Buf};
use tokio_util::codec::Decoder;

pub struct BufferDecoder;

impl Decoder for BufferDecoder {
    type Item = Vec<u8>;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Vec<u8>>> {
        Ok(decode(src, 0)?.map(|pos| {
            let vec = src[..pos].to_vec();
            src.advance(pos);
            vec
        }))
    }
}

fn decode(buf: &mut BytesMut, pos: usize) -> Result<Option<usize>> {
    if buf.len() <= pos {
        return Ok(None);
    }

    let first_byte = buf[pos];
    let pos = pos + 1;

    // cf. https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md
    match first_byte {
        b'$' => sized_string(buf, pos),
        b'*' => array(buf, pos),
        b'%' => map(buf, pos),
        b'~' => array(buf, pos),
        b':' => next_crlf(buf, pos),
        b',' => next_crlf(buf, pos),
        b'+' => next_crlf(buf, pos),
        b'-' => next_crlf(buf, pos),
        b'_' => next_crlf(buf, pos),
        b'#' => next_crlf(buf, pos),
        b'!' => sized_string(buf, pos),
        b'=' => sized_string(buf, pos),
        b'>' => array(buf, pos),
        _ => Err(Error::Client(format!(
            "Unknown data type '{}' (0x{:02x})",
            first_byte as char, first_byte
        ))),
    }
}

fn next_crlf(buf: &mut BytesMut, pos: usize) -> Result<Option<usize>> {
    match &buf[pos..].iter().position(|b| *b == b'\r') {
        Some(new_pos) if buf.len() > pos + new_pos + 1 && buf[pos + new_pos + 1] == b'\n' => {
            Ok(Some(pos + new_pos + 2))
        }
        _ => Ok(None),
    }
}

fn sized_string(buf: &mut BytesMut, pos: usize) -> Result<Option<usize>> {
    match size(buf, pos)? {
        None => Ok(None),
        Some((len, pos)) => {
            let len = len as usize;
            if buf.len() - pos < len + 2 {
                if buf.len() - pos == len + 1 && buf[pos + len] != b'\r' {
                    Err(Error::Client("Cannot parse bulk string".to_owned()))
                } else {
                    Ok(None) // EOF
                }
            } else if buf[pos + len] != b'\r' || buf[pos + len + 1] != b'\n' {
                Err(Error::Client("Cannot parse bulk string".to_owned()))
            } else {
                Ok(Some(pos + len + 2))
            }
        }
    }
}

fn map(buf: &mut BytesMut, pos: usize) -> Result<Option<usize>> {
    match size(buf, pos)? {
        None => Ok(None),
        Some((0, pos)) => Ok(Some(pos)),
        Some((len, mut new_pos)) => {
            for _ in 0..len * 2 {
                let Some(p) = decode(buf, new_pos)? else {
                    return Ok(None);
                };
        
                new_pos = p;
            }
        
            Ok(Some(new_pos))
        }
    }
}

fn array(buf: &mut BytesMut, pos: usize) -> Result<Option<usize>> {
    match size(buf, pos)? {
        None => Ok(None),
        Some((len, mut new_pos)) => {
            for _ in 0..len {
                let Some(p) = decode(buf, new_pos)? else {
                    return Ok(None);
                };
        
                new_pos = p;
            }
        
            Ok(Some(new_pos))
        }
    }
}

fn size(buf: &mut BytesMut, pos: usize) -> Result<Option<(usize, usize)>> {
    let Some(new_pos) = next_crlf(buf, pos)? else {
        return Ok(None);
    };

    let slice = &buf[pos..new_pos - 2];
    let str = std::str::from_utf8(slice)?;
    let Ok(size) = str.parse::<usize>() else {
        return Err(Error::Client("malformed size".to_owned()))
    };

    Ok(Some((size, new_pos)))
}
