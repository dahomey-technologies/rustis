use crate::{
    ClientError, Error, RedisError, Result,
    resp::{
        ARRAY_TAG, ElementKind, MAP_TAG, NO_BULK_LIMIT, NULL_TAG, PUSH_TAG, RespBuf,
        RespDeserializer, SET_TAG, TAPE_NODE_SIZE, element_bounds, is_container_tag, node_payload,
        node_tag, read_node,
    },
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::{
    fmt::{self, Write as _},
    ops::Range,
};

/// A decoded RESP frame.
///
/// Scalars carry their value inline (or, for strings, a frame-relative byte
/// range). A collection carries a flat parse **tape** — one fixed-width node per
/// element, all nesting levels — rooted at node `root`, so reading an element is
/// an O(1) node lookup instead of re-parsing the collection from the start. See
/// [`crate::resp::resp_tape`] for the node layout.
///
/// > **Breaking change (unreleased).** The collection variants previously held
/// > `{ len: usize, ranges: [Range<u32>; 5] }`; they now hold `{ tape: Bytes,
/// > root: u32 }`. Code that matched on the old shape must be updated.
#[derive(Clone, PartialEq)]
pub enum RespFrame {
    SimpleString(Range<usize>),
    Integer(i64),
    Double(f64),
    BulkString(Range<usize>),
    Boolean(bool),
    Array { tape: Bytes, root: u32 },
    Map { tape: Bytes, root: u32 },
    Set { tape: Bytes, root: u32 },
    Push { tape: Bytes, root: u32 },
    Error(Range<usize>),
    Null,
}

/// A frame carries no data buffer, so it cannot render the reply it describes;
/// it reports its kind, and — for a collection — the shape it indexes. The tape
/// itself stays out: it is an internal index whose raw bytes are unreadable and
/// routinely larger than the reply. To see the decoded reply, format the
/// enclosing [`RespResponse`].
impl fmt::Debug for RespFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(r) => f.debug_tuple("SimpleString").field(r).finish(),
            Self::Integer(i) => f.debug_tuple("Integer").field(i).finish(),
            Self::Double(d) => f.debug_tuple("Double").field(d).finish(),
            Self::BulkString(r) => f.debug_tuple("BulkString").field(r).finish(),
            Self::Boolean(b) => f.debug_tuple("Boolean").field(b).finish(),
            Self::Error(r) => f.debug_tuple("Error").field(r).finish(),
            Self::Null => f.write_str("Null"),
            Self::Array { tape, root } => fmt_frame_shape(f, "Array", tape, *root),
            Self::Map { tape, root } => fmt_frame_shape(f, "Map", tape, *root),
            Self::Set { tape, root } => fmt_frame_shape(f, "Set", tape, *root),
            Self::Push { tape, root } => fmt_frame_shape(f, "Push", tape, *root),
        }
    }
}

/// Renders a collection frame as `Kind { root, len }`. `len` lives in the head's
/// companion node; a tape too short to hold it means the frame was not produced
/// by the parser, so the count is reported as unknown rather than panicking in a
/// formatter.
fn fmt_frame_shape(f: &mut fmt::Formatter<'_>, kind: &str, tape: &Bytes, root: u32) -> fmt::Result {
    let mut s = f.debug_struct(kind);
    s.field("root", &root);
    match tape_len(tape, root as usize) {
        Some(len) => s.field("len", &len),
        None => s.field("len", &format_args!("<unreadable tape>")),
    };
    s.finish()
}

/// Reads a container's element count from its companion node, or `None` when the
/// tape is too short to hold one.
fn tape_len(tape: &[u8], root: usize) -> Option<usize> {
    let companion = root.checked_add(1)?;
    if tape.len() / TAPE_NODE_SIZE <= companion {
        return None;
    }
    Some(node_payload(read_node(tape, companion)) as usize)
}

#[derive(Clone, PartialEq)]
pub enum RespResponse {
    IntegerArray(Vec<i64>),
    OwnedArray(Vec<RespResponse>),
    Frame(RespBuf, RespFrame),
}

