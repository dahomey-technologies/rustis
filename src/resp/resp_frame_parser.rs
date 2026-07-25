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
/// reply from recursing `emit_value` into a stack overflow (HARD-01), which —
/// unlike a panic — is not catchable and aborts the whole process.
pub(crate) const MAX_NESTING_DEPTH: usize = 128;

/// Maximum byte length the parser accepts for a single bulk string, bulk error
/// or verbatim string, checked against the declared header before the payload
/// is trusted. Matches Redis's default `proto-max-bulk-len` (512 MiB): generous
/// for any legitimate reply, while stopping a crafted `$999999999999\r\n` header
/// from making the streaming decoder accumulate an unbounded buffer (HARD-02).
pub(crate) const MAX_BULK_LENGTH: usize = 512 * 1024 * 1024;

/// Maximum number of elements the parser accepts in a single collection (array,
/// set, push, or map — counted after the map key/value doubling). Bounds an
/// attacker-controlled loop count and any future pre-reservation (HARD-02);
/// generous for real replies.
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
    let i = memchr(b'\r', rem).ok_or(Error::EOF)?;
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
                    .ok_or(Error::Client(ClientError::CannotParseInteger))?;
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
    let tag = *data.get(off).ok_or(Error::EOF)?;
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

/// Streaming RESP parser. One forward pass produces either an inline scalar
/// [`RespFrame`] (top-level scalars carry no tape) or, for a collection, a flat
/// tape of fixed-width nodes written into the borrowed `tape` buffer (see
/// [`crate::resp::resp_tape`]).
pub struct RespFrameParser<'a, 'b> {
    buf: &'a [u8],
    /// Tape builder, borrowed so the decoder can recycle one `BytesMut` across
    /// frames (`split().freeze()` per frame keeps its capacity).
    tape: &'b mut BytesMut,
    pos: usize,
    /// Current collection-nesting depth, bounded by [`MAX_NESTING_DEPTH`].
    depth: usize,
}

impl<'a, 'b> RespFrameParser<'a, 'b> {
    pub fn new(buf: &'a [u8], tape: &'b mut BytesMut) -> Self {
        Self {
            buf,
            tape,
            pos: 0,
            depth: 0,
        }
    }

