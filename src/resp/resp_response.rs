use crate::{
    ClientError, Error, RedisError, Result,
    resp::{
        ARRAY_TAG, ElementKind, MAP_TAG, NULL_TAG, PUSH_TAG, RespBuf, RespDeserializer, SET_TAG,
        element_bounds, is_container_tag, node_payload, node_tag, read_node,
    },
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::{fmt, ops::Range};

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
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum RespResponse {
    IntegerArray(Vec<i64>),
    OwnedArray(Vec<RespResponse>),
    Frame(RespBuf, RespFrame),
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
    let bounds = element_bounds(data, off).ok()?;
    Some(match bounds.kind {
        ElementKind::SimpleString => RespView::SimpleString(&data[bounds.value]),
        ElementKind::Error => RespView::Error(&data[bounds.value]),
        ElementKind::Integer => RespView::Integer(atoi::atoi(&data[bounds.value])?),
        ElementKind::Double => RespView::Double(fast_float2::parse(&data[bounds.value]).ok()?),
        ElementKind::BulkString => RespView::BulkString(&data[bounds.value]),
        ElementKind::Boolean => RespView::Boolean(data[bounds.value.start] == b't'),
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
    let bounds = element_bounds(data, off).ok()?;
    Some(match bounds.kind {
        ElementKind::SimpleString => RespFrame::SimpleString(bounds.value),
        ElementKind::Error => RespFrame::Error(bounds.value),
        ElementKind::Integer => RespFrame::Integer(atoi::atoi(&data[bounds.value])?),
        ElementKind::Double => RespFrame::Double(fast_float2::parse(&data[bounds.value]).ok()?),
        ElementKind::BulkString => RespFrame::BulkString(bounds.value),
        ElementKind::Boolean => RespFrame::Boolean(data[bounds.value.start] == b't'),
        ElementKind::Null => RespFrame::Null,
    })
}

/// Builds the borrowed collection view for a container node at tape index `root`.
#[inline]
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
            Self::Map(arg0) => f.debug_tuple("Map").field(arg0).finish(),
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

/// A borrowed view over a collection: the data buffer, the parse tape, and the
/// tape index of the container's head node. `len` (the exact element count) is
/// read once from the head's companion node, so it is O(1).
#[derive(Debug, Clone, PartialEq)]
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

impl<'a> IntoIterator for RespArrayView<'a> {
    type Item = RespView<'a>;
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
    type Item = RespView<'a>;

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
            Some(container_view(tag, self.buf, self.tape, root))
        } else {
            let off = node_payload(node) as usize;
            let view = read_scalar_view(tag, self.buf, off)?;
            self.cursor += 1;
            self.remaining -= 1;
            Some(view)
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
    type Item = RespResponse;

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
            let frame = match tag {
                ARRAY_TAG => RespFrame::Array { tape, root },
                MAP_TAG => RespFrame::Map { tape, root },
                SET_TAG => RespFrame::Set { tape, root },
                PUSH_TAG => RespFrame::Push { tape, root },
                _ => unreachable!("is_container_tag matched a non-container tag"),
            };
            Some(RespResponse::Frame(self.buf.clone(), frame))
        } else {
            let off = node_payload(node) as usize;
            let frame = read_scalar_frame(tag, self.buf.as_ref(), off)?;
            self.cursor += 1;
            self.remaining -= 1;
            Some(RespResponse::Frame(self.buf.clone(), frame))
        }
    }
}
