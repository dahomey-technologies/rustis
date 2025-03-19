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

async fn get_fred_client() -> fred::clients::Client {
    use fred::prelude::*;

    let config = Config::default();
    let client = Client::new(config, None, None, None);
    client.connect();
    client.wait_for_connect().await.unwrap();

    client
}

async fn get_rustis_client() -> rustis::client::Client {
    rustis::client::Client::connect("127.0.0.1:6379")
        .await
        .unwrap()
}

fn bench_redis_simple_getsetdel_async(b: &mut Bencher) {
    use redis::{cmd, RedisError};

    let client = get_redis_client();
    let runtime = current_thread_runtime();
    let con = client.get_multiplexed_async_connection();
    let mut con = runtime.block_on(con).unwrap();

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";
                cmd("SET")
                    .arg(key)
                    .arg(42.423456)
                    .query_async::<()>(&mut con)
                    .await?;
                let _: f64 = cmd("GET").arg(key).query_async(&mut con).await?;
                cmd("DEL").arg(key).query_async::<usize>(&mut con).await?;
                Ok::<_, RedisError>(())
            })
            .unwrap()
    });
}

fn bench_fred_simple_getsetdel_async(b: &mut Bencher) {
    use fred::prelude::*;
    use fred::types::CustomCommand;

    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_fred_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";

                let args: Vec<Value> = vec![key.into(), 42.423456.into()];
                client
                    .custom::<(), _>(CustomCommand::new_static("SET", None, false), args)
                    .await?;

                let args: Vec<Value> = vec![key.into()];
                client
                    .custom::<f64, _>(CustomCommand::new_static("GET", None, false), args)
                    .await?;

                let args: Vec<Value> = vec![key.into()];
                client
                    .custom::<usize, _>(CustomCommand::new_static("DEL", None, false), args)
                    .await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

fn bench_rustis_simple_getsetdel_async(b: &mut Bencher) {
    use rustis::{resp::cmd, Error};

    let runtime = current_thread_runtime();

    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let key = "test_key";

                client
                    .send(cmd("SET").arg(key).arg(42.423456), None)
                    .await?;
                let _: f64 = client.send(cmd("GET").arg(key), None).await?.to()?;
                client.send(cmd("DEL").arg(key), None).await?;

                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

fn bench_generic_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("generic_api");
    group
        .measurement_time(Duration::from_secs(10))
        .bench_function(
            "redis_simple_getsetdel_async",
            bench_redis_simple_getsetdel_async,
        )
        .bench_function(
            "fred_simple_getsetdel_async",
            bench_fred_simple_getsetdel_async,
        )
        .bench_function(
            "rustis_simple_getsetdel_async",
            bench_rustis_simple_getsetdel_async,
        );
    group.finish();
}

criterion_group!(bench, bench_generic_api);
criterion_main!(bench);
