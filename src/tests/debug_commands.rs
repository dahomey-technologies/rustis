use crate::{
    ErrorKind, Result,
    client::Client,
    commands::{ConnectionCommands, DebugCommands},
    sleep,
    tests::get_test_client,
};
use serial_test::serial;
use std::{future::Future, time::Duration};

/// What a test body concluded; `Err` carries the message it fails with.
type Verdict = std::result::Result<(), String>;

/// A server that dies mid-command must surface an error to the caller instead
/// of leaving it waiting for a reply that will never come.
#[tokio::test]
#[serial]
async fn standalone_server_panic() -> Result<()> {
    with_server_restart("DEBUG PANIC", async |client| {
        if client.debug_panic().await.is_ok() {
            return Err("the server acknowledged instead of dying".to_owned());
        }

        if client.ping::<()>(()).await.is_ok() {
            return Err("the dead server answered a ping".to_owned());
        }

        Ok(())
    })
    .await
}

/// `DEBUG OOM` and `DEBUG ASSERT` are the two other ways of killing the server
/// on purpose. They take no argument, so what a test can still catch is a
/// misspelled subcommand: the server would answer an error instead of dying.
#[tokio::test]
#[serial]
async fn standalone_server_oom() -> Result<()> {
    with_server_restart("DEBUG OOM", async |client| {
        check_died(client.debug_oom().await)
    })
    .await
}

#[tokio::test]
#[serial]
async fn standalone_server_assert() -> Result<()> {
    with_server_restart("DEBUG ASSERT", async |client| {
        check_died(client.debug_assert().await)
    })
    .await
}

/// `DEBUG RESTART [<milliseconds>]` and `DEBUG CRASH-AND-RECOVER
/// [<milliseconds>]` take an optional delay, so both arms of the `Option` are
/// exercised: omitted, it must leave no stray token behind.
#[tokio::test]
#[serial]
async fn standalone_server_restart() -> Result<()> {
    with_server_restart("DEBUG RESTART", async |client| {
        check_died(client.debug_restart(None).await)
    })
    .await?;

    with_server_restart("DEBUG RESTART 100", async |client| {
        check_died(client.debug_restart(Some(Duration::from_millis(100))).await)
    })
    .await
}

#[tokio::test]
#[serial]
async fn standalone_server_crash_and_recover() -> Result<()> {
    with_server_restart("DEBUG CRASH-AND-RECOVER", async |client| {
        check_died(client.debug_crash_and_recover(None).await)
    })
    .await?;

    with_server_restart("DEBUG CRASH-AND-RECOVER 100", async |client| {
        check_died(
            client
                .debug_crash_and_recover(Some(Duration::from_millis(100)))
                .await,
        )
    })
    .await
}

/// A command meant to take the server down succeeds by never answering. An
/// `ErrorKind::Redis` would mean the opposite: the server stayed up long enough to
/// reject what it was sent, which is how a wrong subcommand or a mis-encoded
/// argument shows up here.
fn check_died(result: Result<()>) -> Verdict {
    match result {
        Err(e) if matches!(e.kind(), ErrorKind::Redis(_)) => {
            Err(format!("the server answered instead of dying: {e}"))
        }
        Err(_) => Ok(()),
        Ok(()) => Err("the server acknowledged instead of dying".to_owned()),
    }
}

/// The body kills the test server, so its verdict is reported only once the
/// server answers again: failing earlier would leave the rest of the suite
/// running against a server that is gone.
async fn with_server_restart<F, Fut>(what: &str, body: F) -> Result<()>
where
    F: FnOnce(Client) -> Fut,
    Fut: Future<Output = Verdict>,
{
    let client = get_test_client().await?;

    let outcome = body(client).await;

    wait_for_standalone_server_restart(what).await;

    match outcome {
        Ok(()) => Ok(()),
        Err(message) => panic!("{message}"),
    }
}

/// The test container is configured to restart on failure, and the two restart
/// subcommands re-execute the server in place. Either way, give it back to the
/// rest of the suite only once it answers again, so the next test does not run
/// against a server that is still coming up.
async fn wait_for_standalone_server_restart(what: &str) {
    for _ in 0..100 {
        sleep(Duration::from_millis(200)).await;

        let Ok(client) = get_test_client().await else {
            continue;
        };

        if client.ping::<()>(()).await.is_ok() {
            return;
        }
    }

    panic!("the test server did not come back up after {what}");
}