/// Above this many characters, the rendering stops and is marked as truncated.
/// A response is formatted on the connection's debug-log path, where a
/// multi-megabyte reply would otherwise build a multi-megabyte log line.
const DEBUG_RENDER_LIMIT: usize = 1000;

/// Renders the decoded reply — the buffer read through the frame — rather than
/// the two raw fields, which would print the parse tape's bytes. Formatting goes
/// straight to the output through a capped writer: no intermediate [`Value`] is
/// materialized, so a large reply costs no allocation beyond the cap.
///
/// [`Value`]: crate::resp::Value
impl fmt::Debug for RespResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let truncated = {
            let mut writer = TruncatingWriter::new(f, DEBUG_RENDER_LIMIT);
            write!(writer, "{:?}", self.view())?;
            writer.truncated
        };
        if truncated {
            f.write_str("<truncated>")?;
        }
        Ok(())
    }
}

/// Forwards to a formatter until `remaining` characters have been written, then
/// silently drops the rest and raises `truncated`.
struct TruncatingWriter<'a, 'b> {
    inner: &'a mut fmt::Formatter<'b>,
    remaining: usize,
    truncated: bool,
}

impl<'a, 'b> TruncatingWriter<'a, 'b> {
    fn new(inner: &'a mut fmt::Formatter<'b>, limit: usize) -> Self {
        Self {
            inner,
            remaining: limit,
            truncated: false,
        }
    }
}

impl fmt::Write for TruncatingWriter<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() <= self.remaining {
            self.remaining -= s.len();
            return self.inner.write_str(s);
        }
        // Cut on a char boundary: slicing mid-code-point would panic.
        let mut end = self.remaining;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        self.remaining = 0;
        self.truncated = true;
        self.inner.write_str(&s[..end])
    }
}

impl RespResponse {
    #[inline(always)]
    pub fn new(buf: RespBuf, frame: RespFrame) -> Self {
        Self::Frame(buf, frame)
    }

    #[inline(always)]
    pub fn view(&self) -> RespView<'_> {
        match self {
            RespResponse::IntegerArray(a) => RespView::IntegerArray(a),
            RespResponse::OwnedArray(a) => RespView::OwnedArray(a),
            RespResponse::Frame(buf, frame) => RespView::from_frame(buf.as_ref(), frame),
        }
    }

    /// Returns `true` if the RESP Response is a push message
    #[inline(always)]
    pub fn is_push(&self) -> bool {
        matches!(self, RespResponse::Frame(_, RespFrame::Push { .. }))
    }

    /// Returns `true` if the RESP Response is a monitor message
    #[inline(always)]
    pub fn is_monitor(&self) -> bool {
        matches!(self, RespResponse::Frame(buf, RespFrame::SimpleString(r)) if buf.as_ref().get(r.start).is_some_and(|f| f.is_ascii_digit()))
    }

    /// Returns `true` if the RESP Response is a Redis error
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        matches!(self, RespResponse::Frame(_, RespFrame::Error(_)))
    }

    #[inline(always)]
    pub fn null() -> RespResponse {
        Self::Frame(RespBuf::default(), RespFrame::Null)
    }

    #[inline(always)]
    pub fn integer(i: i64) -> RespResponse {
        Self::Frame(RespBuf::default(), RespFrame::Integer(i))
    }

    #[inline(always)]
    pub fn integer_array(a: Vec<i64>) -> RespResponse {
        Self::IntegerArray(a)
    }

    #[inline(always)]
    pub fn owned_array(a: Vec<RespResponse>) -> RespResponse {
        Self::OwnedArray(a)
    }

    /// Constructs a new `Response` as a RESP Ok message (+OK\r\n)
    #[inline(always)]
    pub fn ok() -> RespResponse {
        Self::Frame(
            RespBuf::from(Bytes::from_static(b"+OK\r\n")),
            RespFrame::SimpleString(1..3),
        )
    }

    /// Convert the RESP Response to a Rust type `T` by using serde deserialization
    #[inline]
    pub fn to<T: DeserializeOwned>(&self) -> Result<T> {
        T::deserialize(RespDeserializer::new(self.view()))
    }

    /// Returns a self-contained copy whose buffers hold **only** the bytes this
    /// response actually references, releasing any larger shared block it was
    /// carved from.
    ///
    /// A response kept alive long after decoding (a cache entry, a buffered
    /// stream item) pins the whole recycled block its data — and, for a
    /// collection, its tape — was split from: a 50-byte cached value can pin a
    /// 64 KiB block. Compacting before retaining copies the referenced bytes out
    /// so the block can be reclaimed. Callers that consume a response promptly
    /// (the normal request/response path) never need this.
    pub fn compact(&self) -> RespResponse {
        match self {
            RespResponse::IntegerArray(a) => RespResponse::IntegerArray(a.clone()),
            RespResponse::OwnedArray(a) => {
                RespResponse::OwnedArray(a.iter().map(RespResponse::compact).collect())
            }
            RespResponse::Frame(buf, frame) => {
                let (buf, frame) = compact_frame(buf.as_ref(), frame);
                RespResponse::Frame(buf, frame)
            }
        }
    }

    pub fn into_array_iter(self) -> Result<RespResponseIter> {
        match self {
            RespResponse::Frame(
                buf,
                RespFrame::Array { tape, root } | RespFrame::Set { tape, root },
            ) => {
                let len = node_payload(read_node(&tape, root as usize + 1)) as usize;
                Ok(RespResponseIter::new(buf, tape, root as usize, len))
            }
            RespResponse::Frame(buf, RespFrame::Error(r)) => {
                Err(Error::Redis(RedisError::try_from(buf.slice(r).as_ref())?))
            }
            _ => Err(Error::Client(ClientError::Unexpected)),
        }
    }
}

