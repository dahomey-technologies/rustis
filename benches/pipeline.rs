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

async fn get_fred_client() -> fred::clients::RedisClient {
    use fred::prelude::*;

    let config = RedisConfig::default();
    let client = RedisClient::new(config, None, None, None);
    client.connect();
    client.wait_for_connect().await.unwrap();

    client
}

fn bench_redis_simple_getsetdel_pipeline(b: &mut Bencher) {
    let client = get_redis_client();
    let mut con = client.get_connection().unwrap();

    b.iter(|| {
        let key = "test_key";
        let _result: ((), i64, usize) = redis::pipe()
            .cmd("SET")
            .arg(key)
            .arg(42)
            .cmd("GET")
            .arg(key)
            .cmd("DEL")
            .arg(key)
            .query(&mut con)
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
                pipeline.set(key, 42, None, None, false).await?;
                pipeline.get(key).await?;
                pipeline.del(key).await?;
                let _result: ((), i64, usize) = pipeline.all().await?;

                Ok::<_, RedisError>(())
            })
            .unwrap()
    });
}

fn bench_rustis_simple_getsetdel_pipeline(b: &mut Bencher) {
    use rustis::{resp::cmd, Error};

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";

                let mut pipeline = client.create_pipeline();
                pipeline.queue(cmd("SET").arg(key).arg(42));
                pipeline.queue(cmd("GET").arg(key));
                pipeline.queue(cmd("DEL").arg(key));
                let _result: ((), i64, usize) = pipeline.execute().await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

const PIPELINE_QUERIES: usize = 1_000;

fn bench_redis_async_long_pipeline(b: &mut Bencher) {
    use redis::RedisError;

    let client = get_redis_client();
    let runtime = current_thread_runtime();
    let mut con = runtime
        .block_on(client.get_multiplexed_async_connection())
        .unwrap();

    b.iter(|| {
        runtime
            .block_on(async {
                let mut pipe = redis::pipe();

                for i in 0..PIPELINE_QUERIES {
                    pipe.set(format!("foo{}", i), "bar");
                }

                let _result: Vec<String> = pipe.query_async(&mut con).await?;

                Ok::<_, RedisError>(())
            })
            .unwrap();
    });
}

fn bench_redis_multiplexed_async_long_pipeline(b: &mut Bencher) {
    use redis::RedisError;

    let client = get_redis_client();
    let runtime = current_thread_runtime();
    let mut con = runtime
        .block_on(client.get_multiplexed_tokio_connection())
        .unwrap();

    b.iter(|| {
        runtime
            .block_on(async {
                let mut pipe = redis::pipe();

                for i in 0..PIPELINE_QUERIES {
                    pipe.set(format!("foo{}", i), "bar");
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
                        .set(format!("foo{}", i), "bar", None, None, false)
                        .await?;
                }

                let _result: Vec<String> = pipeline.all().await?;

                Ok::<_, RedisError>(())
            })
            .unwrap()
    });
}

fn bench_rustis_long_pipeline(b: &mut Bencher) {
    use rustis::{client::BatchPreparedCommand, commands::StringCommands, Error};

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let mut pipeline = client.create_pipeline();
                for i in 0..PIPELINE_QUERIES {
                    pipeline.set(format!("foo{}", i), "bar").queue();
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
            "redis_simple_getsetdel_pipeline",
            bench_redis_simple_getsetdel_pipeline,
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
        .bench_function("redis_async_long_pipeline", bench_redis_async_long_pipeline)
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
