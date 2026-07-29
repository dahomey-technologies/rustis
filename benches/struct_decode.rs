//! Struct-decoding benchmark — the `deserialize_struct` path.
//!
//! No other benchmark reaches it: `resp_parsing` measures scalars, large
//! collections and nesting, none of which decode into a struct. Yet the flat
//! RESP2 array a struct is decoded from has to be classified — field/value pairs
//! or positional tuple — once per element for `Vec<StreamEntry>` (`XRANGE`,
//! `XREAD`) and `FT.SEARCH`, so that decision sits on a hot path and needs a
//! baseline to be measured against.
//!
//! The reply is shaped to make the per-entry decision as large a share of the
//! total decode as a real one ever does: many entries, each carrying the
//! smallest possible payload.

use criterion::{Criterion, criterion_group, criterion_main};
use rustis::{commands::StreamEntry, resp::bench_decode_to};
use std::hint::black_box;

/// An `XRANGE` reply of `n` entries: a stream id and a one-pair field/value map
/// each, which is the two-element positional array `StreamEntry` decodes from.
fn build_stream_reply(n: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{n}\r\n").as_bytes());
    for i in 0..n {
        let id = format!("1526985054069-{i}");
        buf.extend_from_slice(b"*2\r\n");
        buf.extend_from_slice(format!("${}\r\n{id}\r\n", id.len()).as_bytes());
        buf.extend_from_slice(b"%1\r\n$5\r\nfield\r\n$5\r\nvalue\r\n");
    }
    buf
}

fn bench_struct_decode(c: &mut Criterion) {
    let reply = build_stream_reply(5000);
    let mut group = c.benchmark_group("struct_decode/xrange");
    group.bench_function("5000_entries", |b| {
        b.iter(|| {
            let _: Vec<StreamEntry<String>> = bench_decode_to(black_box(&reply)).unwrap();
        })
    });
    group.finish();
}

criterion_group!(benches, bench_struct_decode);
criterion_main!(benches);
