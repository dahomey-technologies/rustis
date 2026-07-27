//! Where one scalar sits in a frame's bytes.
//!
//! A scalar is a tag byte, a payload, and a `\r\n`, and reading it back is a
//! matter of arithmetic on lengths the server announced. This module owns that
//! arithmetic for all ten scalar tags, so the two passes that need it — the
//! parser's forward pass, which only wants to know where the element ends, and
//! the tape reader, which wants the value — can never disagree on the answer.
//!
//! Nothing here decodes: a value's bytes are located, never interpreted. The
//! numeric parsing lives in the reader, in whichever task asked for the value.

use crate::{
    ClientError, Error, Result,
    resp::{
        BIG_NUMBER_TAG, BOOL_TAG, BULK_ERROR_TAG, BULK_STRING_TAG, DOUBLE_TAG, INTEGER_TAG,
        NULL_TAG, SIMPLE_ERROR_TAG, SIMPLE_STRING_TAG, VERBATIM_STRING_TAG,
    },
};
use memchr::memchr;
use std::ops::Range;

/// A `max_bulk_length` that caps nothing. See [`scalar_value`] and
/// [`scalar_span`] for why the read-back path passes it.
const NO_BULK_LIMIT: usize = usize::MAX;

/// The normalized kind of a scalar element, as recovered from its bytes by
/// [`scalar_value`]. This is what the tape reader dispatches on, independent
/// of the exact RESP tag (a big number and a bulk string both read back as
/// [`ScalarKind::BulkString`], for instance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarKind {
    SimpleString,
    Error,
    Integer,
    Double,
    BulkString,
    Boolean,
    Null,
}

/// Everything one read of a scalar recovers: its kind, the byte range of its
/// value payload, and the offset one past the whole element.
///
/// Never handed out whole. Each caller goes through the projection that answers
/// its own question — [`scalar_end`], [`scalar_value`] or [`scalar_span`] — so no
/// call site carries a field it does not read.
struct ScalarLayout {
    kind: ScalarKind,
    value: Range<usize>,
    end: usize,
}

/// Rejects a bulk-family length that exceeds `max_bulk_length`. `len` must
/// already be known non-negative.
#[inline]
fn check_bulk_len(len: i64, max_bulk_length: usize) -> Result<()> {
    if len as usize > max_bulk_length {
        return Err(Error::Client(ClientError::BulkLengthTooLarge));
    }
    Ok(())
}

/// Total buffer offset an incomplete bulk-family value at `pos` ends at, when its
/// length header is already fully buffered — the read buffer can then be reserved
/// to exactly that size in one shot instead of doubling toward it. Returns `None`
/// when `pos` is not a length-prefixed scalar, when its header line has not
/// arrived yet (only a few bytes are pending — the doubling fallback is cheap
/// there), or when the announced length is negative/nil or exceeds
/// `max_bulk_length` (the same cap the parser enforces, so a hostile length
/// cannot drive an unbounded reservation).
///
/// Only `$` bulk strings and `=` verbatim strings are considered: they are the
/// scalars whose payload is large enough for the reallocation cost to bite. All
/// other frames stay on the existing incremental-growth path.
#[inline]
pub(crate) fn bulk_value_end(data: &[u8], pos: usize, max_bulk_length: usize) -> Option<usize> {
    let tag = *data.get(pos)?;
    if tag != b'$' && tag != b'=' {
        return None;
    }
    let (len, after) = parse_int_at(data, pos + 1).ok()?;
    if len < 0 {
        return None;
    }
    check_bulk_len(len, max_bulk_length).ok()?;
    // payload + trailing CRLF
    Some(after + len as usize + 2)
}

/// Slices `data[range]`, answering [`Error::EOF`] instead of panicking when the
/// buffer stops short.
///
/// Every bound computed here is derived from a length the *server* sent, so
/// "past the end" is a routine streaming state — the decoder retries once more
/// bytes arrive — and never a reason to abort the connection. Reading through
/// this helper is what lets the module deny `clippy::indexing_slicing` outright.
#[inline(always)]
fn slice(data: &[u8], range: Range<usize>) -> Result<&[u8]> {
    data.get(range).ok_or_else(|| Error::EOF)
}

/// Finds the `\r` of the next `\r\n` at or after `from`, returning its index.
/// Errors with [`Error::EOF`] when no complete terminator is present yet.
#[inline]
fn find_crlf(data: &[u8], from: usize) -> Result<usize> {
    let rem = data.get(from..).ok_or_else(|| Error::EOF)?;
    let i = memchr(b'\r', rem).ok_or_else(|| Error::EOF)?;
    if rem.get(i + 1) != Some(&b'\n') {
        return Err(Error::EOF);
    }
    Ok(from + i)
}

