//! Command write-path (encode) benchmark.
//!
//! `CommandEncoder` copies an already-serialized command into `FramedWrite`'s
//! write buffer. For a large `SET` value this is a second full copy of the
//! payload (the first happened when the command was built). A vectored write
//! (`writev`) could hand the payload straight to the kernel and skip this copy —
//! but only worth the write-path surgery if the copy is actually expensive on
//! large payloads. This bench isolates that copy across payload sizes so the
//! decision rests on numbers, not intuition.
//!
//! `Throughput::Bytes` lets criterion report GiB/s: compare it against realistic
//! link bandwidth to judge whether the memcpy is anywhere near the bottleneck.

use bytes::BytesMut;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustis::resp::{Command, bench_encode_command, cmd};
use std::hint::black_box;

/// A `SET bench:key <value>` command whose value is `payload_len` bytes.
fn build_set(payload_len: usize) -> Command {
    let value = vec![b'x'; payload_len];
    cmd("SET").key("bench:key").arg(value).into()
}

fn bench_command_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_encode/set");

    // From a small value (encoder overhead dominates) up to multi-MiB payloads
    // (pure memcpy dominates) — the range that brackets the writev decision.
    for &payload_len in &[64usize, 4 * 1024, 256 * 1024, 4 * 1024 * 1024] {
        let command = build_set(payload_len);
        let mut buf = BytesMut::with_capacity(payload_len + 64);
        group.throughput(Throughput::Bytes(payload_len as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(payload_len),
            &command,
            |b, command| {
                b.iter(|| bench_encode_command(black_box(command), &mut buf));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_command_encode);
criterion_main!(benches);
