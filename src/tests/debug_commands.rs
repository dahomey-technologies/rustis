use crate::{
    Error, Result,
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

/// `DEBUG OOM` and `DEBUG ASSERT` are the two other ways of killing the server
/// on purpose. They take no argument, so what a test can still catch is a
/// misspelled subcommand: the server would answer an error instead of dying.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn standalone_server_oom() -> Result<()> {
    let client = get_test_client().await?;

    assert_died(client.debug_oom().await);

    wait_for_standalone_server_restart().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn standalone_server_assert() -> Result<()> {
    let client = get_test_client().await?;

    assert_died(client.debug_assert().await);

    wait_for_standalone_server_restart().await?;

    Ok(())
}

/// `DEBUG RESTART [<milliseconds>]` and `DEBUG CRASH-AND-RECOVER
/// [<milliseconds>]` take an optional delay, so both arms of the `Option` are
/// exercised: omitted, it must leave no stray token behind.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn standalone_server_restart() -> Result<()> {
    let client = get_test_client().await?;
    assert_died(client.debug_restart(None).await);
    wait_for_standalone_server_restart().await?;

    let client = get_test_client().await?;
    assert_died(client.debug_restart(Some(Duration::from_millis(100))).await);
    wait_for_standalone_server_restart().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn standalone_server_crash_and_recover() -> Result<()> {
    let client = get_test_client().await?;
    assert_died(client.debug_crash_and_recover(None).await);
    wait_for_standalone_server_restart().await?;

    let client = get_test_client().await?;
    assert_died(
        client
            .debug_crash_and_recover(Some(Duration::from_millis(100)))
            .await,
    );
    wait_for_standalone_server_restart().await?;

    Ok(())
}

/// A command meant to take the server down succeeds by never answering. An
/// `Error::Redis` would mean the opposite: the server stayed up long enough to
/// reject what it was sent, which is how a wrong subcommand or a mis-encoded
/// argument shows up here.
fn assert_died(result: Result<()>) {
    match result {
        Err(Error::Redis(e)) => panic!("the server answered instead of dying: {e}"),
        Err(_) => (),
        Ok(()) => panic!("the server acknowledged instead of dying"),
    }
}

/// The test container is configured to restart on failure, and the two restart
/// subcommands re-execute the server in place. Either way, give it back to the
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

    panic!("the test server did not come back up");
}