/// Copies the bytes a single frame references into freshly-sized buffers. Scalar
/// strings shrink to exactly their value; a collection copies its data and tape
/// buffers wholesale (frame-sized at the top level), which is enough to release
/// the shared block.
#[expect(
    clippy::indexing_slicing,
    reason = "invariant: `frame`'s ranges were produced by the parser over this \
              very `data`; `RespResponse` owns the two together and never pairs \
              a frame with another buffer."
)]
fn compact_frame(data: &[u8], frame: &RespFrame) -> (RespBuf, RespFrame) {
    match frame {
        RespFrame::SimpleString(r) => (
            RespBuf::from(Bytes::copy_from_slice(&data[r.clone()])),
            RespFrame::SimpleString(0..r.end - r.start),
        ),
        RespFrame::BulkString(r) => (
            RespBuf::from(Bytes::copy_from_slice(&data[r.clone()])),
            RespFrame::BulkString(0..r.end - r.start),
        ),
        RespFrame::Error(r) => (
            RespBuf::from(Bytes::copy_from_slice(&data[r.clone()])),
            RespFrame::Error(0..r.end - r.start),
        ),
        RespFrame::Integer(i) => (RespBuf::default(), RespFrame::Integer(*i)),
        RespFrame::Double(d) => (RespBuf::default(), RespFrame::Double(*d)),
        RespFrame::Boolean(b) => (RespBuf::default(), RespFrame::Boolean(*b)),
        RespFrame::Null => (RespBuf::default(), RespFrame::Null),
        RespFrame::Array { tape, root } => (
            RespBuf::from(Bytes::copy_from_slice(data)),
            RespFrame::Array {
                tape: Bytes::copy_from_slice(tape),
                root: *root,
            },
        ),
        RespFrame::Map { tape, root } => (
            RespBuf::from(Bytes::copy_from_slice(data)),
            RespFrame::Map {
                tape: Bytes::copy_from_slice(tape),
                root: *root,
            },
        ),
        RespFrame::Set { tape, root } => (
            RespBuf::from(Bytes::copy_from_slice(data)),
            RespFrame::Set {
                tape: Bytes::copy_from_slice(tape),
                root: *root,
            },
        ),
        RespFrame::Push { tape, root } => (
            RespBuf::from(Bytes::copy_from_slice(data)),
            RespFrame::Push {
                tape: Bytes::copy_from_slice(tape),
                root: *root,
            },
        ),
    }
}

