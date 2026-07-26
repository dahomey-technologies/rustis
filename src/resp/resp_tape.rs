//! Flat, fixed-width parse tape for RESP collections.
//!
//! A collection reply is parsed once into a sequence of fixed-width nodes stored
//! in a recycled `BytesMut`. Reading an element is then an O(1) node lookup plus
//! a content-length read of its own bytes, instead of re-parsing the RESP
//! structure from the start.
//!
//! # Node layout — 8 bytes, little-endian `u64`
//!
//! The high byte holds a tag; the low 56 bits hold a payload. There are two node
//! kinds, distinguished by the tag:
//!
//! - **Scalar node** (tag = the RESP tag byte `+ - : $ , # _ ( = !`): payload is
//!   the element's **start offset**, frame-relative, pointing at its own tag byte
//!   in the data buffer. Its value is recovered by re-reading from that offset
//!   ([`element_bounds`](super::element_bounds)), a content-length read that
//!   touches only this element — never a structural re-parse of the collection.
//! - **Container node** (tag = `* % ~ >`): a container occupies **two**
//!   consecutive nodes. The *head* carries `next` = the tape index one past the
//!   whole container (all descendants included), giving an O(1) skip to the next
//!   sibling. The node right after the head carries `len` (tag [`TAPE_LEN_TAG`]) =
//!   the element count, giving O(1) size hints and RESP2 struct detection. The
//!   container's children start at `head + 2`.
//!
//! A null collection (`*-1`) parsed as an element emits a single [`NULL_TAG`]
//! scalar node, so it is counted and deserializes to `Null` like any other.
//!
//! # Offsets are frame-relative — non-negotiable
//!
//! Every offset is relative to the frame slice ([`RespBuf`](super::RespBuf)),
//! never to the shared read buffer. `BytesMut` moves bytes to a new base address
//! on realloc/reclaim, so a buffer-absolute offset would silently address the
//! wrong bytes on the next reused frame.

use crate::resp::{ARRAY_TAG, MAP_TAG, PUSH_TAG, SET_TAG};
use bytes::{BufMut, BytesMut};

/// Width of one tape node, in bytes. A power of two so `tape[i]` is
/// `base + (i << 3)` and every node is naturally aligned for a `u64` load.
pub(crate) const TAPE_NODE_SIZE: usize = 8;

/// Mask selecting the 56-bit payload of a node (the low 7 bytes).
const PAYLOAD_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;

/// The largest value a node payload can hold. Offsets (bounded by the frame
/// length) and tape indices stay far below this in any real reply.
pub(crate) const MAX_TAPE_PAYLOAD: u64 = PAYLOAD_MASK;

/// Tag of the second node of a container pair, whose payload is the container's
/// element count. Never used for dispatch (the node is reached positionally, at
/// `head + 1`), so any value outside the RESP tag set is fine; `0` is chosen so
/// a stray read of it is obviously not a real element.
pub(crate) const TAPE_LEN_TAG: u8 = 0;

/// Packs a `(tag, payload)` pair into a node word.
#[inline(always)]
pub(crate) fn encode(tag: u8, payload: u64) -> u64 {
    debug_assert!(
        payload <= MAX_TAPE_PAYLOAD,
        "tape payload overflows 56 bits"
    );
    ((tag as u64) << 56) | (payload & PAYLOAD_MASK)
}

/// Returns the tag byte of a node word.
#[inline(always)]
pub(crate) fn node_tag(node: u64) -> u8 {
    (node >> 56) as u8
}

/// Returns the 56-bit payload of a node word.
#[inline(always)]
pub(crate) fn node_payload(node: u64) -> u64 {
    node & PAYLOAD_MASK
}

/// `true` if `tag` marks a container head (array, map, set or push). Every other
/// emitted tag marks a scalar node.
#[inline(always)]
pub(crate) fn is_container_tag(tag: u8) -> bool {
    matches!(tag, ARRAY_TAG | MAP_TAG | SET_TAG | PUSH_TAG)
}

/// Reads node `index` from a frozen tape slice. `index` must be in bounds
/// (`< tape.len() / 8`); the tape is produced by the parser, so this holds by
/// construction for every index reachable from a valid root.
#[inline(always)]
#[expect(
    clippy::indexing_slicing,
    clippy::expect_used,
    reason = "invariant: `index` addresses a node this crate wrote. Tape indices \
              are never read off the wire — they are literal roots (0) or `next` \
              payloads the parser back-patched from `node_count`. A fallback \
              would have to invent a node word and corrupt the read silently, \
              so the invariant is checked by the debug assertion instead."
)]
pub(crate) fn read_node(tape: &[u8], index: usize) -> u64 {
    let start = index * TAPE_NODE_SIZE;
    debug_assert!(
        start + TAPE_NODE_SIZE <= tape.len(),
        "tape node {index} is past the end of a {}-byte tape",
        tape.len()
    );
    let bytes: [u8; TAPE_NODE_SIZE] = tape[start..start + TAPE_NODE_SIZE]
        .try_into()
        .expect("tape slice shorter than a node");
    u64::from_le_bytes(bytes)
}

/// Number of nodes currently in a tape builder.
#[inline(always)]
pub(crate) fn node_count(tape: &BytesMut) -> usize {
    tape.len() / TAPE_NODE_SIZE
}

/// Appends a node and returns its index.
#[inline(always)]
pub(crate) fn push_node(tape: &mut BytesMut, tag: u8, payload: u64) -> usize {
    let index = node_count(tape);
    tape.put_u64_le(encode(tag, payload));
    index
}

/// Overwrites the payload of an already-emitted node, used to back-patch a
/// container head's `next` once its whole subtree has been written.
#[inline(always)]
#[expect(
    clippy::indexing_slicing,
    reason = "invariant: `index` was returned by `push_node` for this same tape, \
              so the node it addresses has already been appended."
)]
pub(crate) fn patch_node(tape: &mut BytesMut, index: usize, tag: u8, payload: u64) {
    let start = index * TAPE_NODE_SIZE;
    debug_assert!(
        start + TAPE_NODE_SIZE <= tape.len(),
        "tape node {index} is past the end of a {}-byte tape",
        tape.len()
    );
    tape[start..start + TAPE_NODE_SIZE].copy_from_slice(&encode(tag, payload).to_le_bytes());
}
