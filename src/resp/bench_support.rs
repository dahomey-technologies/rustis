//! Benchmark-only entry points into the RESP parser.
//!
//! The parser (`RespFrameParser`, `BufferDecoder`, `RespResponse`, `RespBuf`)
//! is `pub(crate)`, so an external `benches/*.rs` crate cannot reach it. These
//! thin shims expose exactly the decode + deserialize path, isolated from the
//! network, so the parser can be measured on hand-built RESP buffers. They are
//! gated behind the `bench` feature and compiled out of shipped builds, like
//! the `pprof` profiling example.
//!
//! This is the arbitration instrument for the tape rework: the baseline numbers
//! it produces are what the tape is measured against.

use crate::{
    Error, Result,
    resp::{BufferDecoder, RespBuf, RespFrameParser, RespResponse},
};
use bytes::{Bytes, BytesMut};
use serde::de::DeserializeOwned;
use tokio_util::codec::Decoder;

/// Parses one complete RESP frame from `bytes` and deserializes it into `T`.
///
/// This is the whole-buffer path: the frame is fully present, so it exercises
/// the parser's forward pass plus the deserializer, with no resume/EOF handling.
#[inline]
pub fn bench_decode_to<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut tape = BytesMut::new();
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

/// Parses one complete RESP frame from `bytes` into the reused `tape`, without
/// deserializing it — isolating the parser and tape build from serde and
/// allocation cost. Reusing `tape` across calls mirrors the decoder's recycled
/// buffer (the zero-allocation steady state); the built frame is dropped each
/// call, as prompt consumption would. For the CPU profiler (`resp_profiling`);
/// `#[inline(never)]` so it shows as a frame boundary in the sampler.
#[inline(never)]
pub fn bench_parse_only(bytes: &[u8], tape: &mut BytesMut) {
    let (frame, frame_len) = RespFrameParser::new(bytes, tape)
        .parse()
        .expect("bench_parse_only fed a valid frame");
    std::hint::black_box((&frame, frame_len));
}
