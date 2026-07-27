use crate::{
    Result,
    commands::{ConnectionCommands, DebugCommands},
    sleep,
    tests::get_test_client,
};
use serial_test::serial;
use std::time::Duration;

/// A server that dies mid-command must surface an error to the caller instead
/// of leaving it waiting for a reply that will never come.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn standalone_server_panic() -> Result<()> {
    let client = get_test_client().await?;

    let panic_result = client.debug_panic().await;

    assert!(panic_result.is_err());

    let ping_result = client.ping::<()>(()).await;

    assert!(ping_result.is_err());

    wait_for_standalone_server_restart().await?;

    Ok(())
}

/// The test container is configured to restart on failure. Give it back to the
/// rest of the suite only once it answers again, so the next test does not run
/// against a server that is still coming up.
async fn wait_for_standalone_server_restart() -> Result<()> {
    for _ in 0..100 {
        sleep(Duration::from_millis(200)).await;

        let Ok(client) = get_test_client().await else {
            continue;
        };

        if client.ping::<()>(()).await.is_ok() {
            return Ok(());
        }
    }

    panic!("the test server did not restart after DEBUG PANIC");
}
