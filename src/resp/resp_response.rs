use crate::{
    ClientError, Error, RedisError, Result,
    resp::{
        ARRAY_TAG, BULK_ERROR_TAG, MAP_TAG, NULL_TAG, PUSH_TAG, ParsedFrame, RespBuf,
        RespDeserializer, RespTape, SET_TAG, SIMPLE_ERROR_TAG, SIMPLE_STRING_TAG, ScalarKind,
        TapeNode, frame_scalar_bounds, scalar_span, scalar_value,
    },
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::{
    fmt::{self, Write as _},
    ops::Range,
};

/// A decoded RESP reply, either as it came off the wire or synthesized by the
/// cluster and cache layers.
///
/// The wire form keeps the frame's bytes undecoded and reads a value only when
/// the caller asks for one, so the decode runs in the calling task rather than
/// in the connection's shared network task.
#[derive(Clone, PartialEq)]
pub enum RespResponse {
    Null,
    Integer(i64),
    Double(f64),
    IntegerArray(Vec<i64>),
    OwnedArray(Vec<RespResponse>),
    /// A frame's bytes plus the flat parse **tape** indexing it — one
    /// fixed-width node per element, all nesting levels — with `root` the tape
    /// index of the collection's head node, so reading an element is an O(1) node
    /// lookup instead of a re-parse. See [`crate::resp::resp_tape`].
    ///
    /// An **empty tape** means the frame is a lone scalar, which no node would
    /// help to index. `buf` is then exactly that scalar's RESP bytes, tag byte
    /// first and terminating `\r\n` last — every producer slices to it, and the
    /// read path relies on it to locate the value without scanning for the
    /// terminator.
    Frame {
        buf: RespBuf,
        tape: RespTape,
        root: u32,
    },
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
            match self.view() {
                Ok(view) => write!(writer, "{view:?}")?,
                Err(e) => write!(writer, "{:?}", UnreadableElement(e))?,
            }
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
    /// Pairs a frame's bytes with what one parse pass recovered from them.
    #[inline(always)]
    pub fn new(buf: RespBuf, parsed: ParsedFrame) -> Self {
        match parsed {
            ParsedFrame::Scalar { at } => Self::Frame {
                // A frame opening on an attribute leaves the scalar past `at`;
                // slicing to it drops the metadata nobody reads and leaves `buf`
                // as the scalar's own bytes.
                buf: if at == 0 {
                    buf
                } else {
                    RespBuf::from(buf.slice(at..))
                },
                tape: RespTape::default(),
                root: 0,
            },
            ParsedFrame::Collection(tape) => Self::Frame { buf, tape, root: 0 },
            // A null collection carries nothing to read back, so its bytes go.
            ParsedFrame::Null => Self::Null,
        }
    }

    /// Reads this response as a borrowed view, decoding a scalar's bytes on the
    /// way. Fails when those bytes are not a value of the kind their tag
    /// announces — a malformed numeric, which framing accepted and only the read
    /// can catch.
    #[inline(always)]
    pub fn view(&self) -> Result<RespView<'_>> {
        match self {
            RespResponse::Null => Ok(RespView::Null),
            // Synthesized: no wire bytes to hand back, so a string rendering has
            // to be built from the value.
            RespResponse::Integer(i) => Ok(RespView::Integer(*i, b"")),
            RespResponse::Double(d) => Ok(RespView::Double(*d, b"")),
            RespResponse::IntegerArray(a) => Ok(RespView::IntegerArray(a)),
            RespResponse::OwnedArray(a) => Ok(RespView::OwnedArray(a)),
            RespResponse::Frame { buf, tape, root } => view_at(buf.as_ref(), tape, *root as usize),
        }
    }

    /// The RESP tag of the value this response points at, for the callers that
    /// only need to classify a reply. `None` for a synthesized response, which
    /// never came off the wire and has no tag.
    #[inline(always)]
    fn frame_tag(&self) -> Option<u8> {
        match self {
            RespResponse::Frame { buf, tape, root } => {
                if tape.is_empty() {
                    buf.first().copied()
                } else {
                    Some(tape.node(*root as usize).tag())
                }
            }
            _ => None,
        }
    }

    /// Returns `true` if the RESP Response is a push message
    #[inline(always)]
    pub fn is_push(&self) -> bool {
        self.frame_tag() == Some(PUSH_TAG)
    }

    /// Returns `true` if the RESP Response is a monitor message
    ///
    /// A monitor line is a simple string opening on the event's timestamp, which
    /// is what tells it apart from any other simple-string reply.
    #[inline(always)]
    pub fn is_monitor(&self) -> bool {
        match self {
            RespResponse::Frame { buf, tape, .. } if tape.is_empty() => {
                matches!(buf.as_ref(), [SIMPLE_STRING_TAG, second, ..] if second.is_ascii_digit())
            }
            _ => false,
        }
    }

    /// Returns `true` if the RESP Response is a Redis error
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        matches!(self.frame_tag(), Some(SIMPLE_ERROR_TAG | BULK_ERROR_TAG))
    }

    #[inline(always)]
    pub fn null() -> RespResponse {
        Self::Null
    }

    #[inline(always)]
    pub fn integer(i: i64) -> RespResponse {
        Self::Integer(i)
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
        Self::Frame {
            buf: RespBuf::from(Bytes::from_static(b"+OK\r\n")),
            tape: RespTape::default(),
            root: 0,
        }
    }

    /// Convert the RESP Response to a Rust type `T` by using serde deserialization
    #[inline]
    pub fn to<T: DeserializeOwned>(&self) -> Result<T> {
        T::deserialize(RespDeserializer::new(self.view()?))
    }

    /// Returns a self-contained copy that holds **only** what this response
    /// needs, releasing any larger shared block it was carved from.
    ///
    /// A response kept alive long after decoding (a cache entry, a buffered
    /// stream item) pins the whole recycled block its data — and, for a
    /// collection, its tape — was split from: a 50-byte cached value can pin a
    /// 64 KiB block. Compacting before retaining copies out what is referenced so
    /// the block can be reclaimed. Callers that consume a response promptly (the
    /// normal request/response path) never need this.
    ///
    /// A numeric scalar is decoded here rather than copied, because a retained
    /// response is read repeatedly — a cache entry hit a thousand times would
    /// otherwise decode a thousand times. Strings keep their RESP header: the
    /// read path recovers the value from the tag, so the bytes have to stay a
    /// readable element.
    pub fn compact(&self) -> RespResponse {
        match self {
            RespResponse::Null => RespResponse::Null,
            RespResponse::Integer(i) => RespResponse::Integer(*i),
            RespResponse::Double(d) => RespResponse::Double(*d),
            RespResponse::IntegerArray(a) => RespResponse::IntegerArray(a.clone()),
            RespResponse::OwnedArray(a) => {
                RespResponse::OwnedArray(a.iter().map(RespResponse::compact).collect())
            }
            RespResponse::Frame { buf, tape, .. } if tape.is_empty() => {
                let data = buf.as_ref();
                match read_frame_view(data) {
                    Ok(RespView::Integer(i, _)) => RespResponse::Integer(i),
                    Ok(RespView::Double(d, _)) => RespResponse::Double(d),
                    Ok(RespView::Null) => RespResponse::Null,
                    // Anything else copies its bytes out as they are. A scalar
                    // that does not read back keeps them too, so the failure
                    // survives compaction instead of becoming another one.
                    _ => RespResponse::Frame {
                        buf: RespBuf::from(Bytes::copy_from_slice(data)),
                        tape: RespTape::default(),
                        root: 0,
                    },
                }
            }
            RespResponse::Frame { buf, tape, root } => RespResponse::Frame {
                buf: RespBuf::from(Bytes::copy_from_slice(buf.as_ref())),
                tape: tape.compact(),
                root: *root,
            },
        }
    }

    /// Walks a collection's elements as owned responses, in wire order.
    ///
    /// All four collection tags are accepted: a map yields its keys and values
    /// flattened, a push yields its kind as the first element. An error reply is
    /// surfaced as the Redis error itself, so a caller cannot mistake a failure
    /// for an empty reply.
    pub fn into_collection_iter(self) -> Result<RespResponseIter> {
        // `is_error` is a tag check, so a non-error reply does not pay for a view.
        if self.is_error()
            && let Ok(RespView::Error(message)) = self.view()
        {
            return Err(Error::Redis(RedisError::try_from(message)?));
        }
        match self {
            RespResponse::Frame { buf, tape, root }
                if !tape.is_empty() && tape.node(root as usize).is_collection() =>
            {
                let root = root as usize;
                let len = tape.node(root + 1).payload() as usize;
                Ok(RespResponseIter::new(buf, tape, root, len))
            }
            _ => Err(Error::Client(ClientError::Unexpected)),
        }
    }
}

