use crate::{
    Result, client::PooledClientManager, commands::StringCommands, tests::get_default_addr,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn pooled_client_manager() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr())?;
    let pool = crate::bb8::Pool::builder().build(manager).await?;
    let client = pool.get().await.unwrap();

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

/// The pool asks the manager whether a connection is broken before handing it
/// out again. A client whose network task has ended can no longer answer
/// anything, so the pool must drop it instead of recycling it forever.
#[cfg(feature = "tokio-runtime")]
#[tokio::test]
#[serial]
async fn a_client_whose_network_task_ended_is_reported_broken() -> Result<()> {
    use crate::{
        client::{Config, IntoConfig, ReconnectionConfig},
        tests::fault_injection_proxy::FaultProxy,
    };
    use bb8::ManageConnection;

    // Front the server with a proxy so the connection can be made unrecoverable:
    // once the proxy is gone, nothing listens on that port any more.
    let proxy = FaultProxy::start(get_default_addr(), vec![]).await?;
    let mut config: Config = format!("redis://{}", proxy.addr).into_config()?;
    // A single reconnection attempt, so the network task gives up promptly.
    config.reconnection = ReconnectionConfig::new_constant(1, 10);

    let manager = PooledClientManager::new(config)?;
    let mut client = manager.connect().await?;
    client.set("pool_probe", "value").await?;

    drop(proxy);

    // Give the network task the time to fail its single reconnection attempt and end.
    crate::network::sleep(std::time::Duration::from_millis(500)).await;

    assert!(
        manager.has_broken(&mut client),
        "a client whose network task has ended must be reported broken"
    );

    Ok(())
}
