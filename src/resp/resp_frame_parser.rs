use crate::{
    ClientError, Error, Result,
    resp::{RespFrame, TAPE_LEN_TAG, is_container_tag, node_count, patch_node, push_node},
};
use bytes::BytesMut;
use memchr::memchr;
use std::ops::Range;

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

/// Maximum collection-nesting depth the parser will descend into before
/// rejecting a frame. RESP replies are shallow in practice (a handful of levels
/// for the deepest cluster/stream introspection commands), so this bound is
/// generous for legitimate traffic while stopping a crafted `*1\r\n*1\r\n…`
/// reply from driving the parser into a stack overflow, which — unlike a panic —
/// is not catchable and aborts the whole process. The element loop is iterative,
/// so this bounds the explicit stack (and the recursion left in attribute
/// skipping) rather than the call stack.
pub(crate) const MAX_NESTING_DEPTH: usize = 128;

/// Maximum byte length the parser accepts for a single bulk string, bulk error
/// or verbatim string, checked against the declared header before the payload
/// is trusted. Matches Redis's default `proto-max-bulk-len` (512 MiB): generous
/// for any legitimate reply, while stopping a crafted `$999999999999\r\n` header
/// from making the streaming decoder accumulate an unbounded buffer.
pub(crate) const MAX_BULK_LENGTH: usize = 512 * 1024 * 1024;

/// Maximum number of elements the parser accepts in a single collection (array,
/// set, push, or map — counted after the map key/value doubling). Bounds an
/// attacker-controlled loop count and any future pre-reservation; generous for
/// real replies.
pub(crate) const MAX_COLLECTION_LENGTH: usize = 128 * 1024 * 1024;

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
/// on where an element ends — the divergence class of bug that corrupted
/// elements read against a mismatched layout.
pub(crate) struct ElementBounds {
    pub kind: ElementKind,
    pub value: Range<usize>,
    pub end: usize,
}

/// Rejects a bulk-family length that exceeds [`MAX_BULK_LENGTH`]. `len` must
/// already be known non-negative.
#[inline]
fn check_bulk_len(len: i64) -> Result<()> {
    if len as usize > MAX_BULK_LENGTH {
        return Err(Error::Client(ClientError::BulkLengthTooLarge));
    }
    Ok(())
}

/// Finds the `\r` of the next `\r\n` at or after `from`, returning its index.
/// Errors with [`Error::EOF`] when no complete terminator is present yet.
#[inline]
fn find_crlf(data: &[u8], from: usize) -> Result<usize> {
    let rem = &data[from..];
    let i = memchr(b'\r', rem).ok_or_else(|| Error::EOF)?;
    if i + 1 >= rem.len() || rem[i + 1] != b'\n' {
        return Err(Error::EOF);
    }
    Ok(from + i)
}

