//! Read-buffer reservation benchmark.
//!
//! A large RESP reply arrives from the socket in TCP-sized slices. `FramedRead`
//! today reserves one byte before each read, so its `BytesMut` read buffer grows
//! by doubling and memcpy's everything received so far ~log2(size) times as a
//! multi-MB reply accumulates. RESP is length-delimited, so the decoder *could*
//! reserve the announced size once and never double.
//!
//! This bench feeds the same reply through the real `BufferDecoder` two ways —
//! the `FramedRead`-style loop that only reserves one byte per read vs a harness
//! that reserves the whole reply once — with no socket, so the reallocation cost
//! is isolated from network time.
//!
//! The decoder itself now reserves the announced bulk length, so the two
//! converge: the first variant relies solely on the decoder's internal
//! reservation, so it doubles as a **regression guard** — if that reservation is
//! ever removed, `grow_by_doubling` blows back up (~3.7× on 16 MiB, measured).
//!
//! Run with:
//!   cargo bench --features bench --bench large_reply_reserve

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustis::resp::bench_support::{bench_decode_stream_grow, bench_decode_stream_prereserve};
use std::hint::black_box;

/// A single RESP bulk string of `payload` bytes: `$<len>\r\n<payload>\r\n`.
fn build_bulk_string(payload: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(payload + 32);
    buf.extend_from_slice(format!("${payload}\r\n").as_bytes());
    buf.resize(buf.len() + payload, b'x');
    buf.extend_from_slice(b"\r\n");
    buf
}

fn bench(c: &mut Criterion) {
    // TCP-sized delivery slice; independent of the read buffer's growth.
    const CHUNK: usize = 16 * 1024;

    let mut group = c.benchmark_group("large_reply_reserve");
    for &mib in &[1usize, 4, 16] {
        let payload = mib * 1024 * 1024;
        let data = build_bulk_string(payload);
        group.throughput(Throughput::Bytes(data.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("grow_by_doubling", format!("{mib}MiB")),
            &data,
            |b, data| {
                b.iter(|| black_box(bench_decode_stream_grow(black_box(data), CHUNK).unwrap()))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("reserve_once", format!("{mib}MiB")),
            &data,
            |b, data| {
                b.iter(|| {
                    black_box(bench_decode_stream_prereserve(black_box(data), CHUNK).unwrap())
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
