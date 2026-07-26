//! Per-wakeup allocation cost of the cluster read fan-out.
//!
//! `ClusterConnection::read` rebuilds, on every wakeup, one boxed future per node
//! (`nodes.iter_mut().map(|n| n.connection.read().boxed())`) and hands the `Vec`
//! to `future::select_all`. The alternative is a manual poll loop keeping
//! persistent per-node read futures, which must register the task waker across
//! all nodes without a `Context` — getting that wrong causes missed-wakeup hangs.
//!
//! This bench quantifies the burst such a rewrite would remove: N `Box::pin`
//! allocations + the `select_all` `Vec`, per wakeup, for realistic and
//! pathological node counts. Compare it against the µs-scale per-reply
//! parse/dispatch work and the loopback RTT to decide whether it is worth it.
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