/// Reads the value `root` points at: a tape node when the frame has a tape, the
/// byte offset of a lone scalar when it does not.
#[inline]
fn view_at<'a>(buf: &'a [u8], tape: &'a RespTape, root: usize) -> Result<RespView<'a>> {
    if tape.is_empty() {
        return read_frame_view(buf);
    }
    let node = tape.node(root);
    if node.is_collection() {
        Ok(collection_view(node.tag(), buf, tape, root))
    } else {
        read_node_view(node, buf)
    }
}

/// Reads the lone scalar a frame consists of, `data` being its own bytes.
///
/// Nothing is scanned to find the terminator — see [`frame_scalar_bounds`]. That
/// is what makes reading on demand about as cheap as decoding eagerly was.
#[inline]
fn read_frame_view(data: &[u8]) -> Result<RespView<'_>> {
    let (kind, value) = frame_scalar_bounds(data)?;
    decode_value(kind, data, value)
}

/// Reads the scalar a non-collection tape node points at.
///
/// A [`NULL_TAG`] node is `Null` without touching the data buffer: it stands in
/// for a null child collection (`*-1`), whose offset points at `*`, not at a
/// scalar.
#[inline]
fn read_node_view<'a>(node: TapeNode, data: &'a [u8]) -> Result<RespView<'a>> {
    if node.tag() == NULL_TAG {
        return Ok(RespView::Null);
    }
    read_scalar_view(data, node.payload() as usize)
}