/// Parses a RESP integer header starting at `from`, returning `(value, end)`
/// where `end` is the offset just past the terminating `\r\n`. Mirrors
/// [`RespFrameParser::parse_integer`] but over a free slice, for reading a node
/// back without a parser instance.
#[inline]
fn parse_int_at(data: &[u8], from: usize) -> Result<(i64, usize)> {
    let slice = &data[from..];
    let mut i = 0;

    let sign = if let Some(&b'-') = slice.first() {
        i += 1;
        -1
    } else {
        1
    };

    let mut n = 0i64;
    while i < slice.len() {
        match slice[i] {
            b'0'..=b'9' => {
                n = n
                    .checked_mul(10)
                    .and_then(|n| n.checked_add((slice[i] - b'0') as i64))
                    .ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?;
                i += 1;
            }
            b'\r' => match slice.get(i + 1) {
                Some(&b'\n') => return Ok((n * sign, from + i + 2)),
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
pub(crate) fn element_bounds(data: &[u8], off: usize) -> Result<ElementBounds> {
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
            if start + 3 > data.len() {
                return Err(Error::EOF);
            }
            match data[start] {
                b't' | b'f' => {}
                _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
            }
            if &data[start + 1..start + 3] != b"\r\n" {
                return Err(Error::Client(ClientError::CannotParseBoolean));
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
            check_bulk_len(len)?;
            let end = after + len as usize + 2;
            if data.len() < end {
                return Err(Error::EOF);
            }
            if &data[end - 2..end] != b"\r\n" {
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
            check_bulk_len(len)?;
            let end = after + len as usize + 2;
            if data.len() < end {
                return Err(Error::EOF);
            }
            if &data[end - 2..end] != b"\r\n" {
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
            check_bulk_len(len)?;
            let end = after + len as usize + 2;
            if data.len() < end {
                return Err(Error::EOF);
            }
            if &data[end - 2..end] != b"\r\n" {
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

/// Rejects a collection cardinality that exceeds [`MAX_COLLECTION_LENGTH`],
/// bounding an attacker-controlled loop count.
#[inline]
fn check_collection_len(len: usize) -> Result<()> {
    if len > MAX_COLLECTION_LENGTH {
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
    let tag = data[at];
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
fn skip_leading_attributes(data: &[u8], mut pos: usize, depth: usize) -> Result<usize> {
    while pos < data.len() && data[pos] == ATTRIBUTE_TAG {
        if depth + 1 > MAX_NESTING_DEPTH {
            return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
        }
        let (n, after) = parse_int_at(data, pos + 1)?;
        if n < 0 {
            return Err(Error::Client(ClientError::CannotParseMap));
        }
        let count = n as usize * 2;
        check_collection_len(count)?;
        let mut child = after;
        for _ in 0..count {
            child = skip_one_value(data, child, depth + 1)?;
        }
        pos = child;
    }
    Ok(pos)
}

/// Advances past exactly one value at `pos` — a scalar, or a nested collection
/// with all of its descendants — returning the offset just past it. Used only to
/// consume attribute payloads, which carry no tape, so it walks the structure
/// without recording anything. Recursion is bounded by [`MAX_NESTING_DEPTH`].
/// [`Error::EOF`] if the value is incomplete.
fn skip_one_value(data: &[u8], pos: usize, depth: usize) -> Result<usize> {
    let pos = skip_leading_attributes(data, pos, depth)?;
    let tag = *data.get(pos).ok_or_else(|| Error::EOF)?;
    if is_container_tag(tag) {
        match parse_container_header(data, pos)? {
            ContainerHeader::Null { end } => Ok(end),
            ContainerHeader::Open { count, end } => {
                check_collection_len(count)?;
                if depth + 1 > MAX_NESTING_DEPTH {
                    return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
                }
                let mut child = end;
                for _ in 0..count {
                    child = skip_one_value(data, child, depth + 1)?;
                }
                Ok(child)
            }
        }
    } else {
        Ok(element_bounds(data, pos)?.end)
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
/// builder it writes into. One forward pass produces either an inline scalar
/// [`RespFrame`] (top-level scalars carry no tape) or, for a collection, a flat
/// tape of fixed-width nodes written into the borrowed `tape` buffer (see
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
    /// Tape builder, borrowed so the decoder can recycle one `BytesMut` across
    /// frames (`split().freeze()` per frame keeps its capacity). While a frame is
    /// incomplete the partial tape stays here, accumulating across chunks.
    tape: &'b mut BytesMut,
    pos: usize,
}

impl<'a, 'b> RespFrameParser<'a, 'b> {
    /// A parser positioned at the start of `buf`. Used both for one-shot parsing
    /// of a complete buffer and as the streaming decoder's entry point for a
    /// brand-new frame.
    pub fn new(buf: &'a [u8], tape: &'b mut BytesMut) -> Self {
        Self { buf, tape, pos: 0 }
    }

    /// A parser positioned at `pos`, used by the streaming decoder to resume a
    /// frame it previously suspended. The partial tape is expected to already be
    /// present in `tape`, and the open-collection stack is passed to
    /// [`Self::parse_resumable`].
    pub(crate) fn at(buf: &'a [u8], tape: &'b mut BytesMut, pos: usize) -> Self {
        Self { buf, tape, pos }
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
    /// The scalar branch is fully inline and never names a `Vec`: the collection
    /// stack is created only inside the container branch, so a scalar reply — the
    /// hot request/response path — stays exactly as flat as it was before the tape
    /// existed. The skeleton mirrors [`Self::parse_resumable`]; both defer the
    /// actual work to `skip_leading_attributes`, `parse_inline_scalar` and
    /// `begin_collection`, so the two cannot diverge on how a value is decoded.
    #[inline(always)]
    pub fn parse(&mut self) -> Result<(RespFrame, usize)> {
        if self.pos < self.buf.len() && self.buf[self.pos] == ATTRIBUTE_TAG {
            self.pos = skip_leading_attributes(self.buf, self.pos, 0)?;
        }
        let tag = *self.buf.get(self.pos).ok_or_else(|| Error::EOF)?;
        if is_container_tag(tag) {
            let mut stack = Vec::new();
            return match self.begin_collection(tag, &mut stack)? {
                Some(frame) => Ok((frame, self.pos)),
                None => Err(Error::EOF),
            };
        }
        let frame = self.parse_inline_scalar(tag)?;
        Ok((frame, self.pos))
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
    ) -> Result<Option<RespFrame>> {
        if !stack.is_empty() {
            return self.run_collection_loop(stack);
        }

        let frame_start = self.pos;
        // Leading attributes are rare out-of-band metadata; peek before calling so
        // the common scalar path pays nothing. A partial attribute rewinds to the
        // frame start; a complete run is consumed and stays in the buffer.
        if self.pos < self.buf.len() && self.buf[self.pos] == ATTRIBUTE_TAG {
            match skip_leading_attributes(self.buf, self.pos, 0) {
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
        match self.parse_inline_scalar(tag) {
            Ok(frame) => Ok(Some(frame)),
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
    ) -> Result<Option<RespFrame>> {
        let at = self.pos;
        match parse_container_header(self.buf, at) {
            Ok(ContainerHeader::Null { end }) => {
                self.pos = end;
                Ok(Some(RespFrame::Null))
            }
            Ok(ContainerHeader::Open { count, end }) => {
                check_collection_len(count)?;
                debug_assert!(self.tape.is_empty(), "tape must start empty per frame");
                let head = push_node(self.tape, tag, 0);
                push_node(self.tape, TAPE_LEN_TAG, count as u64);
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
    ) -> Result<Option<RespFrame>> {
        loop {
            let remaining = stack
                .last()
                .expect("collection loop entered with an empty stack")
                .remaining;

            if remaining == 0 {
                // Every child of this collection is written. Back-patch its head's
                // `next` to the tape end (the reader's O(1) sibling skip) and close
                // the level, crediting the parent — or finish the frame at the root.
                let done = stack.pop().expect("non-empty stack");
                let next = node_count(self.tape) as u64;
                patch_node(self.tape, done.head_index, done.tag, next);
                if let Some(parent) = stack.last_mut() {
                    parent.remaining -= 1;
                    continue;
                }
                let tape = self.tape.split().freeze();
                return Ok(Some(match done.tag {
                    ARRAY_TAG => RespFrame::Array { tape, root: 0 },
                    MAP_TAG => RespFrame::Map { tape, root: 0 },
                    SET_TAG => RespFrame::Set { tape, root: 0 },
                    PUSH_TAG => RespFrame::Push { tape, root: 0 },
                    _ => unreachable!("a non-container tag on the parse stack"),
                }));
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
        if at < self.buf.len() && self.buf[at] == ATTRIBUTE_TAG {
            at = skip_leading_attributes(self.buf, at, stack.len())?;
        }
        let tag = *self.buf.get(at).ok_or_else(|| Error::EOF)?;

        if is_container_tag(tag) {
            match parse_container_header(self.buf, at)? {
                ContainerHeader::Null { end } => {
                    // A null child collection is one counted element that reads
                    // back as `Null`; its stored offset is never dereferenced.
                    push_node(self.tape, NULL_TAG, at as u64);
                    self.pos = end;
                    credit_open_collection(stack);
                }
                ContainerHeader::Open { count, end } => {
                    check_collection_len(count)?;
                    if stack.len() + 1 > MAX_NESTING_DEPTH {
                        return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
                    }
                    let head = push_node(self.tape, tag, 0);
                    push_node(self.tape, TAPE_LEN_TAG, count as u64);
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
            let bounds = element_bounds(self.buf, at)?;
            push_node(self.tape, tag, at as u64);
            self.pos = bounds.end;
            credit_open_collection(stack);
        }
        Ok(())
    }

    /// Decodes an inline scalar whose tag byte is at `self.pos`, advancing past
    /// it. Top-level scalars carry no tape; this is the allocation- and node-free
    /// hot path. Forced inline so the frame is built directly in the caller's
    /// frame rather than returned across a call boundary, keeping a scalar reply
    /// as flat as it was before the tape. [`Error::EOF`] if the scalar is not
    /// fully present.
    #[inline(always)]
    fn parse_inline_scalar(&mut self, tag: u8) -> Result<RespFrame> {
        self.pos += 1;
        let frame = match tag {
            SIMPLE_STRING_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                RespFrame::SimpleString(start..self.pos - 2)
            }
            SIMPLE_ERROR_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                RespFrame::Error(start..self.pos - 2)
            }
            INTEGER_TAG => {
                let val = self.parse_integer()?;
                RespFrame::Integer(val)
            }
            DOUBLE_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                let val = fast_float2::parse(&self.buf[start..self.pos - 2])
                    .map_err(|_| Error::Client(ClientError::CannotParseDouble))?;
                RespFrame::Double(val)
            }
            NULL_TAG => {
                self.parse_crlf()?;
                RespFrame::Null
            }
            BOOL_TAG => {
                if self.pos + 3 > self.buf.len() {
                    return Err(Error::EOF);
                }
                let b = match self.buf[self.pos] {
                    b't' => true,
                    b'f' => false,
                    _ => return Err(Error::Client(ClientError::CannotParseBoolean)),
                };
                if &self.buf[self.pos + 1..self.pos + 3] != b"\r\n" {
                    return Err(Error::Client(ClientError::CannotParseBoolean));
                }
                self.pos += 3;
                RespFrame::Boolean(b)
            }
            BULK_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    if len < 0 {
                        return Err(Error::Client(ClientError::CannotParseBulkString));
                    }
                    check_bulk_len(len)?;
                    let start = self.pos;
                    let need = self.pos + len as usize + 2;
                    if self.buf.len() < need {
                        return Err(Error::EOF);
                    }
                    if &self.buf[need - 2..need] != b"\r\n" {
                        return Err(Error::Client(ClientError::CannotParseBulkString));
                    }
                    self.pos = need;
                    RespFrame::BulkString(start..need - 2)
                }
            }
            // The first three bytes provide information about the format of the following string,
            // which can be txt for plain text, or mkd for markdown.
            // The fourth byte is always :. Then the real string follows.
            VERBATIM_STRING_TAG => {
                let len = self.parse_integer()?;
                if len == -1 {
                    RespFrame::Null
                } else {
                    if len < 4 {
                        return Err(Error::Client(ClientError::VerbatimStringTooShort));
                    }
                    check_bulk_len(len)?;
                    let start = self.pos;
                    let need = self.pos + len as usize + 2;
                    if self.buf.len() < need {
                        return Err(Error::EOF);
                    }
                    if &self.buf[need - 2..need] != b"\r\n" {
                        return Err(Error::Client(ClientError::CannotParseVerbatimString));
                    }
                    self.pos = need;
                    RespFrame::BulkString(start + 4..need - 2)
                }
            }
            BULK_ERROR_TAG => {
                let len = self.parse_integer()?;
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseBulkError));
                }
                check_bulk_len(len)?;
                let start = self.pos;
                let need = self.pos + len as usize + 2;
                if self.buf.len() < need {
                    return Err(Error::EOF);
                }
                if &self.buf[need - 2..need] != b"\r\n" {
                    return Err(Error::Client(ClientError::CannotParseBulkError));
                }
                self.pos = need;
                RespFrame::Error(start..need - 2)
            }
            // A big number does not fit in an i64; surface it as its
            // decimal-string payload so the caller can read it as a string.
            BIG_NUMBER_TAG => {
                let start = self.pos;
                self.parse_crlf()?;
                RespFrame::BulkString(start..self.pos - 2)
            }
            _ => return Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        };

        Ok(frame)
    }

    #[inline]
    fn parse_crlf(&mut self) -> Result<()> {
        let rem = &self.buf[self.pos..];
        let i = memchr(b'\r', rem).ok_or_else(|| Error::EOF)?;
        if i + 1 >= rem.len() || rem[i + 1] != b'\n' {
            return Err(Error::EOF);
        }
        self.pos += i + 2;
        Ok(())
    }

    #[inline]
    fn parse_integer(&mut self) -> Result<i64> {
        let (val, end) = parse_int_at(self.buf, self.pos)?;
        self.pos = end;
        Ok(val)
    }
}

/// Records that one child of the innermost open collection is fully parsed.
#[inline]
fn credit_open_collection(stack: &mut [PendingContainer]) {
    stack
        .last_mut()
        .expect("a child was parsed without an open collection")
        .remaining -= 1;
}
