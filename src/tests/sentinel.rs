use crate::{
    client::Client,
    commands::{ConnectionCommands, SentinelCommands},
    tests::{get_sentinel_master_test_client, get_sentinel_test_client, log_try_init},
    Result,
};
use serial_test::serial;
use std::collections::HashMap;

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
    let mut client = get_sentinel_master_test_client().await?;
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
    // connect to the sentinel instance directly for these commands
    let mut client = get_sentinel_test_client().await?;

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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_ckquorum() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    client.sentinel_ckquorum("myservice").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_flushconfig() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    client.sentinel_flushconfig().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_info_cache() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    let result: HashMap<String, Vec<(u64, String)>> =
        client.sentinel_info_cache("myservice").await?;
    assert_eq!(1, result.len());
    assert!(result.get("myservice").is_some());
    assert!(result.get("myservice").unwrap().len() == 2); // 1 master & 1 replica

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_master() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    let result = client.sentinel_master("myservice").await?;
    assert_eq!("master", result.flags);
    //assert_eq!(2, result.num_other_sentinels);
    assert_eq!(2, result.quorum);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_masters() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    let result = client.sentinel_masters().await?;
    assert_eq!(1, result.len());
    assert_eq!("master", result[0].flags);
    //assert_eq!(2, result[0].num_other_sentinels);
    assert_eq!(2, result[0].quorum);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn sentinel_remove_and_monitor() -> Result<()> {
//     // connect to the sentinel instance directly for these commands
//     let mut client = get_sentinel_test_client().await?;

//     let master_info = client.sentinel_master("myservice").await?;

//     client.sentinel_remove("myservice").await?;
//     client
//         .sentinel_monitor(
//             "myservice",
//             master_info.ip,
//             master_info.port,
//             master_info.quorum,
//         )
//         .await?;

//     client.sentinel_reset("myservice").await?;

//     Ok(())
// }

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_set() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    client
        .sentinel_set(
            "myservice",
            [
                ("down-after-milliseconds", 1000),
                ("failover-timeout", 1000),
            ],
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_myid() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    let id = client.sentinel_myid().await?;
    assert!(!id.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_pending_scripts() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut sentinel_client = get_sentinel_test_client().await?;

    let result = sentinel_client.sentinel_pending_scripts().await?;
    assert!(result.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_replicas() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut sentinel_client = get_sentinel_test_client().await?;

    let result = sentinel_client.sentinel_replicas("myservice").await?;
    assert_eq!(1, result.len());
    assert_eq!("slave", result[0].flags);
    assert_eq!(6382, result[0].port);
    assert_eq!(6381, result[0].master_port);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sentinel_sentinels() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let mut client = get_sentinel_test_client().await?;

    let result = client.sentinel_sentinels("myservice").await?;
    assert!(!result.is_empty());
    assert!(result[0].flags.contains("sentinel"));
    //assert_eq!(26379, result[0].port);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn sentinel_reset() -> Result<()> {
//     // connect to the sentinel instance directly for this command
//     let mut client = get_sentinel_test_client().await?;

//     let num = client.sentinel_reset("myservice").await?;
//     assert_eq!(1, num);

//     Ok(())
// }
