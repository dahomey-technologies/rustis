use crate::{
    Result,
    resp::{
        ARRAY_TAG, BULK_ERROR_TAG, BULK_STRING_TAG, DOUBLE_TAG, INTEGER_TAG, MAP_TAG, PUSH_TAG,
        RespCollectionView, RespResponse, RespView, SET_TAG, SIMPLE_ERROR_TAG, SIMPLE_STRING_TAG,
    },
};
use std::{fmt, io::Write as _};

/// One reply in its [RESP](https://redis.io/docs/reference/protocol-spec/) form:
/// the tag byte, the value and the terminating `CRLF`, exactly as a server
/// writes them.
///
/// This is what a caller reads a reply below the serde layer with: a proxy
/// forwarding replies to another connection, a bridge to another protocol, or a
/// reader of a shape no Rust type models. Everything else is better served by
/// [`Client::send`](crate::client::Client::send), which reads the reply into the
/// type the caller declares, or by [`Value`](crate::resp::Value), which is the
/// same reply as a tree.
///
/// The bytes are owned. A reply borrowed from the network read buffer would pin
/// the whole block it was split from — a block the connection recycles across
/// replies — for as long as the caller held it, so they are copied out, as
/// [`PubSubMessage`](crate::client::PubSubMessage) copies a message out.
///
/// # What is verbatim and what is rewritten
///
/// A reply that reached the client as one frame is handed back byte for byte,
/// which is every reply of a standalone connection and every reply a cluster
/// took from a single node.
///
/// The client builds the rest itself and writes them as RESP3: the reply of a
/// command split over several cluster nodes, whose sub-replies are aggregated;
/// a value the client-side cache decoded when it retained it; and a null
/// collection (`*-1`), which carries nothing to keep and comes back as `_`. A
/// rewritten reply says what the client read, which is not always the tag the
/// server wrote: a verbatim string (`=`) and a big number (`(`) are read as
/// strings, here as everywhere else in the crate, so they are written back as
/// bulk strings.
#[derive(Clone, PartialEq, Eq)]
pub struct RawResponse(Box<[u8]>);

/// Above this many bytes, the rendering stops and is marked as truncated: a
/// multi-megabyte reply would otherwise build a multi-megabyte log line.
const DEBUG_RENDER_LIMIT: usize = 1000;

impl RawResponse {
    /// The reply's RESP bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The reply's RESP bytes, owned.
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.0.into_vec()
    }

    /// `true` when the reply is a Redis error.
    ///
    /// [`send_raw`](crate::client::Client::send_raw) hands an error reply back
    /// rather than raising it, so a caller that does not forward it verbatim
    /// asks here.
    #[inline]
    pub fn is_error(&self) -> bool {
        matches!(self.0.first(), Some(&SIMPLE_ERROR_TAG | &BULK_ERROR_TAG))
    }

    /// Copies `bytes` into a reply of their own.
    #[inline]
    pub(crate) fn from_slice(bytes: &[u8]) -> RawResponse {
        RawResponse(Box::from(bytes))
    }

    /// Takes `bytes` as the reply, without copying them again.
    #[inline]
    pub(crate) fn from_vec(bytes: Vec<u8>) -> RawResponse {
        RawResponse(bytes.into_boxed_slice())
    }
}

/// Renders the reply as text, since RESP is text with byte payloads. Bytes that
/// are not UTF-8 are replaced rather than escaped, as a payload is rendered
/// everywhere else in the crate.
impl fmt::Debug for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown = self.0.get(..DEBUG_RENDER_LIMIT).unwrap_or(&self.0);
        write!(f, "{:?}", String::from_utf8_lossy(shown))?;
        if shown.len() < self.0.len() {
            f.write_str("<truncated>")?;
        }
        Ok(())
    }
}

/// Writes `response` as RESP bytes, verbatim when it holds its own frame.
pub(crate) fn write_response(response: &RespResponse, out: &mut Vec<u8>) -> Result<()> {
    if let Some(frame) = response.wire_bytes() {
        out.extend_from_slice(frame);
        return Ok(());
    }
    write_view(&response.view()?, out)
}