/// Parses a RESP integer header starting at `from`, returning `(value, end)`
/// where `end` is the offset just past the terminating `\r\n`. Every announced
/// length on the wire goes through here — a bulk payload's, a collection's
/// cardinality, an attribute's pair count — so the one accumulation serves the
/// scalar layout and the frame parser alike.
#[inline]
pub(crate) fn parse_int_at(data: &[u8], from: usize) -> Result<(i64, usize)> {
    let digits = data.get(from..).ok_or_else(|| Error::EOF)?;
    let mut i = 0;

    let negative = if let Some(&b'-') = digits.first() {
        i += 1;
        true
    } else {
        false
    };

    // Accumulate the magnitude as a *negative* number so that `i64::MIN` — whose
    // positive magnitude is not representable — parses instead of overflowing.
    // A positive result is negated back at the end.
    let mut n = 0i64;
    while let Some(&digit) = digits.get(i) {
        match digit {
            b'0'..=b'9' => {
                n = n
                    .checked_mul(10)
                    .and_then(|n| n.checked_sub((digit - b'0') as i64))
                    .ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?;
                i += 1;
            }
            b'\r' => match digits.get(i + 1) {
                Some(&b'\n') => {
                    let value = if negative {
                        n
                    } else {
                        n.checked_neg()
                            .ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?
                    };
                    return Ok((value, from + i + 2));
                }
                Some(_) => return Err(Error::Client(ClientError::CannotParseInteger)),
                None => return Err(Error::EOF),
            },
            _ => return Err(Error::Client(ClientError::CannotParseInteger)),
        }
    }
    Err(Error::EOF)
}

/// Reads the [`ScalarLayout`] of the scalar element whose tag byte is at `at` in
/// `data`. `at` must point at a non-collection, non-attribute tag (the parser
/// dispatches collections and skips attributes before recording a node's offset),
/// so those tags are rejected as unknown.
///
/// The one tag table in the crate for scalar byte layout: the framing pass and
/// the read-back pass reach it through different projections, so they can never
/// disagree on where an element ends. The validation is exactly the parser's
/// original per-tag validation, so a frame the decoder accepted reads back
/// byte-identically.
///
/// Inlined, and so are its three projections, because this is the whole of the
/// scalar hot path on both sides: out of line, each side pays a call plus the
/// full tag dispatch, where inlining lets a call site keep only the arm its tag
/// selects, and lets the fields that site does not read be dropped rather than
/// built and spilled.
#[inline(always)]
fn scalar_layout(data: &[u8], at: usize, max_bulk_length: usize) -> Result<ScalarLayout> {
    let tag = *data.get(at).ok_or_else(|| Error::EOF)?;
    let start = at + 1;

    match tag {
        SIMPLE_STRING_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::SimpleString,
                value: start..cr,
                end: cr + 2,
            })
        }
        SIMPLE_ERROR_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Error,
                value: start..cr,
                end: cr + 2,
            })
        }
        INTEGER_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Integer,
                value: start..cr,
                end: cr + 2,
            })
        }
        DOUBLE_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Double,
                value: start..cr,
                end: cr + 2,
            })
        }
        // A big number is an arbitrary-precision integer surfaced as its
        // decimal-string payload so the caller can read it as a string.
        BIG_NUMBER_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::BulkString,
                value: start..cr,
                end: cr + 2,
            })
        }
        NULL_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Null,
                value: start..start,
                end: cr + 2,
            })
        }
        BOOL_TAG => {
            match slice(data, start..start + 3)? {
                [b't' | b'f', b'\r', b'\n'] => {}
                _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
            }
            Ok(ScalarLayout {
                kind: ScalarKind::Boolean,
                value: start..start + 1,
                end: start + 3,
            })
        }
        BULK_STRING_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            if len == -1 {
                return Ok(ScalarLayout {
                    kind: ScalarKind::Null,
                    value: after..after,
                    end: after,
                });
            }
            if len < 0 {
                return Err(Error::Client(ClientError::CannotParseBulkString));
            }
            check_bulk_len(len, max_bulk_length)?;
            let end = after + len as usize + 2;
            if slice(data, end - 2..end)? != b"\r\n" {
                return Err(Error::Client(ClientError::CannotParseBulkString));
            }
            Ok(ScalarLayout {
                kind: ScalarKind::BulkString,
                value: after..after + len as usize,
                end,
            })
        }
        // The first three bytes give the format (txt / mkd), the fourth is `:`,
        // then the real string follows — so the value skips the 4-byte prefix.
        VERBATIM_STRING_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            if len == -1 {
                return Ok(ScalarLayout {
                    kind: ScalarKind::Null,
                    value: after..after,
                    end: after,
                });
            }
            if len < 4 {
                return Err(Error::Client(ClientError::VerbatimStringTooShort));
            }
            check_bulk_len(len, max_bulk_length)?;
            let end = after + len as usize + 2;
            if slice(data, end - 2..end)? != b"\r\n" {
                return Err(Error::Client(ClientError::CannotParseVerbatimString));
            }
            Ok(ScalarLayout {
                kind: ScalarKind::BulkString,
                value: after + 4..after + len as usize,
                end,
            })
        }
        BULK_ERROR_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            if len < 0 {
                return Err(Error::Client(ClientError::CannotParseBulkError));
            }
            check_bulk_len(len, max_bulk_length)?;
            let end = after + len as usize + 2;
            if slice(data, end - 2..end)? != b"\r\n" {
                return Err(Error::Client(ClientError::CannotParseBulkError));
            }
            Ok(ScalarLayout {
                kind: ScalarKind::Error,
                value: after..after + len as usize,
                end,
            })
        }
        _ => Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
    }
}

