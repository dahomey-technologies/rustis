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
//!
//! # Offsets versus announced lengths
//!
//! Two kinds of arithmetic live here, and only one of them can overflow.
//!
//! Advancing an offset into `data` by a small constant — past a tag byte, past a
//! `\r\n`, past a digit — cannot: a slice's length is bounded by `isize::MAX`, so
//! an in-bounds offset plus a constant stays well inside `usize`. Those sites
//! exempt `clippy::arithmetic_side_effects` and name that bound.
//!
//! Adding a length the *server* announced can, and none of it happens in the open:
//! it goes through [`check_bulk_len`] and [`bulk_payload`], which compare before
//! narrowing and add with `checked_add`.

use crate::{
    ClientError, Error, Result,
    resp::{
        BIG_NUMBER_TAG, BOOL_TAG, BULK_ERROR_TAG, BULK_STRING_TAG, DOUBLE_TAG, INTEGER_TAG,
        NULL_TAG, SIMPLE_ERROR_TAG, SIMPLE_STRING_TAG, VERBATIM_STRING_TAG,
    },
};
use memchr::memchr;
use std::ops::Range;

/// A `max_bulk_length` that caps nothing. See [`scalar_value`] for why the
/// read-back path passes it.
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
/// its own question — [`scalar_end`], [`scalar_value`], [`scalar_span`] or
/// [`frame_scalar_value`] — so no call site carries a field it does not read.
struct ScalarLayout {
    kind: ScalarKind,
    value: Range<usize>,
    end: usize,
}

/// Rejects a bulk-family length that exceeds `max_bulk_length`. `len` must
/// already be known non-negative.
///
/// The comparison widens instead of narrowing: narrowing `len` to a `usize`
/// truncates on a 32-bit target, so an announced length above `u32::MAX` would
/// slip under the cap as a small number and the payload bounds would then be
/// computed from it. Widening a non-negative `i64` to `u64` is lossless, and so
/// is `usize` to `u64` at every pointer width.
///
/// Passing this is what makes the callers' narrowing to `usize` infallible: the
/// length is bounded by a `usize` afterwards.
#[inline]
fn check_bulk_len(len: i64, max_bulk_length: usize) -> Result<()> {
    if len.cast_unsigned() > max_bulk_length as u64 {
        return Err(Error::Client(ClientError::BulkLengthTooLarge));
    }
    Ok(())
}

/// Where the payload of a length-prefixed scalar sits, and where the whole
/// element ends.
///
/// The `$`, `=` and `!` tags share one byte layout — `len` payload bytes at
/// `after`, then `\r\n` — so they share the arithmetic that turns an announced
/// length into offsets, and the terminator check that proves the element
/// well-formed. `skip` drops a fixed prefix from the reported payload, which only
/// the verbatim tag uses for its `txt:` / `mkd:` marker; the caller has already
/// established `skip <= len`. `malformed` is the tag's own error for a payload
/// that is not `\r\n`-terminated.
///
/// `len` must be non-negative. The payload end is computed with `checked_add`:
/// `max_bulk_length` is a connection setting, and the read-back path deliberately
/// passes [`NO_BULK_LIMIT`], so no cap here bounds the sum on its own.
#[inline(always)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the two sums a server-announced length drives are `checked_add`; the \
              remaining one steps over a fixed prefix the caller has already \
              bounded by that length."
)]
fn bulk_payload(
    data: &[u8],
    after: usize,
    len: i64,
    skip: usize,
    max_bulk_length: usize,
    malformed: ClientError,
) -> Result<(Range<usize>, usize)> {
    check_bulk_len(len, max_bulk_length)?;
    let len = usize::try_from(len).map_err(|_| Error::Client(malformed.clone()))?;
    let payload_end = after
        .checked_add(len)
        .ok_or_else(|| Error::Client(malformed.clone()))?;
    let end = payload_end
        .checked_add(2)
        .ok_or_else(|| Error::Client(malformed.clone()))?;
    if slice(data, payload_end..end)? != b"\r\n" {
        return Err(Error::Client(malformed));
    }
    // The caller established `skip <= len`, so this lands at or before
    // `payload_end`, whose own addition just succeeded.
    Ok((after + skip..payload_end, end))
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
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`pos` indexes `data` — the read on the line below proves it — so \
              stepping past the tag byte stays inside `usize`. The announced \
              length is added under `checked_add` further down."
)]
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
    // payload + trailing CRLF. `max_bulk_length` is a connection setting, so the
    // cap above bounds `len` but says nothing about the sum; answering `None` here
    // just sends the caller back to the doubling fallback.
    after
        .checked_add(usize::try_from(len).ok()?)?
        .checked_add(2)
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
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`i` is a `memchr` hit inside `rem`, itself a suffix of `data`, so \
              both `i + 1` and `from + i` are offsets into a slice — bounded by \
              `isize::MAX`."
)]
fn find_crlf(data: &[u8], from: usize) -> Result<usize> {
    let rem = data.get(from..).ok_or_else(|| Error::EOF)?;
    let i = memchr(b'\r', rem).ok_or_else(|| Error::EOF)?;
    if rem.get(i + 1) != Some(&b'\n') {
        return Err(Error::EOF);
    }
    Ok(from + i)
}

