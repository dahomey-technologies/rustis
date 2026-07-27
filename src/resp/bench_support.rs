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
    reason = "bench harness over hand-built buffers: a panic on a malformed \
              fixture is the report, and adding error handling would put \
              branches in the code being measured"
)]

use crate::{
    Error, Result,
    resp::{
        BufferDecoder, Command, CommandEncoder, RespBuf, RespFrameParser, RespResponse, RespTapeMut,
    },
};
use bytes::{Bytes, BytesMut};
use serde::de::DeserializeOwned;
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
        None => Err(Error::EOF),
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
    use crate::client::BufferConfig;

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
            return Err(Error::EOF);
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

/// Parses one complete RESP frame from `bytes` into the reused `tape`, without
/// deserializing it — isolating the parser and tape build from serde and
/// allocation cost. Reusing `tape` across calls mirrors the decoder's recycled
/// buffer (the zero-allocation steady state); the built frame is dropped each
/// call, as prompt consumption would. For the CPU profiler (`resp_profiling`);
/// `#[inline(never)]` so it shows as a frame boundary in the sampler.
#[inline(never)]
pub fn bench_parse_only(bytes: &[u8], tape: &mut RespTapeMut) {
    let (frame, frame_len) = RespFrameParser::new(bytes, tape)
        .parse()
        .expect("bench_parse_only fed a valid frame");
    std::hint::black_box((&frame, frame_len));
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
