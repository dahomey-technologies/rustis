//! Flat, fixed-width parse tape for RESP collections.
//!
//! A collection reply is parsed once into a sequence of fixed-width nodes stored
//! in a recycled buffer. Reading an element is then an O(1) node lookup plus
//! a content-length read of its own bytes, instead of re-parsing the RESP
//! structure from the start.
//!
//! # Three types
//!
//! - [`TapeNode`] — one node word, giving access to its tag and payload.
//! - [`RespTapeMut`] — the write side, owned by the decoder and recycled across
//!   frames. The parser appends nodes into it and back-patches collection heads.
//! - [`RespTape`] — the read side: one frame's frozen tape, produced by
//!   [`RespTapeMut::split_freeze`] and carried by the decoded response.
//!
//! # Node layout — 8 bytes, little-endian `u64`
//!
//! The high byte holds a tag; the low 56 bits hold a payload. There are two node
//! kinds, distinguished by the tag:
//!
//! - **Scalar node** (tag = the RESP tag byte `+ - : $ , # _ ( = !`): payload is
//!   the element's **start offset**, frame-relative, pointing at its own tag byte
//!   in the data buffer. Its value is recovered by re-reading from that offset
//!   ([`scalar_value`](super::scalar_value)), a content-length read that
//!   touches only this element — never a structural re-parse of the collection.
//! - **Collection node** (tag = `* % ~ >`): a collection occupies **two**
//!   consecutive nodes. The *head* carries `next` = the tape index one past the
//!   whole collection (all descendants included), giving an O(1) skip to the next
//!   sibling. The node right after the head carries `len` (tag [`TAPE_LEN_TAG`]) =
//!   the element count, giving O(1) size hints and RESP2 struct detection. The
//!   collection's children start at `head + 2`.
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
//!
//! [`NULL_TAG`]: super::NULL_TAG

use crate::resp::is_collection_tag;
use bytes::{BufMut, Bytes, BytesMut};

/// Width of one tape node, in bytes. A power of two so node `i` starts at
/// `base + (i << 3)` and every node is naturally aligned for a `u64` load.
pub(crate) const TAPE_NODE_SIZE: usize = 8;

/// Mask selecting the 56-bit payload of a node (the low 7 bytes).
const PAYLOAD_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;

/// The largest value a node payload can hold. Offsets (bounded by the frame
/// length) and tape indices stay far below this in any real reply.
pub(crate) const MAX_TAPE_PAYLOAD: u64 = PAYLOAD_MASK;

/// Tag of the second node of a collection pair, whose payload is the collection's
/// element count. Never used for dispatch (the node is reached positionally, at
/// `head + 1`), so any value outside the RESP tag set is fine; `0` is chosen so
/// a stray read of it is obviously not a real element.
pub(crate) const TAPE_LEN_TAG: u8 = 0;

/// One tape node: a tag byte packed with a 56-bit payload.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TapeNode(u64);

impl TapeNode {
    /// Packs a `(tag, payload)` pair into a node word.
    #[inline(always)]
    pub(crate) fn new(tag: u8, payload: u64) -> Self {
        debug_assert!(
            payload <= MAX_TAPE_PAYLOAD,
            "tape payload overflows 56 bits"
        );
        Self((u64::from(tag) << 56) | (payload & PAYLOAD_MASK))
    }

    /// The node's tag byte.
    #[inline(always)]
    pub(crate) fn tag(self) -> u8 {
        (self.0 >> 56) as u8
    }

    /// The node's 56-bit payload: a frame-relative byte offset for a scalar, a
    /// tape index for a collection head, an element count for a `len` node.
    #[inline(always)]
    pub(crate) fn payload(self) -> u64 {
        self.0 & PAYLOAD_MASK
    }

    /// The payload as an index, for the three uses that are one: a byte offset,
    /// a tape index, an element count.
    ///
    /// The payload is 56 bits wide while `usize` is 32 on some targets, so the
    /// conversion is saturating rather than truncating: an offset too large for
    /// the address space becomes `usize::MAX`, which every bounds check on the
    /// read path rejects, where a wrapped offset would have pointed at a valid
    /// but wrong byte.
    #[inline(always)]
    pub(crate) fn payload_index(self) -> usize {
        usize::try_from(self.payload()).unwrap_or(usize::MAX)
    }

    /// `true` if this node is a collection head.
    #[inline(always)]
    pub(crate) fn is_collection(self) -> bool {
        is_collection_tag(self.tag())
    }
}

/// One frame's frozen tape: the read side, carried by a decoded response.
///
/// An empty tape means the frame is a top-level scalar and carries no nodes.
#[repr(transparent)]
#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct RespTape(Bytes);

