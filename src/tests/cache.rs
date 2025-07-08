use crate::{
    cache::Cache,
    client::Client,
    commands::{
        ClientTrackingOptions, ClientTrackingStatus, ConnectionCommands, FlushingMode,
        HashCommands, ServerCommands, StringCommands,
    },
    network::sleep,
    tests::log_try_init,
    Result,
};
use serial_test::serial;
use std::time::Duration;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cache_get() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    client2.set("key", "value").await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;

    let value: String = cache.get("key").await?;
    assert_eq!("value", value);

    let value: String = cache.get("key").await?;
    assert_eq!("value", value);

    client2.set("key", "new_value").await?;

    sleep(Duration::from_millis(100)).await;

    let value: String = cache.get("key").await?;
    assert_eq!("new_value", value);

    let value: String = cache.get("key").await?;
    assert_eq!("new_value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cache_hash() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    client2
        .hset("key", [("field1", "value1"), ("field2", "value2")])
        .await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value1".to_string()),
            ("field2".to_string(), "value2".to_string())
        ],
        values
    );

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value1".to_string()),
            ("field2".to_string(), "value2".to_string())
        ],
        values
    );

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    client2
        .hset("key", [("field1", "value11"), ("field2", "value22")])
        .await?;

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value11".to_string()),
            ("field2".to_string(), "value22".to_string())
        ],
        values
    );

    let mut values: Vec<(String, String)> = cache.hgetall("key").await?;
    values.sort_by(|(f1, _), (f2, _)| f1.cmp(f2));
    assert_eq!(
        vec![
            ("field1".to_string(), "value11".to_string()),
            ("field2".to_string(), "value22".to_string())
        ],
        values
    );

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    let len = cache.hlen("key").await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn cache_mget() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    client2.flushall(FlushingMode::Sync).await?;
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    let cache = Cache::new(client1.clone(), 60, ClientTrackingOptions::default()).await?;

    client2
        .mset([("key1", "value1"), ("key2", "value2")])
        .await?;

    assert_eq!("value1", cache.get::<_, String>("key1").await?);

    let values: Vec<String> = cache.mget(["key1", "key2"]).await?;
    assert_eq!(vec!["value1".to_string(), "value2".to_string()], values);

    assert_eq!("value1", cache.get::<_, String>("key1").await?);
    assert_eq!("value2", cache.get::<_, String>("key2").await?);

    let values: Vec<String> = cache.mget(["key1", "key2"]).await?;
    assert_eq!(vec!["value1".to_string(), "value2".to_string()], values);

    Ok(())
}