/// Reads the scalar node with tag `tag` at byte offset `off` into a borrowed
/// [`RespView`]. Returns `None` when the element's content fails to parse (a
/// malformed integer or double), which ends iteration — the pre-tape behaviour.
///
/// A [`NULL_TAG`] node is `Null` without touching the data buffer: it may stand
/// in for a null collection (`*-1`), whose offset points at `*`, not a scalar.
#[inline]
fn read_scalar_view<'a>(tag: u8, data: &'a [u8], off: usize) -> Option<RespView<'a>> {
    if tag == NULL_TAG {
        return Some(RespView::Null);
    }
    // Read-back of a frame the decoder already validated: re-applying a bulk
    // cap here would reject values a raised limit legitimately let through.
    let bounds = element_bounds(data, off, NO_BULK_LIMIT).ok()?;
    Some(match bounds.kind {
        ElementKind::SimpleString => RespView::SimpleString(data.get(bounds.value)?),
        ElementKind::Error => RespView::Error(data.get(bounds.value)?),
        ElementKind::Integer => RespView::Integer(atoi::atoi(data.get(bounds.value)?)?),
        ElementKind::Double => RespView::Double(fast_float2::parse(data.get(bounds.value)?).ok()?),
        ElementKind::BulkString => RespView::BulkString(data.get(bounds.value)?),
        ElementKind::Boolean => RespView::Boolean(*data.get(bounds.value.start)? == b't'),
        ElementKind::Null => RespView::Null,
    })
}

/// Owned equivalent of [`read_scalar_view`], producing a self-contained
/// [`RespFrame`] for an element yielded by [`RespResponseIter`].
#[inline]
fn read_scalar_frame(tag: u8, data: &[u8], off: usize) -> Option<RespFrame> {
    if tag == NULL_TAG {
        return Some(RespFrame::Null);
    }
    // Read-back of a frame the decoder already validated: re-applying a bulk
    // cap here would reject values a raised limit legitimately let through.
    let bounds = element_bounds(data, off, NO_BULK_LIMIT).ok()?;
    Some(match bounds.kind {
        ElementKind::SimpleString => RespFrame::SimpleString(bounds.value),
        ElementKind::Error => RespFrame::Error(bounds.value),
        ElementKind::Integer => RespFrame::Integer(atoi::atoi(data.get(bounds.value)?)?),
        ElementKind::Double => RespFrame::Double(fast_float2::parse(data.get(bounds.value)?).ok()?),
        ElementKind::BulkString => RespFrame::BulkString(bounds.value),
        ElementKind::Boolean => RespFrame::Boolean(*data.get(bounds.value.start)? == b't'),
        ElementKind::Null => RespFrame::Null,
    })
}

/// Builds the borrowed collection view for a container node at tape index `root`.
#[inline]
#[expect(
    clippy::unreachable,
    reason = "invariant: callers gate on `is_container_tag`, whose `matches!` \
              lists exactly these four tags. The arm asserts that pairing; a \
              fallback would have to invent a view for a tag that is not a \
              container."
)]
fn container_view<'a>(tag: u8, buf: &'a [u8], tape: &'a [u8], root: usize) -> RespView<'a> {
    let view = RespArrayView::new(buf, tape, root);
    match tag {
        ARRAY_TAG => RespView::Array(view),
        MAP_TAG => RespView::Map(view),
        SET_TAG => RespView::Set(view),
        PUSH_TAG => RespView::Push(view),
        _ => unreachable!("container_view called with a non-container tag"),
    }
}

