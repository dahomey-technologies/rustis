use crate::{resp::Value, Error, RedisError, Result};
use bytes::{Buf, BytesMut};
use log::trace;
use std::str::{self, FromStr};
use tokio_util::codec::Decoder;

pub(crate) struct ValueDecoder;

impl Decoder for ValueDecoder {
    type Item = Value;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Value>> {
        Ok(decode(src, 0)?.map(|(item, pos)| {
            trace!(
                "decode: {}",
                String::from_utf8_lossy(&src.as_ref()[..pos]).replace("\r\n", "\\r\\n")
            );
            src.advance(pos);
            item
        }))
    }
}

fn decode(buf: &mut BytesMut, idx: usize) -> Result<Option<(Value, usize)>> {
    if buf.len() <= idx {
        return Ok(None);
    }

    let first_byte = buf[idx];
    let idx = idx + 1;

    // cf. https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md
    match first_byte {
        b'$' => Ok(decode_bulk_string(buf, idx)?.map(|(bs, pos)| match bs {
            Some(bs) => (Value::BulkString(bs), pos),
            None => (Value::Nil, pos),
        })),
        b'*' => Ok(decode_array(buf, idx)?.map(|(v, pos)| match v {
            Some(v) => (Value::Array(v), pos),
            None => (Value::Nil, pos),
        })),
        b'%' => Ok(decode_map(buf, idx)?.map(|(v, pos)| match v {
            Some(v) => (Value::Array(v), pos),
            None => (Value::Nil, pos),
        })),
        b'~' => Ok(decode_array(buf, idx)?.map(|(v, pos)| match v {
            Some(v) => (Value::Array(v), pos),
            None => (Value::Nil, pos),
        })),
        b':' => Ok(decode_number::<i64>(buf, idx)?.map(|(i, pos)| (Value::Integer(i), pos))),
        b',' => Ok(decode_number::<f64>(buf, idx)?.map(|(d, pos)| (Value::Double(d), pos))),
        b'+' => {
            Ok(decode_string(buf, idx)?.map(|(s, pos)| (Value::SimpleString(s.to_owned()), pos)))
        }
        b'-' => decode_string(buf, idx)?
            .map(|(s, pos)| RedisError::from_str(s).map(|e| (Value::Error(e), pos)))
            .transpose(),
        b'_' => Ok(decode_null(buf, idx)?.map(|pos| (Value::Nil, pos))),
        b'#' => Ok(decode_boolean(buf, idx)?.map(|(i, pos)| (Value::Integer(i), pos))),
        b'=' => Ok(decode_bulk_string(buf, idx)?.map(|(bs, pos)| match bs {
            Some(bs) => (Value::BulkString(bs), pos),
            None => (Value::Nil, pos),
        })),
        b'>' => Ok(decode_array(buf, idx)?.map(|(v, pos)| match v {
            Some(v) => (Value::Push(v), pos),
            None => (Value::Nil, pos),
        })),
        _ => Err(Error::Client(format!(
            "Unknown data type '{}' (0x{:02x})",
            first_byte as char, first_byte
        ))),
    }
}

fn decode_line(buf: &mut BytesMut, idx: usize) -> Result<Option<(&[u8], usize)>> {
    match buf[idx..].iter().position(|b| *b == b'\r') {
        Some(pos) if buf.len() > idx + pos + 1 && buf[idx + pos + 1] == b'\n' => {
            let slice = &buf[idx..idx + pos];
            Ok(Some((slice, pos + idx + 2)))
        }
        _ => Ok(None),
    }
}

fn decode_bulk_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(Option<Vec<u8>>, usize)>> {
    match decode_number::<isize>(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((None, pos))),
        Some((len, pos)) if len >= 0 => {
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
                Ok(Some((Some(buf[pos..(pos + len)].to_vec()), pos + len + 2)))
            }
        }
        _ => Err(Error::Client("Cannot parse bulk string".to_owned())),
    }
}

fn decode_array(buf: &mut BytesMut, idx: usize) -> Result<Option<(Option<Vec<Value>>, usize)>> {
    match decode_number::<isize>(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((None, pos))),
        Some((len, mut pos)) => {
            let mut values = Vec::with_capacity(len as usize);
            for _ in 0..len {
                match decode(buf, pos)? {
                    None => return Ok(None),
                    Some((value, new_pos)) => {
                        values.push(value);
                        pos = new_pos;
                    }
                }
            }
            Ok(Some((Some(values), pos)))
        }
    }
}

fn decode_map(buf: &mut BytesMut, idx: usize) -> Result<Option<(Option<Vec<Value>>, usize)>> {
    match decode_number::<isize>(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((None, pos))),
        Some((len, mut pos)) => {
            let len = len * 2;
            let mut values = Vec::with_capacity(len as usize);
            for _ in 0..len {
                match decode(buf, pos)? {
                    None => return Ok(None),
                    Some((value, new_pos)) => {
                        values.push(value);
                        pos = new_pos;
                    }
                }
            }
            Ok(Some((Some(values), pos)))
        }
    }
}

fn decode_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(&str, usize)>> {
    match decode_line(buf, idx)? {
        Some((slice, pos)) => Ok(Some((str::from_utf8(slice)?, pos))),
        None => Ok(None),
    }
}

fn decode_number<T>(buf: &mut BytesMut, idx: usize) -> Result<Option<(T, usize)>>
where
    T: FromStr,
{
    match decode_line(buf, idx)? {
        Some((slice, pos)) => {
            let str = str::from_utf8(slice)?;
            match str.parse::<T>() {
                Ok(d) => Ok(Some((d, pos))),
                Err(_) => Err(Error::Client("Cannot parse number".to_owned())),
            }
        }
        None => Ok(None),
    }
}

fn decode_null(buf: &mut BytesMut, idx: usize) -> Result<Option<usize>> {
    match decode_line(buf, idx)? {
        Some((slice, pos)) if slice.is_empty() => Ok(Some(pos)),
        None => Ok(None),
        _ => Err(Error::Client("Cannot parse null".to_owned())),
    }
}

fn decode_boolean(buf: &mut BytesMut, idx: usize) -> Result<Option<(i64, usize)>> {
    match decode_line(buf, idx)? {
        Some((slice, pos)) if slice.len() == 1 => match slice[0] {
            b't' => Ok(Some((1, pos))),
            b'f' => Ok(Some((0, pos))),
            _ => Err(Error::Client("Cannot parse boolean".to_owned())),
        },
        None => Ok(None),
        _ => Err(Error::Client("Cannot parse boolean".to_owned())),
    }
}
