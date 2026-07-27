use crate::{
    ClientError, Error, Result,
    client::RespLimits,
    resp::{
        ATTRIBUTE_TAG, MAP_TAG, NULL_TAG, RespTape, RespTapeMut, TAPE_LEN_TAG, element_bounds,
        is_collection_tag, parse_int_at,
    },
};
use std::fmt;

/// What one forward pass recovers from a frame's bytes.
///
/// The parser only *frames*: it finds where the frame ends so the buffer can be
/// sliced, and indexes a collection's elements. It decodes no value — the tag
/// alone says how to read one, and the read happens in the calling task rather
/// than in the shared network task.
pub enum ParsedFrame {
    /// A single scalar, whose tag byte sits at `at` in the frame. It carries no
    /// tape: one node for one value would buy nothing, and keeping the hot
    /// request/response path node-free keeps the recycled tape buffer untouched.
    Scalar { at: usize },
    /// A collection, with the tape indexing it and all of its descendants, rooted
    /// at node 0.
    Collection(RespTape),
    /// A null collection (`*-1\r\n`): a collection tag introducing no element.
    /// Its bytes hold nothing to read back, so they are dropped.
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
enum CollectionHeader {
    /// A null collection (`*-1\r\n`): it has no children and deserializes to
    /// `Null`, but is still counted as one element by its parent.
    Null { end: usize },
    /// A present collection with `count` children to follow (already doubled for
    /// maps), whose first child begins at `end`.
    Open { count: usize, end: usize },
}

/// Reads the header of the collection whose tag byte is at `at`, returning its
/// child count (doubled for maps) and the offset just past the `\r\n`. `at` must
/// point at a collection tag. [`Error::EOF`] when the header has not fully
/// arrived, so the streaming decoder can retry once more bytes are read.
fn parse_collection_header(data: &[u8], at: usize) -> Result<CollectionHeader> {
    let tag = *data.get(at).ok_or_else(|| Error::EOF)?;
    let (n, end) = parse_int_at(data, at + 1)?;
    if n == -1 {
        return Ok(CollectionHeader::Null { end });
    }
    if n < 0 {
        return Err(Error::Client(if tag == MAP_TAG {
            ClientError::CannotParseMap
        } else {
            ClientError::CannotParseSequence
        }));
    }
    let multiplier = if tag == MAP_TAG { 2 } else { 1 };
    Ok(CollectionHeader::Open {
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
    if is_collection_tag(tag) {
        match parse_collection_header(data, pos)? {
            CollectionHeader::Null { end } => Ok(end),
            CollectionHeader::Open { count, end } => {
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
pub(crate) struct OpenCollection {
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
/// (owned by the caller, see [`OpenCollection`]), not recursion: each step
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
    /// inside the collection branch, so a scalar reply — the hot request/response
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
        if is_collection_tag(tag) {
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
        stack: &mut Vec<OpenCollection>,
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
        if is_collection_tag(tag) {
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
    /// rewinds `pos` to the collection tag (any leading attributes are already
    /// consumed and remain buffered), so a later chunk re-reads the header.
    fn begin_collection(
        &mut self,
        tag: u8,
        stack: &mut Vec<OpenCollection>,
    ) -> Result<Option<ParsedFrame>> {
        let at = self.pos;
        match parse_collection_header(self.buf, at) {
            Ok(CollectionHeader::Null { end }) => {
                self.pos = end;
                Ok(Some(ParsedFrame::Null))
            }
            Ok(CollectionHeader::Open { count, end }) => {
                check_collection_len(count, self.limits.max_collection_length)?;
                debug_assert!(self.tape.is_empty(), "tape must start empty per frame");
                let head = self.tape.push(tag, 0);
                self.tape.push(TAPE_LEN_TAG, count as u64);
                self.pos = end;
                stack.push(OpenCollection {
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
        stack: &mut Vec<OpenCollection>,
    ) -> Result<Option<ParsedFrame>> {
        // The loop is entered with at least one open collection and returns the
        // moment the root closes, so neither the empty stack nor the non-collection
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
                if !is_collection_tag(done.tag) {
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
    fn emit_one_child(&mut self, stack: &mut Vec<OpenCollection>) -> Result<()> {
        // Elements rarely carry a leading attribute; peek before paying for the
        // (non-inlinable, recursive) skip call, so the common case is one compare.
        let mut at = self.pos;
        if self.buf.get(at) == Some(&ATTRIBUTE_TAG) {
            at = skip_leading_attributes(self.buf, at, stack.len(), &self.limits)?;
        }
        let tag = *self.buf.get(at).ok_or_else(|| Error::EOF)?;

        if is_collection_tag(tag) {
            match parse_collection_header(self.buf, at)? {
                CollectionHeader::Null { end } => {
                    // A null child collection is one counted element that reads
                    // back as `Null`; its stored offset is never dereferenced.
                    self.tape.push(NULL_TAG, at as u64);
                    self.pos = end;
                    credit_open_collection(stack);
                }
                CollectionHeader::Open { count, end } => {
                    check_collection_len(count, self.limits.max_collection_length)?;
                    if stack.len() + 1 > self.limits.max_nesting_depth {
                        return Err(Error::Client(ClientError::MaxNestingDepthExceeded));
                    }
                    let head = self.tape.push(tag, 0);
                    self.tape.push(TAPE_LEN_TAG, count as u64);
                    self.pos = end;
                    stack.push(OpenCollection {
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
fn credit_open_collection(stack: &mut [OpenCollection]) {
    if let Some(open) = stack.last_mut() {
        open.remaining -= 1;
    }
}
