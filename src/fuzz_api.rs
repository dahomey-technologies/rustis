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

use crate::resp::{BufferDecoder, RespBuf, RespFrameParser, RespTapeMut, Value};
use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// Parse a single RESP frame out of `data` in one shot.
///
/// Mirrors what [`BufferDecoder`] does once it holds a complete frame, but
/// hits [`RespFrameParser`] directly so the fuzzer drives the parser without
/// the streaming layer in between.
pub fn parse_frame(data: &[u8]) {
    // The streaming decoder guards emptiness before calling the parser; mirror
    // that here so the fuzz target exercises the same entry conditions.
    if data.is_empty() {
        return;
    }
    let mut tape = RespTapeMut::default();
    let _ = RespFrameParser::new(data, &mut tape).parse();
}

/// Feed `data` through the streaming [`BufferDecoder`], cutting the input at the
/// byte offsets in `splits` so partial frames are handed to the decoder exactly
/// as they would arrive across TCP segments.
///
/// This drives the decoder's chunk-boundary resume path: feeding the input in
/// pieces makes partial frames exercise the `ErrorKind::EOF` suspend-and-resume
/// behaviour, in addition to the plain parse path.
pub fn decode_chunked(data: &[u8], splits: &[u8]) {
    let mut decoder = BufferDecoder::new();
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
/// `value_deserializer.rs` (the numeric-coercion panic family, lossy casts,
/// empty-collection handling, …).
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