impl RespTape {
    /// Number of nodes in the tape.
    #[inline(always)]
    pub(crate) fn node_count(&self) -> usize {
        self.0.len() / TAPE_NODE_SIZE
    }

    /// `true` if the tape holds no node.
    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reads node `index`. `index` must be in bounds (`< node_count()`); the tape
    /// is produced by the parser, so this holds by construction for every index
    /// reachable from a valid root.
    #[inline(always)]
    #[expect(
        clippy::indexing_slicing,
        clippy::expect_used,
        clippy::arithmetic_side_effects,
        reason = "invariant: `index` addresses a node this crate wrote. Tape indices \
                  are never read off the wire — they are literal roots (0) or `next` \
                  payloads the parser back-patched from `node_count`. The byte offset \
                  therefore lands inside a buffer that already holds that node, so \
                  neither the multiply nor the add can leave `usize`. A fallback \
                  would have to invent a node word and corrupt the read silently, \
                  so the invariant is checked by the debug assertion instead."
    )]
    pub(crate) fn node(&self, index: usize) -> TapeNode {
        let start = index * TAPE_NODE_SIZE;
        debug_assert!(
            start + TAPE_NODE_SIZE <= self.0.len(),
            "tape node {index} is past the end of a {}-node tape",
            self.node_count()
        );
        let bytes: [u8; TAPE_NODE_SIZE] = self.0[start..start + TAPE_NODE_SIZE]
            .try_into()
            .expect("tape slice shorter than a node");
        TapeNode(u64::from_le_bytes(bytes))
    }

    /// Byte length of the tape, for the decoder's recycling policy.
    #[inline(always)]
    pub(crate) fn byte_len(&self) -> usize {
        self.0.len()
    }

    /// Copies the nodes into a freshly-sized buffer, releasing the larger recycled
    /// block this tape was split from. Used when a response is retained.
    #[inline]
    #[cfg(any(test, feature = "client-cache"))]
    pub(crate) fn compact(&self) -> RespTape {
        RespTape(Bytes::copy_from_slice(&self.0))
    }

    /// Element count of the collection whose head is at `root`, read from its
    /// companion node. `None` when the tape is too short to hold one, which means
    /// it was not produced by the parser.
    #[cfg(test)]
    #[inline]
    pub(crate) fn collection_len(&self, root: usize) -> Option<usize> {
        let companion = root.checked_add(1)?;
        if self.node_count() <= companion {
            return None;
        }
        Some(self.node(companion).payload_index())
    }
}

/// The write side: the tape builder, owned by the decoder and recycled across
/// frames. [`split_freeze`](Self::split_freeze) hands each frame its own
/// [`RespTape`] while the buffer keeps its capacity for the next one.
#[derive(Default)]
pub(crate) struct RespTapeMut(BytesMut);

impl RespTapeMut {
    /// A builder preallocated for `capacity` bytes.
    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self(BytesMut::with_capacity(capacity))
    }

    /// Number of nodes written so far.
    #[inline(always)]
    pub(crate) fn node_count(&self) -> usize {
        self.0.len() / TAPE_NODE_SIZE
    }

    /// `true` if no node has been written for the current frame.
    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Capacity of the underlying block, in bytes. Lets the tests and the memory
    /// benchmark observe the recycling policy's effect on the block.
    #[cfg(any(test, feature = "bench"))]
    #[inline(always)]
    pub(crate) fn byte_capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Appends a node and returns its index.
    #[inline(always)]
    pub(crate) fn push(&mut self, tag: u8, payload: u64) -> usize {
        let index = self.node_count();
        self.0.put_u64_le(TapeNode::new(tag, payload).0);
        index
    }

    /// Overwrites an already-emitted node, used to back-patch a collection head's
    /// `next` once its whole subtree has been written. `index` must have been
    /// returned by [`push`](Self::push) on this same builder.
    #[inline(always)]
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "invariant: `index` was returned by `push` for this same tape, \
                  so the node it addresses has already been appended and its byte \
                  offset is inside the buffer."
    )]
    pub(crate) fn patch(&mut self, index: usize, tag: u8, payload: u64) {
        let start = index * TAPE_NODE_SIZE;
        debug_assert!(
            start + TAPE_NODE_SIZE <= self.0.len(),
            "tape node {index} is past the end of a {}-node tape",
            self.node_count()
        );
        self.0[start..start + TAPE_NODE_SIZE]
            .copy_from_slice(&TapeNode::new(tag, payload).0.to_le_bytes());
    }

    /// Detaches the nodes written so far as one frame's immutable tape, keeping
    /// the block's capacity for the next frame.
    #[inline]
    pub(crate) fn split_freeze(&mut self) -> RespTape {
        RespTape(self.0.split().freeze())
    }

    /// Discards a partially built tape, after a malformed frame.
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}
