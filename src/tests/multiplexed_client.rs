use crate::{
    Result,
    client::Client,
    commands::{FlushingMode, ServerCommands, StringCommands},
    spawn,
    tests::log_try_init,
};
use futures_util::future;
use rand::Rng;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn multiplexed_client() -> Result<()> {
    log_try_init();
    let client = Client::connect("redis://127.0.0.1:6379").await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .mset(
            (0..100)
                .map(|i| (format!("key{i}"), format!("value{i}")))
                .collect::<Vec<_>>(),
        )
        .await?;

    let tasks: Vec<_> = (1..100)
        .map(|_| {
            let client = client.clone();
            spawn(async move {
                for _ in 0..100 {
                    let i = rand::rng().random_range(0..100);
                    let key = format!("key{}", i);
                    let valyue: String = client.get(key.clone()).await.unwrap();
                    assert_eq!(format!("value{}", i), valyue)
                }
            })
        })
        .collect();

    future::join_all(tasks).await;

    Ok(())
}