    /// Enters one collection-nesting level, rejecting the frame once the bound
    /// is crossed. Paired with [`Self::leave`] on the success path; error paths
    /// abort parsing outright, so the counter need not be unwound there.
    #[inline]
    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
        }
        Ok(())
    }

    #[inline]
    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Rejects a collection cardinality that exceeds [`MAX_COLLECTION_LENGTH`].
    #[inline]
    fn check_collection_len(len: usize) -> Result<()> {
        if len > MAX_COLLECTION_LENGTH {
            return Err(Error::Client(ClientError::CollectionLengthTooLarge));
        }
        Ok(())
    }

    /// Consumes any leading RESP3 attribute frames (`|<n>\r\n` followed by `2n`
    /// values). Attributes are out-of-band metadata that may legally precede
    /// *any* reply, so they are skipped at frame-dispatch level and never
    /// surfaced — neither as a frame nor as a tape node. Element values are
    /// consumed with [`Self::skip_value`], which itself skips nested attributes.
    #[inline]
    fn skip_attributes(&mut self) -> Result<()> {
        while self.pos < self.buf.len() && self.buf[self.pos] == ATTRIBUTE_TAG {
            self.pos += 1;
            let len = self.parse_integer()?;
            if len < 0 {
                return Err(Error::Client(ClientError::CannotParseMap));
            }
            let len = len as usize * 2;
            Self::check_collection_len(len)?;
            self.enter()?;
            for _ in 0..len {
                self.skip_value()?;
            }
            self.leave();
        }
        Ok(())
    }

    #[inline(always)]
    pub fn parse(&mut self) -> Result<(RespFrame, usize)> {
        self.skip_attributes()?;
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }
        let tag = self.buf[self.pos];

        // Collections build a tape; top-level scalars stay inline (no tape),
        // keeping the scalar path allocation- and node-free.
        if is_container_tag(tag) {
            let frame = self.parse_top_container(tag)?;
            return Ok((frame, self.pos));
        }

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

        Ok((frame, self.pos))
    }

    /// Parses a top-level collection header (positioned at its tag) and, unless
    /// it is a null collection, builds its tape rooted at node 0.
    fn parse_top_container(&mut self, tag: u8) -> Result<RespFrame> {
        self.pos += 1;
        let multiplier = if tag == MAP_TAG { 2 } else { 1 };
        let n = self.parse_integer()?;
        if n == -1 {
            return Ok(RespFrame::Null);
        }
        if n < 0 {
            return Err(Error::Client(ClientError::CannotParseSequence));
        }
        let count = n as usize * multiplier;
        Self::check_collection_len(count)?;

        debug_assert!(self.tape.is_empty(), "tape must start empty per frame");
        self.emit_container_body(tag, count)?;
        let tape = self.tape.split().freeze();

        Ok(match tag {
            ARRAY_TAG => RespFrame::Array { tape, root: 0 },
            MAP_TAG => RespFrame::Map { tape, root: 0 },
            SET_TAG => RespFrame::Set { tape, root: 0 },
            PUSH_TAG => RespFrame::Push { tape, root: 0 },
            _ => unreachable!("parse_top_container called with a non-container tag"),
        })
    }

    /// Emits the `[head, len, children…]` nodes of a collection whose element
    /// count is already known and whose first child is at `self.pos`. The head's
    /// `next` is back-patched once the whole subtree has been written, giving an
    /// O(1) sibling skip.
    fn emit_container_body(&mut self, tag: u8, count: usize) -> Result<()> {
        self.enter()?;
        let head = push_node(self.tape, tag, 0);
        push_node(self.tape, TAPE_LEN_TAG, count as u64);
        for _ in 0..count {
            self.emit_value()?;
        }
        let next = node_count(self.tape) as u64;
        patch_node(self.tape, head, tag, next);
        self.leave();
        Ok(())
    }

    /// Parses one value at `self.pos`, emitting its node(s) into the tape and
    /// advancing past it. Scalars emit one node holding their start offset;
    /// collections recurse through [`Self::emit_container_body`]; a null
    /// collection emits a single [`NULL_TAG`] node so it is still counted.
    fn emit_value(&mut self) -> Result<()> {
        self.skip_attributes()?;
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }
        let tag = self.buf[self.pos];

        if is_container_tag(tag) {
            let off = self.pos;
            self.pos += 1;
            let multiplier = if tag == MAP_TAG { 2 } else { 1 };
            let n = self.parse_integer()?;
            if n == -1 {
                push_node(self.tape, NULL_TAG, off as u64);
                return Ok(());
            }
            if n < 0 {
                return Err(Error::Client(if tag == MAP_TAG {
                    ClientError::CannotParseMap
                } else {
                    ClientError::CannotParseSequence
                }));
            }
            let count = n as usize * multiplier;
            Self::check_collection_len(count)?;
            self.emit_container_body(tag, count)
        } else {
            let off = self.pos;
            let bounds = element_bounds(self.buf, off)?;
            push_node(self.tape, tag, off as u64);
            self.pos = bounds.end;
            Ok(())
        }
    }

    /// Advances past one value at `self.pos` without emitting any tape, used to
    /// consume attribute payloads. Structurally identical to [`Self::emit_value`]
    /// minus the node writes.
    fn skip_value(&mut self) -> Result<()> {
        self.skip_attributes()?;
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }
        let tag = self.buf[self.pos];

        if is_container_tag(tag) {
            self.pos += 1;
            let multiplier = if tag == MAP_TAG { 2 } else { 1 };
            let n = self.parse_integer()?;
            if n == -1 {
                return Ok(());
            }
            if n < 0 {
                return Err(Error::Client(if tag == MAP_TAG {
                    ClientError::CannotParseMap
                } else {
                    ClientError::CannotParseSequence
                }));
            }
            let count = n as usize * multiplier;
            Self::check_collection_len(count)?;
            self.enter()?;
            for _ in 0..count {
                self.skip_value()?;
            }
            self.leave();
            Ok(())
        } else {
            self.pos = element_bounds(self.buf, self.pos)?.end;
            Ok(())
        }
    }

    #[inline]
    fn parse_crlf(&mut self) -> Result<()> {
        let rem = &self.buf[self.pos..];
        let i = memchr(b'\r', rem).ok_or(Error::EOF)?;
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
