use crate::{
    Error, Result,
    resp::{RespBuf, RespFrame, RespFrameParser, RespResponse},
};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// Capacity a recycled tape buffer is reset to once it has been oversized and
/// quiet for long enough. 64 KiB = 8192 nodes, deep enough that a normal reply's
/// tape never reallocates. Mirrors `TARGET_BUFFER_CAPACITY` for the read/write
/// buffers (HARD-03); the knob belongs with those under PROC-07.
pub(crate) const TARGET_TAPE_CAPACITY: usize = 64 * 1024;

/// A tape only marks its recycled block "oversized" once a single frame's tape
/// exceeds this multiple of the target, so a workload of merely largish replies
/// does not trip the reset (hysteresis, part 1).
const TAPE_SHRINK_FACTOR: usize = 8;

/// Consecutive small/scalar frames required after an oversized spike before the
/// block is actually released (hysteresis, part 2).
pub(crate) const TAPE_SHRINK_HYSTERESIS: usize = 16;

/// Streaming RESP decoder.
///
/// Holds one `tape_buf` recycled across frames: [`RespFrameParser`] writes a
/// collection's parse tape into it, then `split().freeze()` per frame hands the
/// frame its own immutable tape while the buffer keeps its capacity for the
/// next one. With prompt consumption (deserialize then drop, the normal
/// request/response path) this reaches a zero-allocation steady state; a
/// retained response pins its split-off tape and forces at most one
/// reallocation of the block.
#[derive(Default)]
pub(crate) struct BufferDecoder {
    tape_buf: BytesMut,
    /// `true` once a frame's tape has grown the recycled block past the shrink
    /// bound. The block then stays pinned — including by `tape_buf`'s own
    /// zero-length tail after a `split()`, whose `capacity()` hides the block
    /// size — until it is explicitly released, so this is tracked directly
    /// rather than inferred from `capacity()`.
    tape_oversized: bool,
    /// Consecutive small/scalar frames seen while `tape_oversized` (HARD-03
    /// hysteresis counter).
    quiet_streak: usize,
}

impl BufferDecoder {
    #[inline]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns the recycled tape buffer's block to the allocator once an
    /// oversized spike has been followed by a quiet streak.
    ///
    /// `BytesMut` has no `shrink_to_fit`, and after a `split()` the buffer is an
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
        if last_tape_len > TARGET_TAPE_CAPACITY * TAPE_SHRINK_FACTOR {
            // The block just grew large; it is legitimately in use this frame.
            self.tape_oversized = true;
            self.quiet_streak = 0;
            return;
        }
        if last_tape_len > TARGET_TAPE_CAPACITY {
            // Moderately busy — not a spike to reclaim, but not quiet either.
            self.quiet_streak = 0;
            return;
        }
        if !self.tape_oversized {
            return;
        }
        self.quiet_streak += 1;
        if self.quiet_streak < TAPE_SHRINK_HYSTERESIS {
            return;
        }
        self.quiet_streak = 0;
        self.tape_oversized = false;
        self.tape_buf = BytesMut::with_capacity(TARGET_TAPE_CAPACITY);
    }

    #[cfg(test)]
    pub(crate) fn tape_capacity(&self) -> usize {
        self.tape_buf.capacity()
    }
}

/// Byte length of a frame's tape, or `0` for a scalar frame (which carries none).
#[inline]
fn frame_tape_len(frame: &RespFrame) -> usize {
    match frame {
        RespFrame::Array { tape, .. }
        | RespFrame::Map { tape, .. }
        | RespFrame::Set { tape, .. }
        | RespFrame::Push { tape, .. } => tape.len(),
        _ => 0,
    }
}

impl Decoder for BufferDecoder {
    type Item = RespResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        if src.is_empty() {
            return Ok(None);
        }

        match RespFrameParser::new(src.as_ref(), &mut self.tape_buf).parse() {
            Ok((frame, frame_len)) => {
                let tape_len = frame_tape_len(&frame);
                let bytes = src.split_to(frame_len).freeze();
                self.recycle_tape(tape_len);
                Ok(Some(RespResponse::new(RespBuf::from(bytes), frame)))
            }
            Err(Error::EOF) => {
                // The frame is incomplete. Discard any partial tape this attempt
                // wrote so the retry — once more bytes arrive — starts from an
                // empty buffer. There is no resume state yet, so the retry
                // re-parses the frame from the start.
                self.tape_buf.clear();
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}
