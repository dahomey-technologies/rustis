//! Benchmark-only entry points into the RESP parser.
//!
//! The parser (`RespFrameParser`, `BufferDecoder`, `RespResponse`, `RespBuf`)
//! is `pub(crate)`, so an external `benches/*.rs` crate cannot reach it. These
//! thin shims expose exactly the decode + deserialize path, isolated from the
//! network, so the parser can be measured on hand-built RESP buffers. They are
//! gated behind the `bench` feature and compiled out of shipped builds, like
//! the `pprof` profiling example.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "bench harness over hand-built buffers: a panic on a malformed \
              fixture is the report, and adding error handling would put \
              branches in the code being measured"
)]

use crate::{
    Error, ErrorKind, Result,
    client::{BufferConfig, PubSubMessage},
    network::PubSubPush,
    resp::{
        BufferDecoder, Command, CommandEncoder, ParsedFrame, RespBuf, RespFrameParser,
        RespResponse, RespTapeMut,
    },
};
use bytes::{Bytes, BytesMut};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use tokio_util::codec::{Decoder, Encoder as _};

/// Parses one complete RESP frame from `bytes` and deserializes it into `T`.
///
/// This is the whole-buffer path: the frame is fully present, so it exercises
/// the parser's forward pass plus the deserializer, with no resume/EOF handling.
#[inline]
pub fn bench_decode_to<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut tape = RespTapeMut::default();
    let (frame, frame_len) = RespFrameParser::new(bytes, &mut tape).parse()?;
    let buf = RespBuf::from(Bytes::copy_from_slice(&bytes[..frame_len]));
    RespResponse::new(buf, frame).to()
}

/// A pub/sub push held the way the network task holds one, so delivery can be
/// measured without the artefacts a per-call fixture adds.
///
/// Reparsing from raw bytes on every call would reallocate a parse tape and copy
/// the frame into a fresh `Bytes`; in production the decoder recycles its tape
/// and the frame is already a slice of the read buffer. Both artefacts are
/// allocations, which is the quantity under test — they would drown the answer.
/// This type pays them once, at construction, so each `deliver*` call is the
/// per-message cost a subscriber actually pays.
pub struct BenchPubSubPush {
    buf: RespBuf,
    tape: RespTapeMut,
}

impl BenchPubSubPush {
    /// Parses `bytes` once and keeps the frame, as the read buffer holds it.
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let mut tape = RespTapeMut::default();
        let (_, frame_len) = RespFrameParser::new(bytes, &mut tape).parse()?;
        Ok(Self {
            buf: RespBuf::from(Bytes::copy_from_slice(&bytes[..frame_len])),
            tape,
        })
    }

    fn response(&mut self) -> Result<RespResponse> {
        let (frame, _) = RespFrameParser::new(&self.buf, &mut self.tape).parse()?;
        Ok(RespResponse::new(self.buf.clone(), frame))
    }

    /// Splits the push into its three segments, as every shape below starts by
    /// doing.
    fn segments(response: &RespResponse) -> Result<(&[u8], &[u8], &[u8])> {
        match PubSubPush::try_from(response) {
            Ok(PubSubPush::Message(channel, payload) | PubSubPush::SMessage(channel, payload)) => {
                Ok((&[], channel, payload))
            }
            Ok(PubSubPush::PMessage(pattern, channel, payload)) => Ok((pattern, channel, payload)),
            _ => Err(Error::from(ErrorKind::EOF)),
        }
    }

    /// One delivery, shipped shape: parse the push, then build the
    /// [`PubSubMessage`] a subscriber is handed.
    #[inline(never)]
    pub fn deliver(&mut self) -> Result<PubSubMessage> {
        let response = self.response()?;
        PubSubMessage::try_from(&response)
    }

    /// One delivery into one owned `Vec` per segment — the shape
    /// `PubSubMessage` had before, at two allocations for a `message` and three
    /// for a `pmessage`.
    #[inline(never)]
    pub fn deliver_owned(&mut self) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let response = self.response()?;
        let (pattern, channel, payload) = Self::segments(&response)?;
        Ok((pattern.to_vec(), channel.to_vec(), payload.to_vec()))
    }

    /// One delivery into a 64-byte inline buffer that spills to the heap — no
    /// allocation at all below the inline width, at the price of a wider message
    /// to move on every delivery.
    #[inline(never)]
    pub fn deliver_inline(&mut self) -> Result<(SmallVec<[u8; 64]>, usize, usize)> {
        let response = self.response()?;
        let (pattern, channel, payload) = Self::segments(&response)?;
        let channel_start = pattern.len();
        let payload_start = channel_start + channel.len();
        let mut buf = SmallVec::with_capacity(payload_start + payload.len());
        buf.extend_from_slice(pattern);
        buf.extend_from_slice(channel);
        buf.extend_from_slice(payload);
        Ok((buf, channel_start, payload_start))
    }
}