/// Where the `\r` terminating a line-shaped scalar's payload sits, when the
/// payload starts at `from`.
///
/// `FRAME` says the scalar is a whole frame's content, so `data` stops right after
/// its terminator and the position is arithmetic instead of a search. That is the
/// caller's invariant, not something the bytes prove, so debug builds check it
/// against the search — exactly, because `data.ends_with(b"\r\n")` would accept
/// `+OK\r\n\r\n`, which is two frames.
///
/// The search costs a `memchr` over a payload that is often two or three bytes
/// (`OK`, `12`), and on the read-back of a lone scalar it is a second one after
/// the framing pass already did it. Measured on `+OK\r\n`, that is 4.6 ns of the
/// 61 ns a caller spends parsing and deserializing the reply, and 4.9 of 49 on
/// `:1000\r\n` — the reply shape of every `SET` and every `INCR`.
#[inline(always)]
fn crlf_at<const FRAME: bool>(data: &[u8], from: usize) -> Result<usize> {
    if !FRAME {
        return find_crlf(data, from);
    }
    let cr = data.len().checked_sub(2).ok_or_else(|| Error::EOF)?;
    if cr < from {
        return Err(Error::EOF);
    }
    debug_assert_eq!(
        Some(cr),
        find_crlf(data, from).ok(),
        "a frame's own bytes must end at its scalar's terminator"
    );
    Ok(cr)
}