#[derive(PartialEq)]
pub enum RespView<'a> {
    SimpleString(&'a [u8]),
    Integer(i64),
    Double(f64),
    BulkString(&'a [u8]),
    Boolean(bool),
    IntegerArray(&'a [i64]),
    OwnedArray(&'a [RespResponse]),
    Array(RespArrayView<'a>),
    Map(RespArrayView<'a>),
    Set(RespArrayView<'a>),
    Push(RespArrayView<'a>),
    Error(&'a [u8]),
    Null,
}

impl<'a> RespView<'a> {
    /// Borrows a decoded frame as a view. Collections read their structure from
    /// the frame's tape; scalars read their bytes from `data`.
    #[inline]
    #[expect(
        clippy::indexing_slicing,
        reason = "invariant: `frame`'s ranges were produced by the parser over \
                  this very `data`; the two travel together inside a \
                  `RespResponse`."
    )]
    fn from_frame(data: &'a [u8], frame: &'a RespFrame) -> RespView<'a> {
        match frame {
            RespFrame::SimpleString(r) => RespView::SimpleString(&data[r.clone()]),
            RespFrame::Integer(i) => RespView::Integer(*i),
            RespFrame::Double(f) => RespView::Double(*f),
            RespFrame::BulkString(r) => RespView::BulkString(&data[r.clone()]),
            RespFrame::Boolean(b) => RespView::Boolean(*b),
            RespFrame::Array { tape, root } => {
                RespView::Array(RespArrayView::new(data, tape.as_ref(), *root as usize))
            }
            RespFrame::Map { tape, root } => {
                RespView::Map(RespArrayView::new(data, tape.as_ref(), *root as usize))
            }
            RespFrame::Set { tape, root } => {
                RespView::Set(RespArrayView::new(data, tape.as_ref(), *root as usize))
            }
            RespFrame::Push { tape, root } => {
                RespView::Push(RespArrayView::new(data, tape.as_ref(), *root as usize))
            }
            RespFrame::Error(r) => RespView::Error(&data[r.clone()]),
            RespFrame::Null => RespView::Null,
        }
    }
}

impl<'a> fmt::Debug for RespView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f
                .debug_tuple("SimpleString")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::BulkString(arg0) => f
                .debug_tuple("BulkString")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::IntegerArray(arg0) => f.debug_tuple("IntegerArray").field(arg0).finish(),
            Self::OwnedArray(arg0) => f.debug_tuple("OwnedArray").field(arg0).finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Map(arg0) => {
                f.write_str("Map(")?;
                fmt_pairs(f, arg0)?;
                f.write_str(")")
            }
            Self::Set(arg0) => f.debug_tuple("Set").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Error(arg0) => f
                .debug_tuple("Error")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::Null => write!(f, "Null"),
        }
    }
}

/// Renders a map's flat element sequence as `{key: value, …}`. An element that
/// the tape cannot resolve is rendered in place instead of aborting the whole
/// rendering — a formatter cannot report an error other than "formatting
/// failed", and a debug line showing where a reply went wrong beats no line.
fn fmt_pairs(f: &mut fmt::Formatter<'_>, view: &RespArrayView<'_>) -> fmt::Result {
    let mut map = f.debug_map();
    let mut it = view.clone().into_iter();
    while let Some(key) = it.next() {
        match (key, it.next()) {
            (Ok(k), Some(Ok(v))) => map.entry(&k, &v),
            (Ok(k), Some(Err(e))) => map.entry(&k, &UnreadableElement(e)),
            (Ok(k), None) => map.entry(&k, &format_args!("<missing value>")),
            (Err(e), v) => match v {
                Some(Ok(v)) => map.entry(&UnreadableElement(e), &v),
                Some(Err(e2)) => map.entry(&UnreadableElement(e), &UnreadableElement(e2)),
                None => map.entry(&UnreadableElement(e), &format_args!("<missing value>")),
            },
        };
    }
    map.finish()
}

/// Stands in for an element the tape could not resolve while formatting.
struct UnreadableElement(Error);

impl fmt::Debug for UnreadableElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<unreadable element: {}>", self.0)
    }
}

/// A borrowed view over a collection: the data buffer, the parse tape, and the
/// tape index of the container's head node. `len` (the exact element count) is
/// read once from the head's companion node, so it is O(1).
#[derive(Clone, PartialEq)]
pub struct RespArrayView<'a> {
    buf: &'a [u8],
    tape: &'a [u8],
    root: usize,
    len: usize,
}

impl<'a> RespArrayView<'a> {
    #[inline(always)]
    pub fn new(buf: &'a [u8], tape: &'a [u8], root: usize) -> Self {
        let len = node_payload(read_node(tape, root + 1)) as usize;
        Self {
            buf,
            tape,
            root,
            len,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
}

/// Renders the elements, not the two raw buffers the view borrows. Unresolvable
/// elements are rendered in place, as in [`fmt_pairs`].
impl fmt::Debug for RespArrayView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for element in self.clone() {
            match element {
                Ok(view) => list.entry(&view),
                Err(e) => list.entry(&UnreadableElement(e)),
            };
        }
        list.finish()
    }
}

