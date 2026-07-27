use crate::{
    Error, Result,
    client::{BufferConfig, RespLimits},
    resp::{
        PendingContainer, RespBuf, RespFrame, RespFrameParser, RespResponse, RespTapeMut,
        bulk_value_end,
    },
};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// Streaming RESP decoder.
///
/// Holds one `tape_buf` recycled across frames: [`RespFrameParser`] writes a
/// collection's parse tape into it, then [`RespTapeMut::split_freeze`] per frame
/// hands the frame its own immutable tape while the buffer keeps its capacity for
/// the next one. With prompt consumption (deserialize then drop, the normal
/// request/response path) this reaches a zero-allocation steady state; a
/// retained response pins its split-off tape and forces at most one
/// reallocation of the block.
///
/// It is also **resumable**: when a reply arrives split across TCP chunks, the
/// decoder keeps the partial tape in `tape_buf`, the open-collection `stack`, and
/// a resume offset, so the next chunk continues the frame instead of re-parsing
/// it from the start — removing the quadratic re-scan of large chunked replies.
#[derive(Default)]
pub(crate) struct BufferDecoder {
    /// Tape sizing and shrink policy, from the connection's
    /// [`Config`](crate::client::Config).
    buffers: BufferConfig,
    /// Hostile-input bounds handed to every parser this decoder builds.
    limits: RespLimits,
    tape_buf: RespTapeMut,
    /// `true` once a frame's tape has grown the recycled block past the shrink
    /// bound. The block then stays pinned — including by `tape_buf`'s own
    /// zero-length tail after a split, whose capacity hides the block size — until
    /// it is explicitly released, so this is tracked directly rather than inferred
    /// from the capacity.
    tape_oversized: bool,
    /// Consecutive small/scalar frames seen while `tape_oversized`, gating the
    /// shrink hysteresis.
    quiet_streak: usize,
    /// Open-collection stack, reused across frames. Non-empty only while a
    /// collection reply is mid-flight; combined with `resume_pos` it lets the next
    /// chunk continue the frame instead of re-parsing it from the start.
    stack: Vec<PendingContainer>,
    /// Parse offset to resume from, `Some` only while a frame's bytes are still
    /// arriving. Its partial tape lives in `tape_buf`.
    resume_pos: Option<usize>,
}

impl BufferDecoder {
    /// A decoder on the default policy and limits. The connection path always
    /// goes through [`with_config`](Self::with_config); this is for the harnesses
    /// that drive the decoder outside a connection.
    #[cfg(any(test, feature = "bench", feature = "fuzzing"))]
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A decoder driven by the connection's configured buffer policy and parser
    /// limits, rather than the defaults [`new`](Self::new) applies.
    #[inline]
    pub(crate) fn with_config(buffers: BufferConfig, limits: RespLimits) -> Self {
        Self {
            buffers,
            limits,
            ..Self::default()
        }
    }

    /// Returns the recycled tape buffer's block to the allocator once an
    /// oversized spike has been followed by a quiet streak.
    ///
    /// `BytesMut` has no `shrink_to_fit`, and after a split the buffer is an
    /// empty tail that still holds a reference to the whole (possibly huge)
    /// block, so `capacity()` cannot tell us the block is oversized. We instead
    /// remember that a large tape was built, then — after enough quiet frames
    /// that its frozen tape is surely consumed — replace the buffer with a fresh
    /// target-sized one. That drops our reference: if the block is now solely
    /// ours it is freed immediately, otherwise it lives on through its still-held
    /// frozen tape and is freed when that is dropped. Either way `tape_buf` stops
    /// pinning oversized memory.
    #[inline]
    fn recycle_tape(&mut self, last_tape_len: usize) {
        if last_tape_len > self.buffers.tape_capacity * self.buffers.shrink_factor {
            // The block just grew large; it is legitimately in use this frame.
            self.tape_oversized = true;
            self.quiet_streak = 0;
            return;
        }
        if last_tape_len > self.buffers.tape_capacity {
            // Moderately busy — not a spike to reclaim, but not quiet either.
            self.quiet_streak = 0;
            return;
        }
        if !self.tape_oversized {
            return;
        }
        self.quiet_streak += 1;
        if self.quiet_streak < self.buffers.shrink_hysteresis {
            return;
        }
        self.quiet_streak = 0;
        self.tape_oversized = false;
        self.tape_buf = RespTapeMut::with_capacity(self.buffers.tape_capacity);
    }

    #[cfg(test)]
    pub(crate) fn tape_capacity(&self) -> usize {
        self.tape_buf.byte_capacity()
    }
}

/// Byte length of a frame's tape, or `0` for a scalar frame (which carries none).
#[inline]
fn frame_tape_len(frame: &RespFrame) -> usize {
    match frame {
        RespFrame::Array { tape, .. }
        | RespFrame::Map { tape, .. }
        | RespFrame::Set { tape, .. }
        | RespFrame::Push { tape, .. } => tape.byte_len(),
        _ => 0,
    }
}

impl Decoder for BufferDecoder {
    type Item = RespResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            // Nothing to parse; any in-flight resume state is preserved for the
            // next call (the front bytes it references are still buffered).
            return Ok(None);
        }

        // Resume the in-flight frame if one is suspended, else start a fresh one.
        // Frame-front offsets stay valid because the framing layer never removes
        // bytes from the front except through our own `split_to` on completion.
        let pos = self.resume_pos.unwrap_or(0);

        // Scope the parser so its borrow of `tape_buf` ends before the outcome is
        // acted on; `stack` is the decoder's reused buffer, threaded in by ref.
        let (outcome, end_pos) = {
            let mut parser =
                RespFrameParser::at(src.as_ref(), &mut self.tape_buf, pos, self.limits);
            let outcome = parser.parse_resumable(&mut self.stack);
            (outcome, parser.pos())
        };

        match outcome {
            Ok(Some(frame)) => {
                let tape_len = frame_tape_len(&frame);
                let bytes = src.split_to(end_pos).freeze();
                self.resume_pos = None;
                self.recycle_tape(tape_len);
                Ok(Some(RespResponse::new(RespBuf::from(bytes), frame)))
            }
            Ok(None) => {
                // Keep the partial tape in `tape_buf` and the stack; the next chunk
                // continues this frame from `end_pos` rather than re-parsing it.
                self.resume_pos = Some(end_pos);
                // If the suspension is on a large bulk-family value whose length is
                // already known, reserve the read buffer to the value's exact end in
                // one shot. Otherwise `FramedRead` grows it by doubling, memcpy-ing
                // the whole accumulated reply ~log2(size) times — measurably costly
                // on multi-MB replies (see `benches/large_reply_reserve.rs`). The
                // reservation is bounded by the parser's own bulk-length cap.
                if let Some(end) = bulk_value_end(src, end_pos, self.limits.max_bulk_length)
                    && end > src.len()
                {
                    src.reserve(end - src.len());
                }
                Ok(None)
            }
            Err(e) => {
                // A malformed frame desynchronizes the stream position; drop the
                // partial tape and resume state. The connection layer tears the
                // socket down on this error, so nothing downstream reuses them.
                self.tape_buf.clear();
                self.stack.clear();
                self.resume_pos = None;
                Err(e)
            }
        }
    }
}
