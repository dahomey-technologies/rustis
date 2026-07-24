use criterion::{Bencher, Criterion, criterion_group, criterion_main};
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

fn get_redis_host() -> String {
    std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn get_redis_client() -> redis::Client {
    let redis_host = get_redis_host();
    redis::Client::open(format!("redis://{redis_host}:6379")).unwrap()
}

async fn get_rustis_client() -> rustis::client::Client {
    let redis_host = get_redis_host();
    rustis::client::Client::connect(redis_host).await.unwrap()
}

async fn get_fred_client() -> fred::clients::Client {
    use fred::prelude::*;

    let redis_host = get_redis_host();
    let config = Config::from_url(&format!("redis://{redis_host}:6379/0")).unwrap();
    let client = Client::new(config, None, None, None);
    client.connect();
    client.wait_for_connect().await.unwrap();

    client
}

// This baseline uses redis-rs's *synchronous* connection (`get_connection()`) —
// no channel hop, no task wakeup, no multiplexer — while the rustis benchmark
// below pays a full multiplexed round-trip. The two are therefore *different
// execution models*: part of any measured gap is the model, not batch
// efficiency. It is kept only as a raw lower-bound reference. For a like-for-like
// comparison against rustis, use the multiplexed baseline below.
fn bench_redis_sync_simple_getsetdel_pipeline(b: &mut Bencher) {
    let client = get_redis_client();
    let mut con = client.get_connection().unwrap();

    b.iter(|| {
        let key = "test_key";
        let _result: ((), i64, usize) = redis::pipe()
            .set(key, 42)
            .get(key)
            .del(key)
            .query(&mut con)
            .unwrap();
    });
}

// Same execution model as `bench_rustis_simple_getsetdel_pipeline`: redis-rs's
// multiplexed async connection. This is the fair head-to-head baseline for the
// 3-command pipeline.
fn bench_redis_multiplexed_simple_getsetdel_pipeline(b: &mut Bencher) {
    use redis::RedisError;

    let client = get_redis_client();
    let runtime = current_thread_runtime();
    let mut con = runtime
        .block_on(client.get_multiplexed_async_connection())
        .unwrap();

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";
                let _result: ((), i64, usize) = redis::pipe()
                    .set(key, 42)
                    .get(key)
                    .del(key)
                    .query_async(&mut con)
                    .await?;

                Ok::<_, RedisError>(())
            })
            .unwrap();
    });
}

fn bench_fred_simple_getsetdel_pipeline(b: &mut Bencher) {
    use fred::prelude::*;

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_fred_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";

                let pipeline = client.pipeline();
                pipeline.set::<(), _, _>(key, 42, None, None, false).await?;
                pipeline.get::<(), _>(key).await?;
                pipeline.del::<(), _>(key).await?;
                let _result: ((), i64, usize) = pipeline.all().await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

fn bench_rustis_simple_getsetdel_pipeline(b: &mut Bencher) {
    use rustis::{
        Error,
        client::BatchPreparedCommand,
        commands::{GenericCommands, StringCommands},
    };

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";

                let mut pipeline = client.create_pipeline();
                pipeline.set(key, 42).queue();
                pipeline.get::<i64>(key).queue();
                pipeline.del(key).queue();
                let _result: ((), i64, usize) = pipeline.execute().await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

const PIPELINE_QUERIES: usize = 1_000;

// redis-rs's multiplexed async connection — the fair head-to-head baseline for
// the 1000-command pipeline, sharing rustis's execution model. redis 1.0 exposes
// no separate single-async connection (`get_async_connection` was removed), so
// this multiplexed connection is the only async model available to compare.
fn bench_redis_multiplexed_async_long_pipeline(b: &mut Bencher) {
    use redis::RedisError;

    let client = get_redis_client();
    let runtime = current_thread_runtime();
    let mut con = runtime
        .block_on(client.get_multiplexed_async_connection())
        .unwrap();

    b.iter(|| {
        runtime
            .block_on(async {
                let mut pipe = redis::Pipeline::with_capacity(PIPELINE_QUERIES);

                for i in 0..PIPELINE_QUERIES {
                    pipe.set(format!("foo{i}"), "bar");
                }

                let _result: Vec<String> = pipe.query_async(&mut con).await?;

                Ok::<_, RedisError>(())
            })
            .unwrap();
    });
}

fn bench_fred_long_pipeline(b: &mut Bencher) {
    use fred::prelude::*;

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_fred_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let pipeline = client.pipeline();
                for i in 0..PIPELINE_QUERIES {
                    pipeline
                        .set::<(), _, _>(format!("foo{i}"), "bar", None, None, false)
                        .await?;
                }

                let _result: Vec<String> = pipeline.all().await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

fn bench_rustis_long_pipeline(b: &mut Bencher) {
    use rustis::{Error, client::BatchPreparedCommand, commands::StringCommands};

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let mut pipeline = client.create_pipeline();
                pipeline.reserve(PIPELINE_QUERIES);

                for i in 0..PIPELINE_QUERIES {
                    pipeline.set(format!("foo{i}"), "bar").queue();
                }

                let _result: Vec<String> = pipeline.execute().await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

fn bench_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_pipeline");
    group
        .measurement_time(Duration::from_secs(10))
        .bench_function(
            "redis_sync_simple_getsetdel_pipeline",
            bench_redis_sync_simple_getsetdel_pipeline,
        )
        .bench_function(
            "redis_multiplexed_simple_getsetdel_pipeline",
            bench_redis_multiplexed_simple_getsetdel_pipeline,
        )
        .bench_function(
            "fred_simple_getsetdel_pipeline",
            bench_fred_simple_getsetdel_pipeline,
        )
        .bench_function(
            "rustis_simple_getsetdel_pipeline",
            bench_rustis_simple_getsetdel_pipeline,
        );
    group.finish();
}

fn bench_long(c: &mut Criterion) {
    let mut group = c.benchmark_group("long_pipeline");
    group
        .measurement_time(Duration::from_secs(10))
        .bench_function(
            "redis_multiplexed_async_long_pipeline",
            bench_redis_multiplexed_async_long_pipeline,
        )
        .bench_function("fred_long_pipeline", bench_fred_long_pipeline)
        .bench_function("rustis_long_pipeline", bench_rustis_long_pipeline);
    group.finish();
}

criterion_group!(bench, bench_simple, bench_long);
criterion_main!(bench);
