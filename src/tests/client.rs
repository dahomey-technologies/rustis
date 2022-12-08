use crate::{
    commands::{ClientKillOptions, ConnectionCommands, StringCommands},
    resp::cmd,
    tests::get_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn send() -> Result<()> {
    let mut client = get_test_client().await?;

    client.send(cmd("PING")).await?;

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn forget() -> Result<()> {
    let mut client = get_test_client().await?;

    client.send_and_forget(cmd("PING"))?;
    client.send(cmd("PING")).await?;

    client.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn on_reconnect() -> Result<()> {
    let mut client1 = get_test_client().await?;
    let mut client2 = get_test_client().await?;

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
