//! Public façade exposing internal RESP parsing and deserialization entry
//! points to the `cargo-fuzz` targets in `fuzz/`.
//!
//! This module is only compiled with the `fuzzing` feature, which is meant for
//! the fuzz crate exclusively — it is **not** part of the public API and gives
//! no stability guarantee. It exists because [`crate::resp::RespFrameParser`]
//! and [`crate::resp::RespBuf`] are `pub(crate)` and therefore unreachable from
//! the separate fuzz crate.
//!
//! Invariant exercised by every entry point below: whatever bytes come in, the
//! parser / deserializer must return an error, never panic or abort.
#![allow(missing_docs)]

use crate::resp::{BufferDecoder, RespBuf, RespFrameParser, Value};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// Parse a single RESP frame out of `data` in one shot.
///
/// Mirrors what [`BufferDecoder`] does once it holds a complete frame, but
/// hits [`RespFrameParser`] directly so the fuzzer drives the parser without
/// the streaming layer in between.
pub fn parse_frame(data: &[u8]) {
    // `RespFrameParser::parse` indexes `buf[0]` unconditionally; the streaming
    // decoder guards emptiness, so we replicate that guard here.
    if data.is_empty() {
        return;
    }
    let _ = RespFrameParser::new(data).parse();
}

/// Feed `data` through the streaming [`BufferDecoder`], cutting the input at the
/// byte offsets in `splits` so partial frames are handed to the decoder exactly
/// as they would arrive across TCP segments.
///
/// This is the chunked-path harness the audit's PROC-01 calls for: it exercises
/// the `Error::EOF` resume behaviour and makes RESP-06's re-parse cost
/// observable, in addition to the plain parse path.
pub fn decode_chunked(data: &[u8], splits: &[u8]) {
    let mut decoder = BufferDecoder;
    let mut buf = BytesMut::new();

    // Turn the raw split bytes into sorted, in-range offsets, then always end on
    // the full length so every byte is eventually fed.
    let mut bounds: Vec<usize> = splits
        .iter()
        .map(|&s| (s as usize).min(data.len()))
        .collect();
    bounds.sort_unstable();
    bounds.push(data.len());

    let mut cursor = 0usize;
    for bound in bounds {
        if bound > cursor {
            buf.extend_from_slice(&data[cursor..bound]);
            cursor = bound;
        }
        // Drain every complete frame currently buffered.
        while let Ok(Some(_)) = decoder.decode(&mut buf) {}
    }
}

/// Full read path: parse `data` into a [`Value`] via [`RespBuf::to`], exercising
/// the frame parser and the [`crate::resp::RespDeserializer`] together.
pub fn deserialize_to_value(data: &[u8]) -> Option<Value> {
    if data.is_empty() {
        return None;
    }
    RespBuf::from_slice(data).to::<Value>().ok()
}

/// Value-deserializer path: parse `data` to a [`Value`], then deserialize that
/// `Value` into several concrete Rust types, exercising the coercions in
/// `value_deserializer.rs` (the VAL-02 panic family, RESP-09/VAL-06 lossy
/// casts, empty-collection handling, …).
pub fn value_deserializer_roundtrip(data: &[u8]) {
    // `Value` is not `Clone`, so re-parse for each target type. Parsing is
    // cheap and the fuzzer cares about the deserialize step, not the parse.
    if deserialize_to_value(data).is_none() {
        return;
    }
    macro_rules! target {
        ($t:ty) => {
            if let Some(value) = deserialize_to_value(data) {
                let _ = value.into::<$t>();
            }
        };
    }
    target!(String);
    target!(i64);
    target!(u64);
    target!(f64);
    target!(bool);
    target!(Vec<String>);
    target!(Vec<i64>);
    target!(std::collections::HashMap<String, String>);
    target!(Vec<Vec<u8>>);
}
