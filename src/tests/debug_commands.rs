use crate::{
    commands::{ConnectionCommands, DebugCommands, PingOptions},
    tests::{get_cluster_test_client_with_command_timeout, get_test_client},
    Error, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
#[ignore]
async fn standalone_server_panic() -> Result<()> {
    let client = get_test_client().await?;

    let panic_result = client.debug_panic().await;

    assert!(panic_result.is_err());

    let ping_result = client.ping::<()>(PingOptions::default()).await;

    assert!(ping_result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
#[ignore]
async fn cluster_server_panic() -> Result<()> {
    let client = get_cluster_test_client_with_command_timeout().await?;

    let panic_result = client.debug_panic().await;

    assert!(panic_result.is_err());

    let ping_result = client.ping::<()>(PingOptions::default()).await;

    assert!(matches!(ping_result, Err(err) if !matches!(err, Error::Timeout(_))));

    Ok(())
}
