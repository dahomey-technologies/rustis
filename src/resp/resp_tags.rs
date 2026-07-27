//! The RESP tag alphabet: the byte that opens every frame on the wire.
//!
//! A tag says how to read what follows, so every layer that touches raw RESP
//! bytes needs the same 15 constants — the frame parser to dispatch on them, the
//! tape to tell a collection head from a scalar node, the response reader to
//! classify a reply. They live in their own module so no two of those layers
//! have to reach into each other for the alphabet they share.
//!
//! RESP2 uses `+ - : $ *`; RESP3 adds the nine others. A RESP2 connection simply
//! never receives them.

/// `+` — a single line of text, terminated by `\r\n`.
pub(crate) const SIMPLE_STRING_TAG: u8 = b'+';
/// `-` — an error as a single line of text.
pub(crate) const SIMPLE_ERROR_TAG: u8 = b'-';
/// `:` — a 64-bit signed integer, in decimal.
pub(crate) const INTEGER_TAG: u8 = b':';
/// `$` — a length-prefixed byte string; `$-1\r\n` is the null bulk string.
pub(crate) const BULK_STRING_TAG: u8 = b'$';
/// `*` — an ordered collection; `*-1\r\n` is the null array.
pub(crate) const ARRAY_TAG: u8 = b'*';
/// `_` — the RESP3 null.
pub(crate) const NULL_TAG: u8 = b'_';
/// `#` — a boolean, `t` or `f`.
pub(crate) const BOOL_TAG: u8 = b'#';
/// `,` — a double, including `inf`, `-inf` and `nan`.
pub(crate) const DOUBLE_TAG: u8 = b',';
/// `!` — an error as a length-prefixed byte string.
pub(crate) const BULK_ERROR_TAG: u8 = b'!';
/// `=` — a length-prefixed string whose first four bytes give its format.
pub(crate) const VERBATIM_STRING_TAG: u8 = b'=';
/// `%` — a collection of key/value pairs, announcing pair count, not element
/// count.
pub(crate) const MAP_TAG: u8 = b'%';
/// `~` — an unordered collection.
pub(crate) const SET_TAG: u8 = b'~';
/// `>` — an out-of-band message, pushed by the server outside any reply.
pub(crate) const PUSH_TAG: u8 = b'>';
/// `|` — out-of-band metadata that may precede any value and is never surfaced.
/// Shaped like a map, but not a value of its own.
pub(crate) const ATTRIBUTE_TAG: u8 = b'|';
/// `(` — an arbitrary-precision integer, carried as its decimal text.
pub(crate) const BIG_NUMBER_TAG: u8 = b'(';

/// `true` if `tag` opens a collection: an array, map, set or push. Every other
/// tag opens a scalar — `|` excepted, which opens no value at all.
///
/// A free function rather than a [`TapeNode`](super::TapeNode) method because
/// the parser applies it to a tag byte read straight from the wire, before any
/// node exists.
#[inline(always)]
pub(crate) fn is_collection_tag(tag: u8) -> bool {
    matches!(tag, ARRAY_TAG | MAP_TAG | SET_TAG | PUSH_TAG)
}