impl<'a> IntoIterator for RespArrayView<'a> {
    type Item = Result<RespView<'a>>;
    type IntoIter = RespArrayIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RespArrayIter::new(self.buf, self.tape, self.root, self.len)
    }
}

/// Walks the direct children of a collection by stepping the tape: a scalar
/// advances one node, a nested container skips its whole subtree in O(1) via its
/// head's back-patched `next`.
pub struct RespArrayIter<'a> {
    buf: &'a [u8],
    tape: &'a [u8],
    cursor: usize,
    remaining: usize,
}

impl<'a> RespArrayIter<'a> {
    #[inline(always)]
    pub fn new(buf: &'a [u8], tape: &'a [u8], root: usize, len: usize) -> Self {
        Self {
            buf,
            tape,
            cursor: root + 2,
            remaining: len,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.remaining
    }

    #[inline(always)]
    pub fn has_next(&self) -> bool {
        self.remaining > 0
    }
}

impl<'a> Iterator for RespArrayIter<'a> {
    type Item = Result<RespView<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let node = read_node(self.tape, self.cursor);
        let tag = node_tag(node);

        if is_container_tag(tag) {
            let root = self.cursor;
            self.cursor = node_payload(node) as usize;
            self.remaining -= 1;
            Some(Ok(container_view(tag, self.buf, self.tape, root)))
        } else {
            let off = node_payload(node) as usize;
            // A `None` here means the already-validated tape is inconsistent
            // (effectively unreachable). Surface it as an error and end the
            // iterator instead of silently truncating the array: the consumer
            // must be able to tell "the array ended" from "an element could not
            // be read".
            match read_scalar_view(tag, self.buf, off) {
                Some(view) => {
                    self.cursor += 1;
                    self.remaining -= 1;
                    Some(Ok(view))
                }
                None => {
                    self.remaining = 0;
                    Some(Err(Error::Client(ClientError::Unexpected)))
                }
            }
        }
    }
}

/// Owned equivalent of [`RespArrayIter`], yielding a self-contained
/// [`RespResponse`] per element (used by cluster aggregation and the cache,
/// which retain elements past the borrow of the parent response).
pub struct RespResponseIter {
    buf: RespBuf,
    tape: Bytes,
    cursor: usize,
    remaining: usize,
}

impl RespResponseIter {
    pub fn new(buf: RespBuf, tape: Bytes, root: usize, len: usize) -> Self {
        Self {
            buf,
            tape,
            cursor: root + 2,
            remaining: len,
        }
    }
}

impl Iterator for RespResponseIter {
    type Item = Result<RespResponse>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let node = read_node(&self.tape, self.cursor);
        let tag = node_tag(node);

        if is_container_tag(tag) {
            let root = self.cursor;
            self.cursor = node_payload(node) as usize;
            self.remaining -= 1;
            let tape = self.tape.clone();
            let root = root as u32;
            #[expect(
                clippy::unreachable,
                reason = "invariant: guarded by `is_container_tag` just above, \
                          whose `matches!` lists exactly these four tags."
            )]
            let frame = match tag {
                ARRAY_TAG => RespFrame::Array { tape, root },
                MAP_TAG => RespFrame::Map { tape, root },
                SET_TAG => RespFrame::Set { tape, root },
                PUSH_TAG => RespFrame::Push { tape, root },
                _ => unreachable!("is_container_tag matched a non-container tag"),
            };
            Some(Ok(RespResponse::Frame(self.buf.clone(), frame)))
        } else {
            let off = node_payload(node) as usize;
            // Same surface-and-stop handling as `RespArrayIter`: an inconsistent
            // tape yields an error, not a silent truncation.
            match read_scalar_frame(tag, self.buf.as_ref(), off) {
                Some(frame) => {
                    self.cursor += 1;
                    self.remaining -= 1;
                    Some(Ok(RespResponse::Frame(self.buf.clone(), frame)))
                }
                None => {
                    self.remaining = 0;
                    Some(Err(Error::Client(ClientError::Unexpected)))
                }
            }
        }
    }
}