/// Decodes the scalar element whose tag byte is at `off`, whose end is unknown
/// and has to be re-derived — the case of an element sitting inside a collection,
/// where the surrounding bytes belong to its siblings.
#[inline]
fn read_scalar_view(data: &[u8], off: usize) -> Result<RespView<'_>> {
    let (kind, value) = scalar_value(data, off)?;
    decode_value(kind, data, value)
}

/// Turns a scalar's bytes into a value.
///
/// This is where a number is actually parsed, deliberately: framing only needed
/// the `\r\n`, so the arithmetic lands in whichever task asks for the value
/// rather than in the connection's shared network task. Bytes that do not read
/// back as the kind their tag announces fail here, failing one command instead of
/// the whole connection.
#[inline]
fn decode_value(kind: ScalarKind, data: &[u8], value: Range<usize>) -> Result<RespView<'_>> {
    // `ok_or_else`, not `ok_or`: this runs per element, and `Error` is large
    // enough that constructing one eagerly only to drop it costs measurably.
    let value = data
        .get(value)
        .ok_or_else(|| Error::Client(ClientError::Unexpected))?;
    Ok(match kind {
        ScalarKind::SimpleString => RespView::SimpleString(value),
        ScalarKind::Error => RespView::Error(value),
        ScalarKind::Integer => RespView::Integer(
            atoi::atoi(value).ok_or_else(|| Error::Client(ClientError::CannotParseInteger))?,
            value,
        ),
        ScalarKind::Double => RespView::Double(
            fast_float2::parse(value).map_err(|_| Error::Client(ClientError::CannotParseDouble))?,
            value,
        ),
        ScalarKind::BulkString => RespView::BulkString(value),
        // The framing pass already rejected anything but `t` and `f`.
        ScalarKind::Boolean => RespView::Boolean(value.first() == Some(&b't')),
        ScalarKind::Null => RespView::Null,
    })
}

/// Builds the borrowed view of the collection whose head node is at tape index
/// `root`.
#[inline]
#[expect(
    clippy::unreachable,
    reason = "invariant: callers gate on `is_collection_tag`, whose `matches!` \
              lists exactly these four tags. The arm asserts that pairing; a \
              fallback would have to invent a view for a tag that is not a \
              collection."
)]
fn collection_view<'a>(tag: u8, buf: &'a [u8], tape: &'a RespTape, root: usize) -> RespView<'a> {
    let view = RespCollectionView::new(buf, tape, root);
    match tag {
        ARRAY_TAG => RespView::Array(view),
        MAP_TAG => RespView::Map(view),
        SET_TAG => RespView::Set(view),
        PUSH_TAG => RespView::Push(view),
        _ => unreachable!("collection_view called with a non-collection tag"),
    }
}