/// Writes what a reply reads as, one element at a time.
fn write_view(view: &RespView<'_>, out: &mut Vec<u8>) -> Result<()> {
    match view {
        RespView::Array(elements) => return write_collection(ARRAY_TAG, elements, out),
        RespView::Set(elements) => return write_collection(SET_TAG, elements, out),
        RespView::Push(elements) => return write_collection(PUSH_TAG, elements, out),
        // A map's cardinality is its pair count, where the view holds keys and
        // values flattened — the pairing the parser doubled on the way in.
        RespView::Map(elements) => {
            write_header(MAP_TAG, elements.len() / 2, out);
            return write_elements(elements, out);
        }
        RespView::OwnedArray(elements) => {
            write_header(ARRAY_TAG, elements.len(), out);
            for element in *elements {
                write_response(element, out)?;
            }
            return Ok(());
        }
        RespView::SimpleString(value) => write_line(SIMPLE_STRING_TAG, value, out),
        RespView::Error(message) => write_line(SIMPLE_ERROR_TAG, message, out),
        // The wire text, when there is one: a server's spelling of a number is
        // not the one Rust renders, and a caller reading the reply as a string
        // is owed the spelling it arrived with. A synthesized integer has none,
        // and a decimal integer has a single spelling anyway.
        RespView::Integer(value, text) => {
            if text.is_empty() {
                let mut buffer = itoa::Buffer::new();
                write_line(INTEGER_TAG, buffer.format(*value).as_bytes(), out);
            } else {
                write_line(INTEGER_TAG, text, out);
            }
        }
        RespView::Double(value, text) => {
            if text.is_empty() {
                out.push(DOUBLE_TAG);
                // Never taken by a reply of this crate — a double is decoded
                // from a wire text and keeps it — so the rendering is the
                // fallback rather than the rule.
                let _ = write!(out, "{value}");
                out.extend_from_slice(b"\r\n");
            } else {
                write_line(DOUBLE_TAG, text, out);
            }
        }
        RespView::BulkString(value) => {
            write_header(BULK_STRING_TAG, value.len(), out);
            write_line_body(value, out);
        }
        RespView::Boolean(value) => {
            out.extend_from_slice(if *value { b"#t\r\n" } else { b"#f\r\n" })
        }
        RespView::Null => out.extend_from_slice(b"_\r\n"),
        RespView::IntegerArray(values) => {
            write_header(ARRAY_TAG, values.len(), out);
            let mut buffer = itoa::Buffer::new();
            for value in *values {
                write_line(INTEGER_TAG, buffer.format(*value).as_bytes(), out);
            }
        }
    }
    Ok(())
}

/// Writes a collection's header and then its elements.
fn write_collection(tag: u8, elements: &RespCollectionView<'_>, out: &mut Vec<u8>) -> Result<()> {
    write_header(tag, elements.len(), out);
    write_elements(elements, out)
}

/// Writes the elements of a collection, at whatever depth they nest.
fn write_elements(elements: &RespCollectionView<'_>, out: &mut Vec<u8>) -> Result<()> {
    for element in elements.clone() {
        write_view(&element?, out)?;
    }
    Ok(())
}

/// Writes `tag`, `count` and a `CRLF`: a collection's header, and a bulk
/// string's.
fn write_header(tag: u8, count: usize, out: &mut Vec<u8>) {
    let mut buffer = itoa::Buffer::new();
    out.push(tag);
    out.extend_from_slice(buffer.format(count).as_bytes());
    out.extend_from_slice(b"\r\n");
}

/// Writes a whole element that is `tag` followed by `value`.
fn write_line(tag: u8, value: &[u8], out: &mut Vec<u8>) {
    out.push(tag);
    write_line_body(value, out);
}

/// Writes `value` and the `CRLF` closing it.
fn write_line_body(value: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(value);
    out.extend_from_slice(b"\r\n");
}
