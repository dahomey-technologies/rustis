use std::time::Duration;

use crate::{
    client::{Client, IntoConfig},
    commands::{
        BlockingCommands, ClientKillOptions, ConnectionCommands, FlushingMode, LMoveWhere,
        ListCommands, ServerCommands, StringCommands,
    },
    resp::cmd,
    tests::{get_default_addr, get_test_client, log_try_init},
    Error, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn send() -> Result<()> {
    let client = get_test_client().await?;

    client.send(cmd("PING"), None).await?;

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn forget() -> Result<()> {
    let client = get_test_client().await?;

    client.send_and_forget(cmd("PING"), None)?;
    client.send(cmd("PING"), None).await?;

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn on_reconnect() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let mut receiver = client1.on_reconnect();

    let result = receiver.try_recv();
    assert!(result.is_err());

    let client1_id = client1.client_id().await?;
    client2
        .client_kill(ClientKillOptions::default().id(client1_id))
        .await?;

    // send command to be sure that the reconnection has been done
    client1.set("key", "value").await?;

    let result = receiver.try_recv();
    assert!(result.is_ok());

    client1.close().await?;
    client2.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn command_timeout() -> Result<()> {
    log_try_init();

    let client = get_test_client().await?;
    
    client.flushall(FlushingMode::Sync).await?;

    // create an empty list
    client.lpush("key", "value").await?;
    let _result: Vec<String> = client.lpop("key", 1).await?;
    
    client.close().await?;

    let mut config = get_default_addr().into_config()?;
    config.command_timeout = Duration::from_millis(10);

    let client = Client::connect(config).await?;

    // block for 5 seconds
    // since the timeout is configured to 10ms, we should have a timeout error
    let result: Result<Option<(String, Vec<String>)>> =
        client.blmpop(5., "key", LMoveWhere::Left, 1).await;
    assert!(matches!(result, Err(Error::Timeout(_))));

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn connection_name() -> Result<()> {
    log_try_init();

    let mut config = get_default_addr().into_config()?;
    config.connection_name = "myconnection".to_owned();

    let client = Client::connect(config).await?;

    client.flushall(FlushingMode::Sync).await?;

    let connection_name: Option<String> = client.client_getname().await?;
    assert_eq!(Some("myconnection".to_owned()), connection_name);

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn mget_mset() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    client
        .send(
            cmd("MSET")
                .arg("key1")
                .arg("value1")
                .arg("key2")
                .arg("value2")
                .arg("key3")
                .arg("value3")
                .arg("key4")
                .arg("value4"),
            None,
        )
        .await?
        .to::<()>()?;

    let values: Vec<String> = client
        .send(
            cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"),
            None,
        )
        .await?
        .to()?;

    assert_eq!(vec!["value1".to_owned(), "value2".to_owned(), "value3".to_owned(), "value4".to_owned()], values);

    Ok(())
}