/// A borrowed, decoded view of one reply.
///
/// The two numeric variants carry both their decoded value and the bytes the
/// server sent, because the two are not interchangeable: a caller asking for a
/// number wants the value, and a caller asking for a string wants the text
/// Redis chose. Re-rendering the value does not round-trip — `,12.50` would come
/// back as `12.5` and `,1e21` as twenty-two digits — so the text is kept rather
/// than reconstructed. The bytes are empty for a reply the client synthesized,
/// which never came off the wire.
#[derive(PartialEq)]
pub enum RespView<'a> {
    SimpleString(&'a [u8]),
    Integer(i64, &'a [u8]),
    Double(f64, &'a [u8]),
    BulkString(&'a [u8]),
    Boolean(bool),
    IntegerArray(&'a [i64]),
    OwnedArray(&'a [RespResponse]),
    Array(RespCollectionView<'a>),
    Map(RespCollectionView<'a>),
    Set(RespCollectionView<'a>),
    Push(RespCollectionView<'a>),
    Error(&'a [u8]),
    Null,
}

impl<'a> fmt::Debug for RespView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f
                .debug_tuple("SimpleString")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            // The value, not the wire bytes: the two say the same thing, and the
            // value is the readable one.
            Self::Integer(arg0, _) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0, _) => f.debug_tuple("Double").field(arg0).finish(),
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
fn fmt_pairs(f: &mut fmt::Formatter<'_>, view: &RespCollectionView<'_>) -> fmt::Result {
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
/// tape index of the collection's head node. `len` (the exact element count) is
/// read once from the head's companion node, so it is O(1).
#[derive(Clone, PartialEq)]
pub struct RespCollectionView<'a> {
    buf: &'a [u8],
    tape: &'a RespTape,
    root: usize,
    len: usize,
}

impl<'a> RespCollectionView<'a> {
    #[inline(always)]
    pub fn new(buf: &'a [u8], tape: &'a RespTape, root: usize) -> Self {
        let len = tape.node(root + 1).payload() as usize;
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
impl fmt::Debug for RespCollectionView<'_> {
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

impl<'a> IntoIterator for RespCollectionView<'a> {
    type Item = Result<RespView<'a>>;
    type IntoIter = RespCollectionIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RespCollectionIter::new(self.buf, self.tape, self.root, self.len)
    }
}

/// Walks the direct children of a collection by stepping the tape: a scalar
/// advances one node, a nested collection skips its whole subtree in O(1) via its
/// head's back-patched `next`.
pub struct RespCollectionIter<'a> {
    buf: &'a [u8],
    tape: &'a RespTape,
    cursor: usize,
    remaining: usize,
}

impl<'a> RespCollectionIter<'a> {
    #[inline(always)]
    pub fn new(buf: &'a [u8], tape: &'a RespTape, root: usize, len: usize) -> Self {
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

impl<'a> Iterator for RespCollectionIter<'a> {
    type Item = Result<RespView<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let node = self.tape.node(self.cursor);
        let tag = node.tag();

        if node.is_collection() {
            let root = self.cursor;
            self.cursor = node.payload() as usize;
            self.remaining -= 1;
            Some(Ok(collection_view(tag, self.buf, self.tape, root)))
        } else {
            // Surface a read failure as an error and end the iterator rather
            // than truncating the array silently: the consumer must be able to
            // tell "the array ended" from "an element could not be read".
            match read_node_view(node, self.buf) {
                Ok(view) => {
                    self.cursor += 1;
                    self.remaining -= 1;
                    Some(Ok(view))
                }
                Err(e) => {
                    self.remaining = 0;
                    Some(Err(e))
                }
            }
        }
    }
}

/// Owned equivalent of [`RespCollectionIter`], yielding a self-contained
/// [`RespResponse`] per element (used by cluster aggregation and the cache,
/// which retain elements past the borrow of the parent response).
pub struct RespResponseIter {
    buf: RespBuf,
    tape: RespTape,
    cursor: usize,
    remaining: usize,
}

impl RespResponseIter {
    pub fn new(buf: RespBuf, tape: RespTape, root: usize, len: usize) -> Self {
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

        let node = self.tape.node(self.cursor);

        // A collection element is handed out over the same buffer and tape,
        // re-rooted on its own node: no byte is copied and no value decoded, and
        // it reads back exactly as it would through the parent.
        if node.is_collection() {
            let root = self.cursor;
            self.cursor = node.payload() as usize;
            self.remaining -= 1;
            // A tape node index is far below `u32::MAX` for any reply a server
            // can send, but the tape's own payload is wider, so the conversion is
            // checked rather than assumed: a truncated root would read a
            // different element, silently.
            let Ok(root) = u32::try_from(root) else {
                self.remaining = 0;
                return Some(Err(Error::Client(ClientError::Unexpected)));
            };
            return Some(Ok(RespResponse::Frame {
                buf: self.buf.clone(),
                tape: self.tape.clone(),
                root,
            }));
        }

        self.cursor += 1;
        self.remaining -= 1;
        // A null child collection points at its `*`, which is not a scalar to
        // read back; the tape node is all there is.
        if node.tag() == NULL_TAG {
            return Some(Ok(RespResponse::Null));
        }
        // A scalar element is handed out over its own bytes: the buffer is sliced
        // down to the element — a refcount bump, not a copy — so the response
        // holds to the invariant that a tapeless frame ends where its scalar does.
        let at = node.payload() as usize;
        let data = self.buf.as_ref();
        match scalar_span(data, at) {
            Ok(span) => Some(Ok(RespResponse::Frame {
                buf: RespBuf::from(self.buf.slice(span)),
                tape: RespTape::default(),
                root: 0,
            })),
            // Surface a failure rather than truncating the iteration silently: the
            // consumer must be able to tell "the array ended" from "an element
            // could not be read".
            Err(e) => {
                self.remaining = 0;
                Some(Err(e))
            }
        }
    }
}
