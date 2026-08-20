//! The `error` tests that need a live Redis. The ones that need none stay in
//! `error.rs`.

use crate::{
    ErrorKind, RedisError, RedisErrorKind, Result,
    client::BatchPreparedCommand,
    commands::{
        ClientKillOptions, ConnectionCommands, GenericCommands, ListCommands, StringCommands,
    },
    resp::cmd,
    tests::{get_default_config, get_test_client, get_test_client_with_config},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn unknown_command() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.send::<()>(cmd("UNKNOWN").arg("arg"), None).await;

    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description.starts_with("unknown command 'UNKNOWN'")
    ));

    Ok(())
}

#[tokio::test]
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

// #[tokio::test]
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

// #[tokio::test]
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

// #[tokio::test]
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

#[tokio::test]
#[serial]
async fn kill_on_write() -> Result<()> {
    use crate::client::ReconnectionConfig;

    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = get_test_client_with_config(config).await?;

    // 3 reconnections
    let result = client
        .send::<()>(
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
        .send::<()>(
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
        .send::<()>(
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

/// The commonest error of all: the server refused the command. It travels back
/// through the read path rather than through the send path, which is a
/// different route to the caller, and it has to name the command just the same
/// — knowing a `WRONGTYPE` happened is useless without knowing to what.
#[tokio::test]
#[serial]
async fn a_server_error_names_the_command_that_drew_it() -> Result<()> {
    let client = get_test_client().await?;

    client.del("a_list_key").await?;
    client.lpush("a_list_key", "value").await?;

    let result: Result<String> = client.get("a_list_key").await;
    let error = result.expect_err("GET on a list must be refused by the server");

    assert!(
        matches!(error.kind(), ErrorKind::Redis(e) if e.kind == RedisErrorKind::WrongType),
        "expected WRONGTYPE, got {error:?}"
    );
    assert_eq!(Some("GET"), error.command());

    Ok(())
}

/// A batch reply is deserialized command by command, so an error inside it
/// belongs to one command and not to the batch. The naming has to point at the
/// command that actually failed — here the third, not the first, which is what
/// naming a batch after its head would have reported.
#[tokio::test]
#[serial]
async fn a_failing_command_inside_a_transaction_names_itself() -> Result<()> {
    let client = get_test_client().await?;

    client.del("a_list_for_tx").await?;
    client.lpush("a_list_for_tx", "value").await?;

    let mut transaction = client.create_transaction();
    transaction.set("tx_ok_key", "value").forget();
    transaction.get::<String>("a_list_for_tx").queue();
    let result: Result<String> = transaction.execute().await;

    let error = result.expect_err("GET on a list must be refused inside the transaction");
    assert_eq!(
        Some("GET"),
        error.command(),
        "the failing command must name itself, not the head of the batch: {error:?}"
    );

    Ok(())
}
