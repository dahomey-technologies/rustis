use std::time::Duration;

use crate::{
    ClientError, Error, Result,
    client::{Client, IntoConfig},
    commands::{
        BlockingCommands, ClientKillOptions, ConnectionCommands, FlushingMode, LMoveWhere,
        ListCommands, ServerCommands, StringCommands,
    },
    network::timeout,
    resp::cmd,
    tests::{get_default_addr, get_test_client, log_try_init},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn send() -> Result<()> {
    let client = get_test_client().await?;

    client.send::<()>(cmd("PING"), None).await?;

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn failing_user_serialize_surfaces_as_error_not_panic() -> Result<()> {
    let client = get_test_client().await?;

    struct FailingSerialize;
    impl serde::Serialize for FailingSerialize {
        fn serialize<S: serde::Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    let result = client
        .send::<()>(cmd("SET").arg("key").arg(FailingSerialize), None)
        .await;
    assert!(
        matches!(result, Err(Error::Client(ClientError::SerdeSerialize(_)))),
        "expected a deferred serialization error, got {result:?}"
    );

    // The connection is still usable: the doomed command never reached the wire.
    client.send::<()>(cmd("PING"), None).await?;

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn forget() -> Result<()> {
    let client = get_test_client().await?;

    client.send_and_forget(cmd("PING"), None)?;
    client.send::<()>(cmd("PING"), None).await?;

    client.close().await?;

    Ok(())
}

#[tokio::test]
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
    client1.set("key", "value").retry_on_error(true).await?;

    let result = receiver.try_recv();
    assert!(result.is_ok());

    client1.close().await?;
    client2.close().await?;

    Ok(())
}

/// Dropping the last two clones of a client concurrently must still shut the
/// shared connection down. Deciding "am I the last clone?" with two independent
/// `Arc`s and `try_unwrap` let both droppers observe a strong count of 2 and
/// each back off, so the message channel was never closed and the network task,
/// socket and buffers leaked forever. A single shared refcount resolved with
/// `Arc::into_inner` hands exactly one dropper the shutdown, race or not.
#[tokio::test]
#[serial]
async fn concurrent_drop_of_the_last_clones_still_closes_the_connection() -> Result<()> {
    use std::sync::{Arc as StdArc, Barrier};

    log_try_init();

    // The losing interleaving is a narrow window between the swap-out and the
    // ownership check, so a single pair rarely hits it. Repeat enough that the
    // leak surfaces on the buggy path.
    for _ in 0..300 {
        let client = get_test_client().await?;
        let mut on_reconnect = client.on_reconnect();
        let clone = client.clone();

        // Release both threads together so their drops overlap.
        let barrier = StdArc::new(Barrier::new(2));
        let barrier2 = barrier.clone();

        let h1 = std::thread::spawn(move || {
            barrier.wait();
            drop(client);
        });
        let h2 = std::thread::spawn(move || {
            barrier2.wait();
            drop(clone);
        });
        h1.join().unwrap();
        h2.join().unwrap();

        // Once the last clone is gone the network task ends and drops the only
        // remaining reconnect sender, so the receiver reports the channel
        // closed. A leaked task keeps its sender alive and this times out.
        let closed = timeout(Duration::from_secs(5), on_reconnect.recv()).await;
        assert!(
            matches!(closed, Ok(Err(_))),
            "the network task must end when the last client clone is dropped, got {closed:?}"
        );
    }

    Ok(())
}

#[tokio::test]
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
    assert!(matches!(result, Err(Error::Timeout)));

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn connection_name() -> Result<()> {
    log_try_init();

    let mut config = get_default_addr().into_config()?;
    "myconnection".clone_into(&mut config.connection_name);

    let client = Client::connect(config).await?;

    client.flushall(FlushingMode::Sync).await?;

    let connection_name: Option<String> = client.client_getname().await?;
    assert_eq!(Some("myconnection".to_owned()), connection_name);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn mget_mset() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;

    client
        .send::<()>(
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
        .await?;

    let values: Vec<String> = client
        .send(
            cmd("MGET").arg("key1").arg("key2").arg("key3").arg("key4"),
            None,
        )
        .await?;

    assert_eq!(
        vec![
            "value1".to_owned(),
            "value2".to_owned(),
            "value3".to_owned(),
            "value4".to_owned()
        ],
        values
    );

    Ok(())
}
