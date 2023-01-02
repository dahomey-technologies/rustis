use crate::{
    resp::{RawValue, ValueSequenceType},
    Error, Result,
};
use bytes::{Buf, BytesMut};
use std::{
    ops::Range,
    str::{self, FromStr},
};
use tokio_util::codec::Decoder;

pub struct RawValueDecoder;

impl Decoder for RawValueDecoder {
    type Item = Vec<RawValue>;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        let mut raw_values = Vec::<RawValue>::with_capacity(src.len() / 4);
        Ok(decode(src, &mut raw_values, 0)?.map(|pos| {
            src.advance(pos);
            raw_values
        }))
    }
}

fn decode(buf: &mut BytesMut, raw_values: &mut Vec<RawValue>, idx: usize) -> Result<Option<usize>> {
    if buf.len() <= idx {
        return Ok(None);
    }

    let first_byte = buf[idx];
    let idx = idx + 1;

    // cf. https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md
    match first_byte {
        b'$' => Ok(
            decode_bulk_string(buf, idx)?.map(|(range, pos)| match range {
                Some(range) => {
                    raw_values.push(RawValue::BulkString(range));
                    pos
                }
                None => {
                    raw_values.push(RawValue::Nil);
                    pos
                }
            }),
        ),
        b'*' => decode_array(buf, raw_values, ValueSequenceType::Array, idx),
        b'%' => decode_map(buf, raw_values, idx),
        b'~' => decode_array(buf, raw_values, ValueSequenceType::Set, idx),
        b':' => Ok(decode_line(buf, idx)?.map(|(range, pos)| {
            raw_values.push(RawValue::Integer(range));
            pos
        })),
        b',' => Ok(decode_line(buf, idx)?.map(|(range, pos)| {
            raw_values.push(RawValue::Double(range));
            pos
        })),
        b'+' => Ok(decode_line(buf, idx)?.map(|(range, pos)| {
            raw_values.push(RawValue::SimpleString(range));
            pos
        })),
        b'-' => Ok(decode_line(buf, idx)?.map(|(range, pos)| {
            raw_values.push(RawValue::Error(range));
            pos
        })),
        b'_' => Ok(decode_line(buf, idx)?.map(|(_range, pos)| {
            raw_values.push(RawValue::Nil);
            pos
        })),
        b'#' => Ok(decode_line(buf, idx)?.map(|(range, pos)| {
            raw_values.push(RawValue::Bool(range));
            pos
        })),
        b'!' => Ok(
            decode_bulk_string(buf, idx)?.map(|(range, pos)| match range {
                Some(range) => {
                    raw_values.push(RawValue::BlobError(range));
                    pos
                }
                None => {
                    raw_values.push(RawValue::Nil);
                    pos
                }
            }),
        ),
        b'=' => Ok(
            decode_bulk_string(buf, idx)?.map(|(range, pos)| match range {
                Some(range) => {
                    raw_values.push(RawValue::VerbatimString(range));
                    pos
                }
                None => {
                    raw_values.push(RawValue::Nil);
                    pos
                }
            }),
        ),
        b'>' => decode_array(buf, raw_values, ValueSequenceType::Push, idx),
        _ => Err(Error::Client(format!(
            "Unknown data type '{}' (0x{:02x})",
            first_byte as char, first_byte
        ))),
    }
}

fn decode_line(buf: &mut BytesMut, idx: usize) -> Result<Option<(Range<usize>, usize)>> {
    match buf[idx..].iter().position(|b| *b == b'\r') {
        Some(pos) if buf.len() > idx + pos + 1 && buf[idx + pos + 1] == b'\n' => {
            Ok(Some((idx..idx + pos, pos + idx + 2)))
        }
        _ => Ok(None),
    }
}

fn decode_number<T>(buf: &mut BytesMut, idx: usize) -> Result<Option<(T, usize)>>
where
    T: FromStr,
{
    match decode_line(buf, idx)? {
        Some((range, pos)) => {
            let slice = &buf[range];
            let str = str::from_utf8(slice)?;
            match str.parse::<T>() {
                Ok(d) => Ok(Some((d, pos))),
                Err(_) => Err(Error::Client("Cannot parse number".to_owned())),
            }
        }
        None => Ok(None),
    }
}

fn decode_bulk_string(
    buf: &mut BytesMut,
    idx: usize,
) -> Result<Option<(Option<Range<usize>>, usize)>> {
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
                Ok(Some((Some(pos..(pos + len)), pos + len + 2)))
            }
        }
        _ => Err(Error::Client("Cannot parse bulk string".to_owned())),
    }
}

fn decode_array(
    buf: &mut BytesMut,
    raw_values: &mut Vec<RawValue>,
    sequence_type: ValueSequenceType,
    pos: usize,
) -> Result<Option<usize>> {
    match decode_number::<isize>(buf, pos)? {
        None => Ok(None),
        Some((-1, pos)) => {
            raw_values.push(RawValue::Nil);
            Ok(Some(pos))
        }
        Some((len, mut pos)) => {
            raw_values.push(RawValue::new_sequence(sequence_type, len as usize));
            for _ in 0..len {
                match decode(buf, raw_values, pos)? {
                    None => return Ok(None),
                    Some(new_pos) => {
                        pos = new_pos;
                    }
                }
            }
            Ok(Some(pos))
        }
    }
}

fn decode_map(
    buf: &mut BytesMut,
    raw_values: &mut Vec<RawValue>,
    idx: usize,
) -> Result<Option<usize>> {
    match decode_number::<isize>(buf, idx)? {
        None => Ok(None),
        Some((-1, pos)) => {
            raw_values.push(RawValue::Nil);
            Ok(Some(pos))
        }
        Some((len, mut pos)) => {
            raw_values.push(RawValue::Map(len as usize));
            for _ in 0..len {
                match decode(buf, raw_values, pos)? {
                    None => return Ok(None),
                    Some(new_pos) => {
                        pos = new_pos;
                    }
                }

                match decode(buf, raw_values, pos)? {
                    None => return Ok(None),
                    Some(new_pos) => {
                        pos = new_pos;
                    }
                };
            }
            Ok(Some(pos))
        }
    }
}