/// Feeds `chunks` through `BufferDecoder` one at a time, as a socket would
/// deliver a large reply in TCP-sized slices, then deserializes the frame.
///
/// This drives the decoder's chunk-boundary resume path; feeding the same large
/// reply as one slice vs. as many is what makes the streaming (resume-state) win
/// observable — compare the two.
#[inline]
pub fn bench_decode_chunked<T: DeserializeOwned>(chunks: &[&[u8]]) -> Result<T> {
    let mut decoder = BufferDecoder::new();
    let mut buf = BytesMut::new();
    for chunk in chunks {
        buf.extend_from_slice(chunk);
        if let Some(resp) = decoder.decode(&mut buf)? {
            return resp.to();
        }
    }
    match decoder.decode(&mut buf)? {
        Some(resp) => resp.to(),
        None => Err(Error::from(ErrorKind::EOF)),
    }
}

/// Drives [`BufferDecoder`] over `data` delivered in `chunk`-byte slices while
/// growing the read buffer the way `FramedRead` does today: `reserve(1)` before
/// each read, so the `BytesMut` doubles every time it fills. This is the
/// realloc-by-doubling cost — a multi-MB reply is memcpy'd
/// ~log2(size) times as the buffer grows. Returns the decoded frame's byte
/// length; the frame itself is dropped (no deserialize), so the measurement
/// isolates buffer growth from serde.
#[inline(never)]
pub fn bench_decode_stream_grow(data: &[u8], chunk: usize) -> Result<usize> {
    drive_stream(data, chunk, false)
}

/// Same as [`bench_decode_stream_grow`], but reserves the announced frame size
/// once: the read buffer reaches its final capacity in a
/// single allocation and never doubles. Compare the two to decide whether the
/// reallocation cost is worth re-plumbing the decoder's EOF contract.
#[inline(never)]
pub fn bench_decode_stream_prereserve(data: &[u8], chunk: usize) -> Result<usize> {
    drive_stream(data, chunk, true)
}

/// Shared driver for the two streaming-reserve shims above. Models the
/// `FramedRead` read loop: decode what is buffered, and if the frame is
/// incomplete, reserve then copy the next `chunk`-sized slice from `data`.
#[inline(always)]
fn drive_stream(data: &[u8], chunk: usize, prereserve: bool) -> Result<usize> {
    let mut decoder = BufferDecoder::new();
    let mut src = BytesMut::with_capacity(BufferConfig::DEFAULT.read_capacity);
    let mut pos = 0usize;
    let mut reserved = false;
    loop {
        if let Some(resp) = decoder.decode(&mut src)? {
            let len = match std::hint::black_box(&resp) {
                RespResponse::Frame { buf, .. } => buf.as_ref().len(),
                _ => 0,
            };
            return Ok(len);
        }
        if pos >= data.len() {
            return Err(Error::from(ErrorKind::EOF));
        }
        // The fix reserves the whole announced reply once the header is buffered.
        if prereserve && !reserved && !src.is_empty() {
            src.reserve(data.len().saturating_sub(src.len()));
            reserved = true;
        }
        // FramedRead reserves at least one byte before every read; on a full
        // BytesMut that doubles the block.
        src.reserve(1);
        let spare = src.capacity() - src.len();
        let take = spare.min(chunk).min(data.len() - pos);
        src.extend_from_slice(&data[pos..pos + take]);
        pos += take;
    }
}

/// A parse tape a benchmark can carry across calls, the way the decoder carries
/// its recycled one.
///
/// Opaque on purpose: the tape types stay `pub(crate)`, so a benchmark holds one
/// without the parser's internals becoming reachable from outside the crate.
#[derive(Default)]
pub struct BenchTape(RespTapeMut);

