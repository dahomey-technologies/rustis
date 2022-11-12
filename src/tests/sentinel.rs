use std::collections::HashMap;

use crate::{
    tests::{get_sentinel_test_client, log_try_init},
    Client, ConnectionCommands, Result, SentinelCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unreachable() -> Result<()> {
    log_try_init();
    let result = Client::connect("redis+sentinel://127.0.0.1:1234,127.0.0.1:5678/myservice").await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unknown_service() -> Result<()> {
    log_try_init();
    let result = Client::connect("redis+sentinel://127.0.0.1:26379/unknown").await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn connection() -> Result<()> {
    log_try_init();
    let mut client = get_sentinel_test_client().await?;
    client.hello(Default::default()).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn connection_with_failures() -> Result<()> {
    log_try_init();
    let mut client =
        Client::connect("redis+sentinel://127.0.0.1:1234,127.0.0.1:26379/myservice").await?;
    client.hello(Default::default()).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn config_get_set() -> Result<()> {
    log_try_init();

    // connect to the sentinel instance directly
    let mut client = Client::connect("redis://127.0.0.1:26379").await?;

    client.sentinel_config_set("sentinel-user", "user").await?;
    client.sentinel_config_set("sentinel-pass", "pwd").await?;

    let configs: HashMap<String, String> = client.sentinel_config_get("sentinel-*").await?;
    assert_eq!(2, configs.len());
    assert_eq!(Some(&"user".to_owned()), configs.get("sentinel-user"));
    assert_eq!(Some(&"pwd".to_owned()), configs.get("sentinel-pass"));

    client.sentinel_config_set("sentinel-user", "").await?;
    client.sentinel_config_set("sentinel-pass", "").await?;

    let configs: HashMap<String, String> = client.sentinel_config_get("toto").await?;
    assert_eq!(0, configs.len());

    Ok(())
}
