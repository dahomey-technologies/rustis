//! Parser-isolation benchmarks.
//!
//! These measure the RESP decode + deserialize path directly on hand-built
//! buffers, with no network, so the cost the tape rework targets is actually
//! observable — a full client round-trip buries it under socket time. They are
//! the baseline the tape must be measured against:
//!
//! - **scalars** — the tape must stay invisible here (cost provably zero).
//! - **large collection of small elements** — where the tape pays +bytes per
//!   element but saves the header re-parse.
//! - **nested reply** — where the tape wins outright.
//! - **chunked feed** — the same large collection delivered in TCP-sized
//!   slices, the only shape that exposes the partial-parse re-scan.

use criterion::{Criterion, criterion_group, criterion_main};
use rustis::resp::bench_support::{bench_decode_chunked, bench_decode_to};
use std::hint::black_box;

/// A flat RESP array of `n` bulk strings, each `elem_len` bytes.
fn build_array(n: usize, elem_len: usize) -> Vec<u8> {
    let elem = "x".repeat(elem_len);
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{n}\r\n").as_bytes());
    for _ in 0..n {
        buf.extend_from_slice(format!("${elem_len}\r\n").as_bytes());
        buf.extend_from_slice(elem.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }
    buf
}

/// A RESP array of `rows` sub-arrays, each holding `cols` bulk strings of
/// `elem_len` bytes — an `FT.AGGREGATE`-shaped nested reply.
fn build_nested(rows: usize, cols: usize, elem_len: usize) -> Vec<u8> {
    let elem = "x".repeat(elem_len);
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{rows}\r\n").as_bytes());
    for _ in 0..rows {
        buf.extend_from_slice(format!("*{cols}\r\n").as_bytes());
        for _ in 0..cols {
            buf.extend_from_slice(format!("${elem_len}\r\n").as_bytes());
            buf.extend_from_slice(elem.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
    }
    buf
}

fn bench_resp_parsing(c: &mut Criterion) {
    // --- Scalars: the tape must not regress these. ---
    let mut scalars = c.benchmark_group("resp_parsing/scalar");
    scalars.bench_function("simple_string", |b| {
        b.iter(|| {
            let _: String = bench_decode_to(black_box(b"+OK\r\n")).unwrap();
        })
    });
    scalars.bench_function("integer", |b| {
        b.iter(|| {
            let _: i64 = bench_decode_to(black_box(b":1000\r\n")).unwrap();
        })
    });
    scalars.bench_function("bulk_string", |b| {
        b.iter(|| {
            let _: String = bench_decode_to(black_box(b"$5\r\nhello\r\n" as &[u8])).unwrap();
        })
    });
    scalars.finish();

    // --- Large collection of small elements (LRANGE/HGETALL shape). ---
    let large = build_array(5000, 50);
    let chunks_16k: Vec<&[u8]> = large.chunks(16 * 1024).collect();
    let mut collection = c.benchmark_group("resp_parsing/large_collection");
    collection.bench_function("whole", |b| {
        b.iter(|| {
            let _: Vec<String> = bench_decode_to(black_box(&large)).unwrap();
        })
    });
    collection.bench_function("chunked_16k", |b| {
        b.iter(|| {
            let _: Vec<String> = bench_decode_chunked(black_box(&chunks_16k)).unwrap();
        })
    });
    collection.finish();

    // --- Nested reply (FT.AGGREGATE shape). ---
    let nested = build_nested(500, 10, 20);
    let mut nesting = c.benchmark_group("resp_parsing/nested");
    nesting.bench_function("whole", |b| {
        b.iter(|| {
            let _: Vec<Vec<String>> = bench_decode_to(black_box(&nested)).unwrap();
        })
    });
    nesting.finish();
}

criterion_group!(benches, bench_resp_parsing);
criterion_main!(benches);
