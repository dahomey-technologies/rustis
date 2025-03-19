use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use futures_util::Future;
use std::time::Duration;

pub fn current_thread_runtime() -> tokio::runtime::Runtime {
    let mut builder = tokio::runtime::Builder::new_current_thread();
    builder.enable_io();
    builder.enable_time();
    builder.build().unwrap()
}

pub fn block_on_all<F>(f: F) -> F::Output
where
    F: Future,
{
    current_thread_runtime().block_on(f)
}

fn get_redis_client() -> redis::Client {
    redis::Client::open("redis://127.0.0.1:6379").unwrap()
}

async fn get_rustis_client() -> rustis::client::Client {
    rustis::client::Client::connect("127.0.0.1:6379")
        .await
        .unwrap()
}

async fn get_fred_client() -> fred::clients::Client {
    use fred::prelude::*;

    let config = Config::default();
    let client = Client::new(config, None, None, None);
    client.connect();
    client.wait_for_connect().await.unwrap();

    client
}

const PARALLEL_QUERIES: usize = 8;
const ITERATIONS: usize = 100;

fn bench_redis_parallel(b: &mut Bencher) {
    use redis::{AsyncCommands, RedisError};

    let client = get_redis_client();
    let runtime = current_thread_runtime();
    let con = runtime
        .block_on(client.get_multiplexed_tokio_connection())
        .unwrap();

    b.iter(|| {
        runtime.block_on(async {
            let tasks: Vec<_> = (0..PARALLEL_QUERIES)
                .map(|i| {
                    let mut con = con.clone();
                    tokio::spawn(async move {
                        for _ in 0..ITERATIONS {
                            let key = format!("key{i}");
                            let value = format!("value{i}");
                            let _: Result<(), RedisError> = con.set(key, value).await;
                        }
                    })
                })
                .collect();

            futures_util::future::join_all(tasks).await;
        })
    });
}

fn bench_fred_parallel(b: &mut Bencher) {
    use fred::prelude::*;

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_fred_client());

    b.iter(|| {
        runtime.block_on(async {
            let tasks: Vec<_> = (0..PARALLEL_QUERIES)
                .map(|i| {
                    let client = client.clone();
                    tokio::spawn(async move {
                        for _ in 0..ITERATIONS {
                            let key = format!("key{i}");
                            let value = format!("value{i}");
                            let _: Result<(), Error> =
                                client.set(key, value, None, None, false).await;
                        }
                    })
                })
                .collect();

            futures_util::future::join_all(tasks).await;
        })
    });
}

fn bench_rustis_parallel(b: &mut Bencher) {
    use rustis::commands::StringCommands;

    let runtime = current_thread_runtime();

    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime.block_on(async {
            let tasks: Vec<_> = (0..PARALLEL_QUERIES)
                .map(|i| {
                    let client = client.clone();
                    tokio::spawn(async move {
                        for _ in 0..ITERATIONS {
                            let key = format!("key{i}");
                            let value = format!("value{i}");
                            let _ = client.set(key, value).await;
                        }
                    })
                })
                .collect();

            futures_util::future::join_all(tasks).await;
        })
    });
}

fn bench_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel");
    group
        .measurement_time(Duration::from_secs(15))
        .bench_function("redis_parallel", bench_redis_parallel)
        .bench_function("fred_parallel", bench_fred_parallel)
        .bench_function("rustis_parallel", bench_rustis_parallel);
    group.finish();
}

criterion_group!(bench, bench_parallel);
criterion_main!(bench);
