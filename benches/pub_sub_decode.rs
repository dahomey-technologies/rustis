//! Per-message cost of pub/sub delivery — the path no other benchmark reaches.
//!
//! Every delivered message has to leave the network read buffer, which the
//! network task recycles across replies: a message that kept a view into it
//! would pin the whole 64 KiB block for as long as the subscriber held the
//! message. So the segments are copied out, and the only question left is what
//! shape they are copied into.
//!
//! Three shapes, over payload sizes from a bare notification to a large
//! document:
//!
//! - `boxed_slice` — the shipped one: the three segments end to end in one
//!   exactly-sized block, so one allocation and a 32-byte message to move.
//! - `three_vecs` — one owned `Vec` per segment: two allocations for a
//!   `message`, three for a `pmessage`, and a 72-byte message to move.
//! - `inline_buffer` — a 64-byte inline buffer that spills to the heap, so no
//!   allocation at all below the inline width, at the price of an 88-byte
//!   message to move on every delivery.
//!
//! What the comparison says is that the allocation count is not what this path
//! is made of: the parse dominates, the three shapes land within a few percent
//! of each other, and the one that removes *all* the allocations is the slowest
//! because a wider message costs more to move than a small allocation costs to
//! make. The shipped shape is the one that is never worse.
//!
//! The measurement holds the parsed push and its tape across iterations, as the
//! network task holds its recycled buffer, so what is timed is delivery and not
//! the fixture.
//!
//! Run with:
//!   cargo bench --features bench --bench pub_sub_decode

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rustis::resp::bench_support::BenchPubSubPush;
use std::hint::black_box;

/// The RESP3 push frame the server sends for `PUBLISH mychannel <payload>`.
fn build_message_push(channel: &str, payload_len: usize) -> Vec<u8> {
    let payload = vec![b'x'; payload_len];
    let mut buf = b">3\r\n$7\r\nmessage\r\n".to_vec();
    buf.extend_from_slice(format!("${}\r\n{channel}\r\n", channel.len()).as_bytes());
    buf.extend_from_slice(format!("${payload_len}\r\n").as_bytes());
    buf.extend_from_slice(&payload);
    buf.extend_from_slice(b"\r\n");
    buf
}

/// Payload sizes from a bare notification to a document larger than any inline
/// width worth considering.
const PAYLOAD_SIZES: [usize; 6] = [8, 24, 56, 100, 512, 4096];

fn bench_delivery(c: &mut Criterion) {
    let mut group = c.benchmark_group("pub_sub_decode/delivery");
    for size in PAYLOAD_SIZES {
        let push = build_message_push("mychannel", size);

        let mut held = BenchPubSubPush::new(&push).unwrap();
        group.bench_function(BenchmarkId::new("boxed_slice", size), |b| {
            b.iter(|| black_box(held.deliver().unwrap()))
        });

        let mut held = BenchPubSubPush::new(&push).unwrap();
        group.bench_function(BenchmarkId::new("three_vecs", size), |b| {
            b.iter(|| black_box(held.deliver_owned().unwrap()))
        });

        let mut held = BenchPubSubPush::new(&push).unwrap();
        group.bench_function(BenchmarkId::new("inline_buffer", size), |b| {
            b.iter(|| black_box(held.deliver_inline().unwrap()))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_delivery);
criterion_main!(benches);
