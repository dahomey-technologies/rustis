use criterion::{Criterion, criterion_group, criterion_main};
use rustis::resp::{Command, FastPathCommandBuilder, cmd};
use std::hint::black_box;

fn slow_path_get(key: &str) -> Command {
    cmd("GET").key(key).into()
}

fn fast_path_get(key: &str) -> Command {
    FastPathCommandBuilder::get(key)
}

fn bench_get_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("Redis GET");
    let key = "user:123456789:session";

    group.bench_function("Slow Path (Generic)", |b| {
        b.iter(|| black_box(slow_path_get(black_box(key))));
    });

    group.bench_function("Fast Path (Static Header)", |b| {
        b.iter(|| black_box(fast_path_get(black_box(key))));
    });

    group.finish();
}

criterion_group!(benches, bench_get_commands);
criterion_main!(benches);
