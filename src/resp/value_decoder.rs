use crate::{
    resp::{Array, BulkString, Value},
    Error, RedisError, Result,
};
use bytes::{Buf, BytesMut};
use log::trace;
use std::str::FromStr;
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
        b'$' => Ok(decode_bulk_string(buf, idx)?.map(|(bs, pos)| (Value::BulkString(bs), pos))),
        b'*' => Ok(decode_array(buf, idx)?.map(|(v, pos)| (Value::Array(v), pos))),
        b'%' => Ok(decode_map(buf, idx)?.map(|(v, pos)| (Value::Array(v), pos))),
        b'~' => Ok(decode_array(buf, idx)?.map(|(v, pos)| (Value::Array(v), pos))),
        b':' => Ok(decode_integer(buf, idx)?.map(|(i, pos)| (Value::Integer(i), pos))),
        b',' => Ok(decode_double(buf, idx)?.map(|(d, pos)| (Value::Double(d), pos))),
        b'+' => {
            Ok(decode_string(buf, idx)?.map(|(s, pos)| (Value::SimpleString(s.to_owned()), pos)))
        }
        b'-' => decode_string(buf, idx)?
            .map(|(s, pos)| RedisError::from_str(s).map(|e| (Value::Error(e), pos)))
            .transpose(),
        b'_' => Ok(decode_null(buf, idx)?.map(|pos| (Value::BulkString(BulkString::Nil), pos))),
        b'#' => Ok(decode_boolean(buf, idx)?.map(|(i, pos)| (Value::Integer(i), pos))),
        b'=' => Ok(decode_bulk_string(buf, idx)?.map(|(bs, pos)| (Value::BulkString(bs), pos))),
        b'>' => Ok(decode_array(buf, idx)?.map(|(v, pos)| (Value::Push(v), pos))),
        _ => Err(Error::Client(format!(
            "Unknown data type '{}' (0x{:02x})",
            first_byte as char, first_byte
        ))),
    }
}

fn decode_bulk_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(BulkString, usize)>> {
    match decode_integer(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((BulkString::Nil, pos))),
        Some((len, pos)) => {
            let len = usize::try_from(len)
                .map_err(|_| Error::Client("Malformed bulk string len".to_owned()))?;
            if buf.len() - pos < len + 2 {
                Ok(None) // EOF
            } else if buf[pos + len] != b'\r' || buf[pos + len + 1] != b'\n' {
                Err(Error::Client(format!(
                    "Expected \\r\\n after bulk string. Got '{}''{}'",
                    buf[pos + len] as char,
                    buf[pos + len + 1] as char
                )))
            } else {
                Ok(Some((
                    BulkString::Binary(buf[pos..(pos + len)].to_vec()),
                    pos + len + 2,
                )))
            }
        }
    }
}

fn decode_array(buf: &mut BytesMut, idx: usize) -> Result<Option<(Array, usize)>> {
    match decode_integer(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((Array::Nil, pos))),
        Some((len, pos)) => {
            let mut values = Vec::with_capacity(
                usize::try_from(len)
                    .map_err(|_| Error::Client("Malformed array len".to_owned()))?,
            );
            let mut pos = pos;
            for _ in 0..len {
                match decode(buf, pos)? {
                    None => return Ok(None),
                    Some((value, new_pos)) => {
                        values.push(value);
                        pos = new_pos;
                    }
                }
            }
            Ok(Some((Array::Vec(values), pos)))
        }
    }
}

fn decode_map(buf: &mut BytesMut, idx: usize) -> Result<Option<(Array, usize)>> {
    match decode_integer(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => Ok(Some((Array::Nil, pos))),
        Some((len, pos)) => {
            let len = len * 2;
            let mut values = Vec::with_capacity(
                usize::try_from(len).map_err(|_| Error::Client("Malformed map len".to_owned()))?,
            );
            let mut pos = pos;
            for _ in 0..len {
                match decode(buf, pos)? {
                    None => return Ok(None),
                    Some((value, new_pos)) => {
                        values.push(value);
                        pos = new_pos;
                    }
                }
            }
            Ok(Some((Array::Vec(values), pos)))
        }
    }
}

fn decode_string(buf: &mut BytesMut, idx: usize) -> Result<Option<(&str, usize)>> {
    let len = buf.len();
    let mut pos = idx;
    let mut cr = false;

    while pos < len {
        let byte = buf[pos];

        match (cr, byte) {
            (false, b'\r') => cr = true,
            (true, b'\n') => return Ok(Some((std::str::from_utf8(&buf[idx..pos - 1])?, pos + 1))),
            (false, _) => (),
            _ => return Err(Error::Client(format!("Unexpected byte {}", byte))),
        }

        pos += 1;
    }

    Ok(None)
}

fn decode_integer(buf: &mut BytesMut, idx: usize) -> Result<Option<(i64, usize)>> {
    let len = buf.len();
    let mut is_negative = false;
    let mut i = 0i64;
    let mut pos = idx;
    let mut cr = false;

    while pos < len {
        let byte = buf[pos];

        match (cr, is_negative, byte) {
            (false, false, b'-') => is_negative = true,
            (false, false, b'0'..=b'9') => i = i * 10 + i64::from(byte - b'0'),
            (false, true, b'0'..=b'9') => i = i * 10 - i64::from(byte - b'0'),
            (false, _, b'\r') => cr = true,
            (true, _, b'\n') => return Ok(Some((i, pos + 1))),
            _ => return Err(Error::Client(format!("Unexpected byte {}", byte))),
        }

        pos += 1;
    }

    Ok(None)
}

fn decode_double(buf: &mut BytesMut, idx: usize) -> Result<Option<(f64, usize)>> {
    match buf[idx..].iter().position(|b| *b == b'\r') {
        Some(pos) if buf[idx + pos + 1] == b'\n' => {
            let slice = &buf[idx..idx + pos];
            let str = std::str::from_utf8(slice)?;
            let d = str.parse::<f64>()?;
            Ok(Some((d, idx + pos + 2)))
        }
        _ => Err(Error::Client("malformed double".to_owned())),
    }
}

fn decode_null(buf: &mut BytesMut, idx: usize) -> Result<Option<usize>> {
    if buf[idx] != b'\r' || buf[idx + 1] != b'\n' {
        Err(Error::Client(format!(
            "Expected \\r\\n after null. Got '{}''{}'",
            buf[idx] as char,
            buf[idx + 1] as char
        )))
    } else {
        Ok(Some(idx + 2))
    }
}

fn decode_boolean(buf: &mut BytesMut, idx: usize) -> Result<Option<(i64, usize)>> {
    if buf[idx + 1] != b'\r' || buf[idx + 2] != b'\n' {
        Err(Error::Client(format!(
            "Expected \\r\\n after bulk string. Got '{}''{}'",
            buf[idx + 1] as char,
            buf[idx + 2] as char
        )))
    } else {
        match buf[idx] {
            b't' => Ok(Some((1, idx + 2))),
            b'f' => Ok(Some((0, idx + 2))),
            _ => Err(Error::Client(format!(
                "Unexpected boolean character '{}'",
                buf[idx] as char,
            ))),
        }
    }
}
