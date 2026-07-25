//! Per-wakeup allocation cost of the cluster read fan-out (arbitrates CLU-07).
//!
//! `ClusterConnection::read` rebuilds, on every wakeup, one boxed future per node
//! (`nodes.iter_mut().map(|n| n.connection.read().boxed())`) and hands the `Vec`
//! to `future::select_all`. CLU-07 proposes replacing this with a manual poll
//! loop that keeps persistent per-node read futures — a change that must register
//! the task waker across all nodes without `Context`, and getting it wrong
//! reintroduces the missed-wakeup hangs Wave 1 closed.
//!
//! Before taking that risk, this bench quantifies the burst the rewrite would
//! remove: N `Box::pin` allocations + the `select_all` `Vec`, per wakeup, for
//! realistic and pathological node counts. If it is trivial next to the µs-scale
//! per-reply parse/dispatch work and the loopback RTT, the "profile first" gate
//! is not met and the rewrite is not justified.
//!
//! This isolates the allocation/setup only: the per-node futures resolve
//! immediately, so no socket or real I/O is involved.
//!
//! Run with:
//!   cargo bench --features bench --bench cluster_read_alloc

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use futures_util::future::{self, FutureExt};
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("cluster_read_alloc");
    // 3–6 nodes is a common cluster; 16 a large one; 100 pathological.
    for &n in &[3usize, 6, 16, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                rt.block_on(async {
                    // Mirror read()'s fan-out: one boxed future per node, then
                    // select_all over the freshly built Vec.
                    let futures = (0..n).map(|i| std::future::ready(i as u8).boxed());
                    let (result, idx, rest) = future::select_all(futures).await;
                    black_box((result, idx, rest.len()));
                })
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