/// Parses a RESP integer header starting at `from`, returning `(value, end)`
/// where `end` is the offset just past the terminating `\r\n`. Every announced
/// length on the wire goes through here — a bulk payload's, a collection's
/// cardinality, an attribute's pair count — so the one accumulation serves the
/// scalar layout and the frame parser alike.
#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`i` only advances over bytes `digits.get(i)` returned, so it stays \
              an offset into a slice and `from + i + 2` cannot leave `usize`. \
              `digit - b'0'` is inside the `b'0'..=b'9'` arm. The accumulation \
              itself — the one operation here that a hostile length drives — is \
              already `checked_mul` / `checked_sub`."
)]
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
                    .and_then(|n| n.checked_sub(i64::from(digit - b'0')))
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
/// disagree on where an element ends or what it validates. Every malformation the
/// parser rejects is rejected here too, on both sides.
///
/// `FRAME` is the one thing the two sides do not share: it says `data` holds
/// nothing but this scalar, which lets a line-shaped payload's terminator be
/// computed rather than searched for. See [`crlf_at`]. It is `true` only under
/// [`frame_scalar_value`], and only with `at` at 0.
///
/// Inlined, and so are its four projections, because this is the whole of the
/// scalar hot path on both sides: out of line, each side pays a call plus the
/// full tag dispatch, where inlining lets a call site keep only the arm its tag
/// selects, and lets the fields that site does not read be dropped rather than
/// built and spilled.
#[inline(always)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every sum here steps over a fixed number of bytes from an offset \
              already known to index `data`: `at` is read on the first line, and \
              `cr` is where a terminator was found. The three length-prefixed tags \
              add their announced length inside `bulk_payload` instead."
)]
fn scalar_layout<const FRAME: bool>(
    data: &[u8],
    at: usize,
    max_bulk_length: usize,
) -> Result<ScalarLayout> {
    let tag = *data.get(at).ok_or_else(|| Error::EOF)?;
    let start = at + 1;

    match tag {
        SIMPLE_STRING_TAG => {
            let cr = crlf_at::<FRAME>(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::SimpleString,
                value: start..cr,
                end: cr + 2,
            })
        }
        SIMPLE_ERROR_TAG => {
            let cr = crlf_at::<FRAME>(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Error,
                value: start..cr,
                end: cr + 2,
            })
        }
        INTEGER_TAG => {
            let cr = crlf_at::<FRAME>(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Integer,
                value: start..cr,
                end: cr + 2,
            })
        }
        DOUBLE_TAG => {
            let cr = crlf_at::<FRAME>(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::Double,
                value: start..cr,
                end: cr + 2,
            })
        }
        // A big number is an arbitrary-precision integer surfaced as its
        // decimal-string payload so the caller can read it as a string.
        BIG_NUMBER_TAG => {
            let cr = crlf_at::<FRAME>(data, start)?;
            Ok(ScalarLayout {
                kind: ScalarKind::BulkString,
                value: start..cr,
                end: cr + 2,
            })
        }
        NULL_TAG => {
            let cr = crlf_at::<FRAME>(data, start)?;
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
            let (value, end) = bulk_payload(
                data,
                after,
                len,
                0,
                max_bulk_length,
                ClientError::CannotParseBulkString,
            )?;
            Ok(ScalarLayout {
                kind: ScalarKind::BulkString,
                value,
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
            let (value, end) = bulk_payload(
                data,
                after,
                len,
                4,
                max_bulk_length,
                ClientError::CannotParseVerbatimString,
            )?;
            Ok(ScalarLayout {
                kind: ScalarKind::BulkString,
                value,
                end,
            })
        }
        BULK_ERROR_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            if len < 0 {
                return Err(Error::Client(ClientError::CannotParseBulkError));
            }
            let (value, end) = bulk_payload(
                data,
                after,
                len,
                0,
                max_bulk_length,
                ClientError::CannotParseBulkError,
            )?;
            Ok(ScalarLayout {
                kind: ScalarKind::Error,
                value,
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
    Ok(scalar_layout::<false>(data, at, max_bulk_length)?.end)
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
    let layout = scalar_layout::<false>(data, at, NO_BULK_LIMIT)?;
    Ok((layout.kind, layout.value))
}

/// Which bytes the scalar at `at` occupies — `at..end`, tag and terminator
/// included. The range to slice when one element of a collection has to be handed
/// out as a frame of its own.
///
/// No bulk cap is applied, for the reason given on [`scalar_value`].
#[inline(always)]
pub(crate) fn scalar_span(data: &[u8], at: usize) -> Result<Range<usize>> {
    Ok(at..scalar_layout::<false>(data, at, NO_BULK_LIMIT)?.end)
}

/// What a lone scalar is worth, when `data` is that scalar's own bytes and nothing
/// else — the shape a tapeless frame holds. Same answer as [`scalar_value`] at
/// offset 0, for less work: the caller's bytes already say where the scalar ends.
///
/// The invariant is the caller's to keep, so debug builds check both halves of it:
/// [`crlf_at`] checks that a line-shaped payload really stops at the last two
/// bytes, and the assertion below that no other shape leaves bytes over.
///
/// No bulk cap is applied, for the reason given on [`scalar_value`].
#[inline(always)]
pub(crate) fn frame_scalar_value(data: &[u8]) -> Result<(ScalarKind, Range<usize>)> {
    let layout = scalar_layout::<true>(data, 0, NO_BULK_LIMIT)?;
    debug_assert_eq!(
        layout.end,
        data.len(),
        "a frame's own bytes must hold nothing but its scalar"
    );
    Ok((layout.kind, layout.value))
}
