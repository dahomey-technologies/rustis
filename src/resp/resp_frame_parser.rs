use crate::{
    ClientError, Error, Result,
    client::RespLimits,
    resp::{RespTape, RespTapeMut, TAPE_LEN_TAG, is_container_tag},
};
use memchr::memchr;
use std::{fmt, ops::Range};

pub(crate) const SIMPLE_STRING_TAG: u8 = b'+';
pub(crate) const SIMPLE_ERROR_TAG: u8 = b'-';
pub(crate) const INTEGER_TAG: u8 = b':';
pub(crate) const BULK_STRING_TAG: u8 = b'$';
pub(crate) const ARRAY_TAG: u8 = b'*';
pub(crate) const NULL_TAG: u8 = b'_';
pub(crate) const BOOL_TAG: u8 = b'#';
pub(crate) const DOUBLE_TAG: u8 = b',';
pub(crate) const BULK_ERROR_TAG: u8 = b'!';
pub(crate) const VERBATIM_STRING_TAG: u8 = b'=';
pub(crate) const MAP_TAG: u8 = b'%';
pub(crate) const SET_TAG: u8 = b'~';
pub(crate) const PUSH_TAG: u8 = b'>';
pub(crate) const ATTRIBUTE_TAG: u8 = b'|';
pub(crate) const BIG_NUMBER_TAG: u8 = b'(';

/// Sentinel meaning "this data was already validated when it was parsed, do not
/// re-apply a cap". Used by the tape read-back path, which walks a frame the
/// decoder has already accepted: re-checking it against the *current* default
/// would wrongly reject a frame that a raised
/// [`RespLimits::max_bulk_length`] legitimately let through.
pub(crate) const NO_BULK_LIMIT: usize = usize::MAX;

/// What one forward pass recovers from a frame's bytes.
///
/// The parser only *frames*: it finds where the frame ends so the buffer can be
/// sliced, and indexes a container's elements. It decodes no value — the tag
/// alone says how to read one, and the read happens in the calling task rather
/// than in the shared network task.
pub enum ParsedFrame {
    /// A single scalar, whose tag byte sits at `at` in the frame. It carries no
    /// tape: one node for one value would buy nothing, and keeping the hot
    /// request/response path node-free keeps the recycled tape buffer untouched.
    Scalar { at: usize },
    /// A container, with the tape indexing it and all of its descendants, rooted
    /// at node 0.
    Collection(RespTape),
    /// A null collection (`*-1\r\n`): a container tag with no container. Its
    /// bytes hold nothing to read back, so they are dropped.
    Null,
}

/// Reports the shape, never the tape — an internal index whose raw bytes are
/// unreadable and routinely larger than the reply itself. Format the enclosing
/// [`RespResponse`](crate::resp::RespResponse) to see the decoded reply.
impl fmt::Debug for ParsedFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar { at } => f.debug_struct("Scalar").field("at", at).finish(),
            Self::Collection(tape) => f
                .debug_struct("Collection")
                .field("nodes", &tape.node_count())
                .finish(),
            Self::Null => f.write_str("Null"),
        }
    }
}

/// The normalized kind of a scalar element, as recovered from its bytes by
/// [`element_bounds`]. This is what the tape reader dispatches on, independent
/// of the exact RESP tag (a big number and a bulk string both read back as
/// [`ElementKind::BulkString`], for instance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementKind {
    SimpleString,
    Error,
    Integer,
    Double,
    BulkString,
    Boolean,
    Null,
}