impl BenchTape {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parses one complete RESP frame from `bytes` into the reused `tape`, without
/// deserializing it — isolating the parser and tape build from serde and
/// allocation cost. Reusing `tape` across calls mirrors the decoder's recycled
/// buffer (the zero-allocation steady state); the built frame is dropped each
/// call, as prompt consumption would. For the CPU profiler (`resp_profiling`);
/// `#[inline(never)]` so it shows as a frame boundary in the sampler.
#[inline(never)]
pub fn bench_parse_only(bytes: &[u8], tape: &mut BenchTape) {
    let tape = &mut tape.0;
    let (frame, frame_len) = RespFrameParser::new(bytes, tape)
        .parse()
        .expect("bench_parse_only fed a valid frame");
    std::hint::black_box((&frame, frame_len));
}

/// Parses one complete RESP frame from `bytes` and reports what it costs to index:
/// `(frame_len, tape_bytes)`, the reply's own byte length and the byte length of the
/// tape built over it.
///
/// This is the footprint instrument: the tape is a fixed width per element, so its
/// share of the total is set by the average element size, and only measurement over
/// real reply shapes says whether that share is acceptable. A scalar frame carries
/// no tape and reports `0`.
#[inline(never)]
pub fn bench_tape_footprint(bytes: &[u8]) -> (usize, usize) {
    let mut tape = RespTapeMut::default();
    let (frame, frame_len) = RespFrameParser::new(bytes, &mut tape)
        .parse()
        .expect("bench_tape_footprint fed a valid frame");
    let tape_bytes = match &frame {
        ParsedFrame::Collection(tape) => tape.byte_len(),
        _ => 0,
    };
    (frame_len, tape_bytes)
}

/// A [`BufferDecoder`] a benchmark can drive across frames, on the shipped default
/// buffer policy, while observing how much tape memory the recycled block holds.
///
/// Opaque on purpose, like [`BenchTape`]: the decoder and the tape types stay
/// `pub(crate)`. This exposes exactly the two numbers the retained-memory question
/// needs — the tape a frame just built, and the block still pinned afterwards.
pub struct BenchDecoder {
    decoder: BufferDecoder,
    src: BytesMut,
}

impl Default for BenchDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchDecoder {
    pub fn new() -> Self {
        Self {
            decoder: BufferDecoder::new(),
            src: BytesMut::with_capacity(BufferConfig::DEFAULT.read_capacity),
        }
    }

    /// Feeds one whole reply and decodes it, returning the byte length of the tape
    /// built for it. The response is dropped before returning, as prompt
    /// consumption would, so the frozen tape stops pinning the recycled block and
    /// [`retained_tape_capacity`](Self::retained_tape_capacity) reads what the
    /// decoder alone holds.
    pub fn feed(&mut self, reply: &[u8]) -> Result<usize> {
        self.src.extend_from_slice(reply);
        let Some(resp) = self.decoder.decode(&mut self.src)? else {
            return Err(Error::from(ErrorKind::EOF));
        };
        let tape_bytes = match &resp {
            RespResponse::Frame { tape, .. } => tape.byte_len(),
            _ => 0,
        };
        drop(resp);
        Ok(tape_bytes)
    }

    /// Capacity, in bytes, of the tape block the decoder is currently holding.
    ///
    /// This is the memory a quiet connection keeps immobilized after a large reply,
    /// until the shrink hysteresis releases it — the number the per-frame tape size
    /// does not tell you.
    pub fn retained_tape_capacity(&self) -> usize {
        self.decoder.tape_capacity()
    }
}

/// Encodes `command` into `buf` through [`CommandEncoder`] — the exact write-path
/// step that copies the already-serialized command into `FramedWrite`'s buffer.
///
/// This is the isolation instrument for the vectored-write question: on a large
/// `SET` payload, the encoder copies the whole value a second time (once into the
/// command buffer at build time, once here into the write buffer). Reusing `buf`
/// across calls mirrors the recycled write buffer, so the measurement is the pure
/// `reserve` + `memcpy` cost a vectored write would remove — decide on the numbers.
#[inline(never)]
pub fn bench_encode_command(command: &Command, buf: &mut BytesMut) {
    buf.clear();
    CommandEncoder
        .encode(command, buf)
        .expect("bench_encode_command fed a valid command");
    std::hint::black_box(&buf);
}
