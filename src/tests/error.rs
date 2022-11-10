use crate::{resp::cmd, tests::get_test_client, Error, RedisError, RedisErrorKind, Result};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unknown_command() -> Result<()> {
    let mut client = get_test_client().await?;

    let result = client.send(cmd("UNKNOWN").arg("arg")).await;

    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description.starts_with("unknown command 'UNKNOWN'")
    ));

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error() -> Result<()> {
//     let mut client = get_test_client().await?;

//     for i in 1..1000 {
//         let key = format!("key{}", i);
//         let value = format!("value{}", i);
//         client.set(key, value).await?;
//     }

//     for i in 1..1000 {
//         let key = format!("key{}", i);
//         let result: Result<String> = client.get(key.clone()).await;
//         println!("test key: {:?}, value: {:?}", key, result);
//         sleep(std::time::Duration::from_secs(1)).await;
//     }

//     Ok(())
// }

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn network_error_stress_test() -> Result<()> {
//     let mut client = get_test_client().await?;

//     for i in 1..1000 {
//         let key = format!("key{}", i);
//         let value = format!("value{}", i);
//         client.set(key, value).await?;
//     }

//     use rand::Rng;

//     let tasks: Vec<_> = (1..8)
//         .into_iter()
//         .map(|_| {
//             let db = client.clone();
//             tokio::spawn(async move {
//                 for _ in 1..10000 {
//                     let i = rand::thread_rng().gen_range(1..1000);
//                     let key = format!("key{}", i);
//                     let result: Result<String> = db.get(key.clone()).await;
//                     println!("test key: {:?}, value: {:?}", key, result);
//                 }
//             })
//         })
//         .collect();

//     future::join_all(tasks).await;

//     Ok(())
// }
