use crate::{
    Error, RedisError, RedisErrorKind, Result,
    commands::{ClientKillOptions, ConnectionCommands, StringCommands},
    resp::cmd,
    tests::{get_default_config, get_test_client, get_test_client_with_config},
};
use serial_test::serial;
use std::str::FromStr;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unknown_command() -> Result<()> {
    let client = get_test_client().await?;

    let result: Result<()> = client.send(cmd("UNKNOWN").arg("arg"), None).await?.to();

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description.starts_with("unknown command 'UNKNOWN'")
    ));

    Ok(())
}

#[test]
fn moved_error() {
    let raw_error = "MOVED 3999 127.0.0.1:6381";
    let error = RedisError::from_str(raw_error);
    println!("error: {error:?}");
    assert!(matches!(
        error,
        Ok(RedisError {
            kind: RedisErrorKind::Moved { hash_slot: 3999, address: (host, 6381) },
            description
        }) if description.is_empty() && host == "127.0.0.1"
    ));
}

#[test]
fn ask_error() {
    let raw_error = "ASK 3999 127.0.0.1:6381";
    let error = RedisError::from_str(raw_error);
    assert!(matches!(
        error,
        Ok(RedisError {
            kind: RedisErrorKind::Ask { hash_slot: 3999, address: (host, 6381) },
            description
        }) if description.is_empty() && host == "127.0.0.1"
    ));
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reconnection() -> Result<()> {
    let mut config = get_default_config()?;
    config.connection_name = "regular".to_string();
    let regular_client = get_test_client_with_config(config).await?;

    let mut config = get_default_config()?;
    config.connection_name = "killer".to_string();
    let killer_client = get_test_client_with_config(config).await?;

    let client_id = regular_client.client_id().await?;
    killer_client
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    let result = regular_client.set("key", "value").await;
    assert!(result.is_err());

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error() -> Result<()> {
//     use crate::commands::StringCommands;

//     let client = get_test_client().await?;

//     let items = (1..1000)
//         .into_iter()
//         .map(|i| (format!("key{i}"), format!("value{i}")))
//         .collect::<Vec<_>>();

//     client.mset(items).await?;

//     for i in 1..1000 {
//         let key = format!("key{i}");
//         let result: Result<String> = client.get(key.clone()).await;
//         println!("test key: {key:?}, result: {result:?}");
//         crate::network::sleep(std::time::Duration::from_secs(1)).await;
//     }

//     Ok(())
// }

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error_stress_test() -> Result<()> {
//     use crate::commands::StringCommands;

//     let client = get_test_client().await?;

//     let items = (1..1000)
//         .into_iter()
//         .map(|i| (format!("key{i}"), format!("value{i}")))
//         .collect::<Vec<_>>();

//     client.mset(items).await?;

//     use rand::Rng;

//     let tasks: Vec<_> = (0..8)
//         .into_iter()
//         .map(|_| {
//             let client = client.clone();
//             tokio::spawn(async move {
//                 for _ in 1..10000 {
//                     let i = rand::rng().random_range(1..1000);
//                     let key = format!("key{i}");
//                     println!("getting key: {key:?}");
//                     let result: Result<String> = client.get(key.clone()).retry_on_error(true).await;
//                     println!("got key: {key:?}, result: {result:?}");
//                     if let Ok(value) = result {
//                         assert_eq!(format!("value{i}"), value);
//                     }
//                 }
//             })
//         })
//         .collect();

//     futures::future::join_all(tasks).await;

//     Ok(())
// }

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error_forget_stress_test() -> Result<()> {
//     use crate::{client::ClientPreparedCommand, commands::StringCommands};

//     let client = get_test_client().await?;

//     crate::network::sleep(std::time::Duration::from_secs(10)).await;

//     use rand::Rng;

//     let tasks: Vec<_> = (1..8)
//         .into_iter()
//         .map(|_| {
//             let client = client.clone();
//             tokio::spawn(async move {
//                 for _ in 1..10 {
//                     let i = rand::rng().random_range(1..1000);
//                     let result = client
//                         .set(format!("key{i}"), format!("value{i}"))
//                         .retry_on_error()
//                         .forget();
//                     println!("test key: key{i}, value: value{i}, result:{result:?}");
//                 }

//                 let result = client.close().await;
//                 println!("client closed, result:{result:?}");
//             })
//         })
//         .collect();

//     futures::future::join_all(tasks).await;

//     client.close().await?;

//     Ok(())
// }

#[cfg(debug_assertions)]
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn kill_on_write() -> Result<()> {
    use crate::client::ReconnectionConfig;

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = get_test_client_with_config(config).await?;

    // 3 reconnections
    let result = client
        .send(
            cmd("SET")
                .arg("key1")
                .arg("value1")
                .kill_connection_on_write(3),
            Some(true),
        )
        .await;
    assert!(result.is_ok());

    // 2 reconnections
    let result = client
        .send(
            cmd("SET")
                .arg("key2")
                .arg("value2")
                .kill_connection_on_write(2),
            Some(true),
        )
        .await;
    assert!(result.is_ok());

    // 2 reconnections / no retry
    let result = client
        .send(
            cmd("SET")
                .arg("key3")
                .arg("value3")
                .kill_connection_on_write(2),
            Some(false),
        )
        .await;
    assert!(result.is_err());

    Ok(())
}