/// The layout of one scalar element within the data buffer: its kind, the byte
/// range of its value payload, and the offset one past the whole element.
///
/// This is the **single source of truth** for per-element byte layout, shared
/// by the write side (the parser's forward pass, which needs only `end` to
/// advance) and the read side (the tape reader, which needs `kind` + `value` to
/// build a view). Keeping one function means the two passes can never disagree
/// on where an element ends.
pub(crate) struct ElementBounds {
    pub kind: ElementKind,
    pub value: Range<usize>,
    pub end: usize,
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
/// Every bound in this parser is derived from a length the *server* sent, so
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
/// where `end` is the offset just past the terminating `\r\n`. Mirrors
/// the same accumulation the collection and bulk headers rely on, over a free
/// slice rather than a parser instance.
#[inline]
fn parse_int_at(data: &[u8], from: usize) -> Result<(i64, usize)> {
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

/// Computes the [`ElementBounds`] of the scalar element whose tag byte is at
/// `off` in `data`. `off` must point at a non-container, non-attribute tag (the
/// parser dispatches containers and skips attributes before recording a node's
/// offset), so those tags are rejected as unknown.
///
/// The validation here is exactly the parser's original per-tag validation, so
/// a frame the decoder accepted reads back byte-identically.
///
/// Inlined because it is the whole of the scalar hot path, on both the framing
/// and the reading side: out of line, each side pays a call plus the full tag
/// dispatch, where inlining lets the caller keep only the arm its tag selects.
/// The framing side reads only `end`, so inlining is also what lets the rest of
/// the struct be dropped there rather than built and spilled.
#[inline(always)]
pub(crate) fn element_bounds(
    data: &[u8],
    off: usize,
    max_bulk_length: usize,
) -> Result<ElementBounds> {
    let tag = *data.get(off).ok_or_else(|| Error::EOF)?;
    let start = off + 1;

    match tag {
        SIMPLE_STRING_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ElementBounds {
                kind: ElementKind::SimpleString,
                value: start..cr,
                end: cr + 2,
            })
        }
        SIMPLE_ERROR_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ElementBounds {
                kind: ElementKind::Error,
                value: start..cr,
                end: cr + 2,
            })
        }
        INTEGER_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ElementBounds {
                kind: ElementKind::Integer,
                value: start..cr,
                end: cr + 2,
            })
        }
        DOUBLE_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ElementBounds {
                kind: ElementKind::Double,
                value: start..cr,
                end: cr + 2,
            })
        }
        // A big number is an arbitrary-precision integer surfaced as its
        // decimal-string payload so the caller can read it as a string.
        BIG_NUMBER_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ElementBounds {
                kind: ElementKind::BulkString,
                value: start..cr,
                end: cr + 2,
            })
        }
        NULL_TAG => {
            let cr = find_crlf(data, start)?;
            Ok(ElementBounds {
                kind: ElementKind::Null,
                value: start..start,
                end: cr + 2,
            })
        }
        BOOL_TAG => {
            match slice(data, start..start + 3)? {
                [b't' | b'f', b'\r', b'\n'] => {}
                _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
            }
            Ok(ElementBounds {
                kind: ElementKind::Boolean,
                value: start..start + 1,
                end: start + 3,
            })
        }
        BULK_STRING_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            if len == -1 {
                return Ok(ElementBounds {
                    kind: ElementKind::Null,
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
            Ok(ElementBounds {
                kind: ElementKind::BulkString,
                value: after..after + len as usize,
                end,
            })
        }
        // The first three bytes give the format (txt / mkd), the fourth is `:`,
        // then the real string follows — so the value skips the 4-byte prefix.
        VERBATIM_STRING_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            if len == -1 {
                return Ok(ElementBounds {
                    kind: ElementKind::Null,
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
            Ok(ElementBounds {
                kind: ElementKind::BulkString,
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
            Ok(ElementBounds {
                kind: ElementKind::Error,
                value: after..after + len as usize,
                end,
            })
        }
        _ => Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
    }
}

/// Kind and value range of a scalar whose bytes are `data` in their entirety.
///
/// The caller guarantees that: the tag is the first byte and the terminating
/// `\r\n` the last two, so nothing is scanned to find the end — the value stops
/// at `data.len() - 2`. Only the length-prefixed family reads its header, to
/// learn where its payload starts. The layout the framing pass already validated
/// is not re-checked.
#[inline]
pub(crate) fn frame_scalar_bounds(data: &[u8]) -> Result<(ElementKind, Range<usize>)> {
    let tag = *data.first().ok_or_else(|| Error::EOF)?;
    let start = 1;
    let end = data.len().checked_sub(2).ok_or_else(|| Error::EOF)?;

    let kind = match tag {
        SIMPLE_STRING_TAG => ElementKind::SimpleString,
        SIMPLE_ERROR_TAG => ElementKind::Error,
        INTEGER_TAG => ElementKind::Integer,
        DOUBLE_TAG => ElementKind::Double,
        // A big number is surfaced as its decimal-string payload.
        BIG_NUMBER_TAG => ElementKind::BulkString,
        NULL_TAG => return Ok((ElementKind::Null, start..start)),
        BOOL_TAG => return Ok((ElementKind::Boolean, start..start + 1)),
        BULK_STRING_TAG | VERBATIM_STRING_TAG | BULK_ERROR_TAG => {
            let (len, after) = parse_int_at(data, start)?;
            // A nil bulk (`$-1\r\n`) has no payload at all.
            if len < 0 {
                return Ok((ElementKind::Null, start..start));
            }
            let kind = if tag == BULK_ERROR_TAG {
                ElementKind::Error
            } else {
                ElementKind::BulkString
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

/// Rejects a collection cardinality that exceeds `max_collection_length`,
/// bounding an attacker-controlled loop count.
#[inline]
fn check_collection_len(len: usize, max_collection_length: usize) -> Result<()> {
    if len > max_collection_length {
        return Err(Error::Client(ClientError::CollectionLengthTooLarge));
    }
    Ok(())
}

/// Outcome of reading a collection header (`*<n>\r\n`, `%<n>\r\n`, `~`, `>`).
enum ContainerHeader {
    /// A null collection (`*-1\r\n`): it has no children and deserializes to
    /// `Null`, but is still counted as one element by its parent.
    Null { end: usize },
    /// A present collection with `count` children to follow (already doubled for
    /// maps), whose first child begins at `end`.
    Open { count: usize, end: usize },
}

/// Reads the header of the collection whose tag byte is at `at`, returning its
/// child count (doubled for maps) and the offset just past the `\r\n`. `at` must
/// point at a container tag. [`Error::EOF`] when the header has not fully
/// arrived, so the streaming decoder can retry once more bytes are read.
fn parse_container_header(data: &[u8], at: usize) -> Result<ContainerHeader> {
    let tag = *data.get(at).ok_or_else(|| Error::EOF)?;
    let (n, end) = parse_int_at(data, at + 1)?;
    if n == -1 {
        return Ok(ContainerHeader::Null { end });
    }
    if n < 0 {
        return Err(Error::Client(if tag == MAP_TAG {
            ClientError::CannotParseMap
        } else {
            ClientError::CannotParseSequence
        }));
    }
    let multiplier = if tag == MAP_TAG { 2 } else { 1 };
    Ok(ContainerHeader::Open {
        count: n as usize * multiplier,
        end,
    })
}

/// Advances past zero or more consecutive RESP3 attribute frames (`|<n>\r\n`
/// followed by `2n` values) starting at `pos`, returning the offset of the first
/// non-attribute byte. Attributes are out-of-band metadata that may legally
/// precede any value, so they are skipped wherever a value is expected and never
/// surfaced — neither as a frame nor as a tape node. [`Error::EOF`] if an
/// attribute is only partially present; the caller then rewinds and retries.
fn skip_leading_attributes(
    data: &[u8],
    mut pos: usize,
    depth: usize,
    limits: &RespLimits,
) -> Result<usize> {
    while data.get(pos) == Some(&ATTRIBUTE_TAG) {
        if depth + 1 > limits.max_nesting_depth {
            return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
        }
        let (n, after) = parse_int_at(data, pos + 1)?;
        if n < 0 {
            return Err(Error::Client(ClientError::CannotParseMap));
        }
        let count = n as usize * 2;
        check_collection_len(count, limits.max_collection_length)?;
        let mut child = after;
        for _ in 0..count {
            child = skip_one_value(data, child, depth + 1, limits)?;
        }
        pos = child;
    }
    Ok(pos)
}

/// Advances past exactly one value at `pos` — a scalar, or a nested collection
/// with all of its descendants — returning the offset just past it. Used only to
/// consume attribute payloads, which carry no tape, so it walks the structure
/// without recording anything. Recursion is bounded by `limits.max_nesting_depth`.
/// [`Error::EOF`] if the value is incomplete.
fn skip_one_value(data: &[u8], pos: usize, depth: usize, limits: &RespLimits) -> Result<usize> {
    let pos = skip_leading_attributes(data, pos, depth, limits)?;
    let tag = *data.get(pos).ok_or_else(|| Error::EOF)?;
    if is_container_tag(tag) {
        match parse_container_header(data, pos)? {
            ContainerHeader::Null { end } => Ok(end),
            ContainerHeader::Open { count, end } => {
                check_collection_len(count, limits.max_collection_length)?;
                if depth + 1 > limits.max_nesting_depth {
                    return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
                }
                let mut child = end;
                for _ in 0..count {
                    child = skip_one_value(data, child, depth + 1, limits)?;
                }
                Ok(child)
            }
        }
    } else {
        Ok(element_bounds(data, pos, limits.max_bulk_length)?.end)
    }
}

/// One collection still being filled on the resumable parse stack: its tag (to
/// rebuild the frame and back-patch its head node's `next`), the tape index of
/// that head node, and how many of its children are still unparsed.
///
/// The stack of these is owned by the caller (a local for one-shot parsing, the
/// streaming decoder's reused buffer for the network path) and threaded through
/// the element loop, so the parser itself stays a lightweight cursor with no
/// heap field on the hot scalar path.
pub(crate) struct PendingContainer {
    tag: u8,
    head_index: usize,
    remaining: usize,
}

/// Streaming RESP parser — a lightweight cursor over a byte slice plus the tape
/// builder it writes into. One forward pass produces a [`ParsedFrame`]: the
/// offset of a top-level scalar, or, for a collection, a flat tape of
/// fixed-width nodes written into the borrowed `tape` buffer (see
/// [`crate::resp::resp_tape`]).
///
/// The collection pass is an **iterative state machine** over an explicit stack
/// (owned by the caller, see [`PendingContainer`]), not recursion: each step
/// consumes exactly one unit (an element, a collection header, or a run of
/// attributes) and is atomic with respect to `pos` — it either advances past its
/// whole unit or, on [`Error::EOF`], leaves `pos` at the unit's start. That
/// atomicity is what lets a partially-received frame be suspended and resumed
/// byte-for-byte across TCP chunks, and the explicit stack keeps a crafted
/// deeply-nested reply from overflowing the call stack.
pub struct RespFrameParser<'a, 'b> {
    buf: &'a [u8],
    /// Hostile-input bounds this parser enforces, resolved from the connection's
    /// [`Config`](crate::client::Config) so a frame is checked against the same
    /// limits wherever it is parsed.
    limits: RespLimits,
    /// Tape builder, borrowed so the decoder can recycle one across frames
    /// ([`RespTapeMut::split_freeze`] per frame keeps its capacity). While a frame
    /// is incomplete the partial tape stays here, accumulating across chunks.
    tape: &'b mut RespTapeMut,
    pos: usize,
}

impl<'a, 'b> RespFrameParser<'a, 'b> {
    /// A parser positioned at the start of `buf`. Used both for one-shot parsing
    /// of a complete buffer and as the streaming decoder's entry point for a
    /// brand-new frame.
    pub fn new(buf: &'a [u8], tape: &'b mut RespTapeMut) -> Self {
        Self::with_limits(buf, tape, RespLimits::DEFAULT)
    }

    /// A parser positioned at the start of `buf`, enforcing caller-chosen
    /// limits instead of the defaults.
    pub fn with_limits(buf: &'a [u8], tape: &'b mut RespTapeMut, limits: RespLimits) -> Self {
        Self {
            buf,
            limits,
            tape,
            pos: 0,
        }
    }

    /// A parser positioned at `pos`, used by the streaming decoder to resume a
    /// frame it previously suspended. The partial tape is expected to already be
    /// present in `tape`, and the open-collection stack is passed to
    /// [`Self::parse_resumable`].
    pub(crate) fn at(
        buf: &'a [u8],
        tape: &'b mut RespTapeMut,
        pos: usize,
        limits: RespLimits,
    ) -> Self {
        Self {
            buf,
            limits,
            tape,
            pos,
        }
    }

    /// The byte offset the parser has reached — the frame length once a frame is
    /// complete, or the resume point when it is suspended.
    #[inline(always)]
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// One-shot parse of a single frame from a complete buffer. Returns the frame
    /// and its byte length, or [`Error::EOF`] if the buffer stops mid-frame — the
    /// one-shot callers treat a truncated buffer as an error, not a suspension.
    ///
    /// The scalar branch never names a `Vec`: the collection stack is created only
    /// inside the container branch, so a scalar reply — the hot request/response
    /// path — stays flat and allocation-free. The skeleton mirrors
    /// [`Self::parse_resumable`]; both defer the actual work to
    /// `skip_leading_attributes`, [`element_bounds`] and `begin_collection`, so
    /// the two cannot diverge on where a value ends.
    #[inline(always)]
    pub fn parse(&mut self) -> Result<(ParsedFrame, usize)> {
        if self.buf.get(self.pos) == Some(&ATTRIBUTE_TAG) {
            self.pos = skip_leading_attributes(self.buf, self.pos, 0, &self.limits)?;
        }
        let tag = *self.buf.get(self.pos).ok_or_else(|| Error::EOF)?;
        if is_container_tag(tag) {
            let mut stack = Vec::new();
            return match self.begin_collection(tag, &mut stack)? {
                Some(frame) => Ok((frame, self.pos)),
                None => Err(Error::EOF),
            };
        }
        let at = self.pos;
        self.pos = element_bounds(self.buf, at, self.limits.max_bulk_length)?.end;
        Ok((ParsedFrame::Scalar { at }, self.pos))
    }

    /// Resumable parse driven by the streaming decoder, over a caller-owned
    /// `stack`: `Ok(Some(frame))` = a whole frame is ready; `Ok(None)` = more
    /// bytes are needed (the decoder keeps `stack`, the partial tape, and
    /// [`Self::pos`] to resume); `Err` = a malformed frame. A suspended frame
    /// (non-empty `stack`) re-enters its element loop directly; a fresh one takes
    /// the same scalar-inline / collection-delegate path as [`Self::parse`], but
    /// rewinds `pos` on a partial value so the next chunk re-attempts it.
    pub(crate) fn parse_resumable(
        &mut self,
        stack: &mut Vec<PendingContainer>,
    ) -> Result<Option<ParsedFrame>> {
        if !stack.is_empty() {
            return self.run_collection_loop(stack);
        }

        let frame_start = self.pos;
        // Leading attributes are rare out-of-band metadata; peek before calling so
        // the common scalar path pays nothing. A partial attribute rewinds to the
        // frame start; a complete run is consumed and stays in the buffer.
        if self.buf.get(self.pos) == Some(&ATTRIBUTE_TAG) {
            match skip_leading_attributes(self.buf, self.pos, 0, &self.limits) {
                Ok(at) => self.pos = at,
                Err(Error::EOF) => {
                    self.pos = frame_start;
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }
        }

        // `pos` now sits at the value's tag; resuming from here re-reads it.
        let value_pos = self.pos;
        let Some(&tag) = self.buf.get(value_pos) else {
            return Ok(None);
        };
        if is_container_tag(tag) {
            return self.begin_collection(tag, stack);
        }
        match element_bounds(self.buf, value_pos, self.limits.max_bulk_length) {
            Ok(bounds) => {
                self.pos = bounds.end;
                Ok(Some(ParsedFrame::Scalar { at: value_pos }))
            }
            Err(Error::EOF) => {
                self.pos = value_pos;
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Opens the collection whose header is at `self.pos` and runs the element
    /// loop to build its tape. The out-of-line, cold counterpart to the scalar
    /// path in [`Self::parse`] / [`Self::parse_resumable`]. On a partial header it
    /// rewinds `pos` to the container tag (any leading attributes are already
    /// consumed and remain buffered), so a later chunk re-reads the header.
    fn begin_collection(
        &mut self,
        tag: u8,
        stack: &mut Vec<PendingContainer>,
    ) -> Result<Option<ParsedFrame>> {
        let at = self.pos;
        match parse_container_header(self.buf, at) {
            Ok(ContainerHeader::Null { end }) => {
                self.pos = end;
                Ok(Some(ParsedFrame::Null))
            }
            Ok(ContainerHeader::Open { count, end }) => {
                check_collection_len(count, self.limits.max_collection_length)?;
                debug_assert!(self.tape.is_empty(), "tape must start empty per frame");
                let head = self.tape.push(tag, 0);
                self.tape.push(TAPE_LEN_TAG, count as u64);
                self.pos = end;
                stack.push(PendingContainer {
                    tag,
                    head_index: head,
                    remaining: count,
                });
                self.run_collection_loop(stack)
            }
            Err(Error::EOF) => {
                self.pos = at;
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Drives the explicit-stack element loop until the root collection is
    /// complete (`Ok(Some)`) or a child needs more bytes (`Ok(None)`, resumable).
    fn run_collection_loop(
        &mut self,
        stack: &mut Vec<PendingContainer>,
    ) -> Result<Option<ParsedFrame>> {
        // The loop is entered with at least one open collection and returns the
        // moment the root closes, so neither the empty stack nor the non-container
        // tag below is reachable. Both are written as a malformed-frame error
        // rather than an assertion: this runs on the network task, where a panic
        // takes the client down with every in-flight command and no reconnect,
        // while an error fails just this frame.
        while let Some(remaining) = stack.last().map(|open| open.remaining) {
            if remaining == 0 {
                // Every child of this collection is written. Back-patch its head's
                // `next` to the tape end (the reader's O(1) sibling skip) and close
                // the level, crediting the parent — or finish the frame at the root.
                let Some(done) = stack.pop() else { break };
                let next = self.tape.node_count() as u64;
                self.tape.patch(done.head_index, done.tag, next);
                if let Some(parent) = stack.last_mut() {
                    parent.remaining -= 1;
                    continue;
                }
                if !is_container_tag(done.tag) {
                    return Err(Error::Client(ClientError::Unexpected));
                }
                return Ok(Some(ParsedFrame::Collection(self.tape.split_freeze())));
            }

            // Parse one child; on EOF rewind to its start so the resumed parse
            // re-attempts exactly this child, with the tape and stack intact.
            let child_start = self.pos;
            match self.emit_one_child(stack) {
                Ok(()) => {}
                Err(Error::EOF) => {
                    self.pos = child_start;
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }
        }

        Err(Error::Client(ClientError::Unexpected))
    }

    /// Emits the tape node(s) for the value at `self.pos` and advances past it.
    /// A scalar (or null collection) emits one node and credits the current
    /// collection; a nested collection pushes a new stack level whose completion
    /// later credits this one. Writes nothing on [`Error::EOF`], so the caller can
    /// rewind and resume.
    #[inline]
    fn emit_one_child(&mut self, stack: &mut Vec<PendingContainer>) -> Result<()> {
        // Elements rarely carry a leading attribute; peek before paying for the
        // (non-inlinable, recursive) skip call, so the common case is one compare.
        let mut at = self.pos;
        if self.buf.get(at) == Some(&ATTRIBUTE_TAG) {
            at = skip_leading_attributes(self.buf, at, stack.len(), &self.limits)?;
        }
        let tag = *self.buf.get(at).ok_or_else(|| Error::EOF)?;

        if is_container_tag(tag) {
            match parse_container_header(self.buf, at)? {
                ContainerHeader::Null { end } => {
                    // A null child collection is one counted element that reads
                    // back as `Null`; its stored offset is never dereferenced.
                    self.tape.push(NULL_TAG, at as u64);
                    self.pos = end;
                    credit_open_collection(stack);
                }
                ContainerHeader::Open { count, end } => {
                    check_collection_len(count, self.limits.max_collection_length)?;
                    if stack.len() + 1 > self.limits.max_nesting_depth {
                        return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
                    }
                    let head = self.tape.push(tag, 0);
                    self.tape.push(TAPE_LEN_TAG, count as u64);
                    self.pos = end;
                    stack.push(PendingContainer {
                        tag,
                        head_index: head,
                        remaining: count,
                    });
                    // The parent is credited when this child collection closes.
                }
            }
        } else {
            let bounds = element_bounds(self.buf, at, self.limits.max_bulk_length)?;
            self.tape.push(tag, at as u64);
            self.pos = bounds.end;
            credit_open_collection(stack);
        }
        Ok(())
    }
}

/// Records that one child of the innermost open collection is fully parsed.
///
/// A no-op on an empty stack, which the collection loop never produces: it is
/// the only caller of `emit_one_child` and always holds at least one open level.
#[inline]
fn credit_open_collection(stack: &mut [PendingContainer]) {
    if let Some(open) = stack.last_mut() {
        open.remaining -= 1;
    }
}