/// Where the scalar at `at` ends: the offset just past its trailing `\r\n`, and
/// with it the proof that the element is well-formed. This is the framing
/// question, asked once per element by the forward pass.
///
/// `max_bulk_length` caps what a length header may announce, so a hostile length
/// is rejected here rather than turned into an offset.
#[inline(always)]
pub(crate) fn scalar_end(data: &[u8], at: usize, max_bulk_length: usize) -> Result<usize> {
    Ok(scalar_layout(data, at, max_bulk_length)?.end)
}

/// What the scalar at `at` is worth: its kind and the byte range of its payload,
/// for the reader to decode.
///
/// No bulk cap is applied, deliberately. These bytes were validated when the
/// frame was parsed, against the limits of the connection that received them; the
/// read-back happens in the calling task, which no longer knows those limits.
/// Re-checking against the *default* would wrongly reject a frame that a raised
/// [`RespLimits::max_bulk_length`](crate::client::RespLimits::max_bulk_length)
/// legitimately let through.
#[inline(always)]
pub(crate) fn scalar_value(data: &[u8], at: usize) -> Result<(ScalarKind, Range<usize>)> {
    let layout = scalar_layout(data, at, NO_BULK_LIMIT)?;
    Ok((layout.kind, layout.value))
}

/// Which bytes the scalar at `at` occupies — `at..end`, tag and terminator
/// included. The range to slice when one element of a collection has to be handed
/// out as a frame of its own.
///
/// No bulk cap is applied, for the reason given on [`scalar_value`].
#[inline(always)]
pub(crate) fn scalar_span(data: &[u8], at: usize) -> Result<Range<usize>> {
    Ok(at..scalar_layout(data, at, NO_BULK_LIMIT)?.end)
}

/// Kind and value range of a scalar whose bytes are `data` in their entirety.
///
/// The caller guarantees that: the tag is the first byte and the terminating
/// `\r\n` the last two, so nothing is scanned to find the end — the value stops
/// at `data.len() - 2`. Only the length-prefixed family reads its header, to
/// learn where its payload starts. The layout the framing pass already validated
/// is not re-checked.
#[inline]
pub(crate) fn frame_scalar_bounds(data: &[u8]) -> Result<(ScalarKind, Range<usize>)> {
    let tag = *data.first().ok_or_else(|| Error::EOF)?;
    let start = 1;
    let end = data.len().checked_sub(2).ok_or_else(|| Error::EOF)?;

    let kind = match tag {
        SIMPLE_STRING_TAG => ScalarKind::SimpleString,
        SIMPLE_ERROR_TAG => ScalarKind::Error,
        INTEGER_TAG => ScalarKind::Integer,
        DOUBLE_TAG => ScalarKind::Double,
        // A big number is surfaced as its decimal-string payload.
        BIG_NUMBER_TAG => ScalarKind::BulkString,
        NULL_TAG => return Ok((ScalarKind::Null, start..start)),
        BOOL_TAG => return Ok((ScalarKind::Boolean, start..start + 1)),
        BULK_STRING_TAG | VERBATIM_STRING_TAG | BULK_ERROR_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            // A nil bulk (`$-1\r\n`) has no payload at all.
            if len < 0 {
                return Ok((ScalarKind::Null, start..start));
            }
            let kind = if tag == BULK_ERROR_TAG {
                ScalarKind::Error
            } else {
                ScalarKind::BulkString
            };
            // A verbatim string spends its first four bytes on the format prefix.
            let value = if tag == VERBATIM_STRING_TAG {
                after + 4
            } else {
                after
            };
            return Ok((kind, value..end));
        }
        _ => return Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
    };
    Ok((kind, start..end))
}
