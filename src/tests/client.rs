use std::time::Duration;

use crate::{
    ClientError, ErrorKind, Result, TimeoutKind,
    client::{Client, Config, IntoConfig},
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
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::SerdeSerialize(_))
        ),
        "expected a deferred serialization error, got {error:?}"
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

    // A connection that recovers on its own leaves no other trace: the notification
    // is edge-triggered, so a client that was not listening at the time has only
    // this counter.
    let stats = client1.stats();
    assert_eq!(1, stats.reconnections, "{stats:?}");
    assert!(client1.is_connected(), "{stats:?}");

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
        let closed = timeout(
            Duration::from_secs(5),
            TimeoutKind::Command,
            on_reconnect.recv(),
        )
        .await;
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

    let client = Client::connect(config).await?.into_exclusive()?;

    // block for 5 seconds
    // since the timeout is configured to 10ms, we should have a timeout error
    let result: Result<Option<(String, Vec<String>)>> =
        client.blmpop(5., "key", LMoveWhere::Left, 1).await;
    let error = result.expect_err("the command must time out");
    assert!(matches!(error.kind(), ErrorKind::Timeout(_)));
    // A multiplexed client has many commands in flight: the error is worthless
    // unless it names the one that expired.
    assert_eq!(Some("BLMPOP"), error.command());

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

/// A client that exhausts its reconnection budget can never answer again. That
/// state must be observable — a liveness probe has nothing else to read — and it
/// must be reported at `error!`, it being the last event of the client's life.
#[cfg(feature = "tokio-runtime")]
#[tokio::test]
#[serial]
async fn a_client_out_of_reconnection_budget_is_terminated_and_says_so() -> Result<()> {
    use crate::{
        client::ReconnectionConfig,
        network::sleep,
        tests::{LogCapture, fault_injection_proxy::FaultProxy},
    };

    // Once the proxy is gone nothing listens on that port, so the reconnection
    // cannot succeed and the budget runs out.
    let proxy = FaultProxy::start(get_default_addr(), vec![]).await?;
    let mut config: Config = format!("redis://{}", proxy.addr).into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(1, 10);

    let client = Client::connect(config).await?;
    client.set("terminated_probe", "value").await?;
    assert!(
        !client.is_terminated(),
        "a working client must not report itself terminated"
    );

    let capture = LogCapture::start();
    drop(proxy);
    // Long enough for the single reconnection attempt to fail and the task to end.
    sleep(Duration::from_millis(500)).await;
    let events = capture.events();
    drop(capture);

    assert!(
        client.is_terminated(),
        "a client out of reconnection budget must report itself terminated"
    );

    let given_up: Vec<_> = events
        .iter()
        .filter(|(_, message)| message.contains("reconnection attempts"))
        .collect();
    assert!(!given_up.is_empty(), "giving up must be logged: {events:?}");
    for (level, message) in given_up {
        assert_eq!(
            log::Level::Error,
            *level,
            "a client that will never answer again is an error, not a warning: {message}"
        );
    }

    Ok(())
}

/// An interceptor must see every command on both the ergonomic and the generic
/// path, must be able to rewrite one, and must be told how each resolved.
#[tokio::test]
#[serial]
async fn an_interceptor_sees_and_can_rewrite_every_command() -> Result<()> {
    use crate::{
        Error,
        client::{CommandInterceptor, CustomInterceptor},
        resp::Command,
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct Recorder {
        sent: Mutex<Vec<String>>,
        completed: AtomicUsize,
        failed: AtomicUsize,
    }

    impl CommandInterceptor for Arc<Recorder> {
        fn on_command(&self, command: &mut Command) {
            if let Ok(mut sent) = self.sent.lock() {
                sent.push(String::from_utf8_lossy(&command.name_bytes()).into_owned());
            }
        }

        fn on_complete(&self, _command_name: &[u8], _elapsed: Duration, error: Option<&Error>) {
            self.completed.fetch_add(1, Ordering::SeqCst);
            if error.is_some() {
                self.failed.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    log_try_init();
    let recorder = Arc::new(Recorder::default());
    let mut config = crate::tests::get_default_config()?;
    config.interceptor = Some(CustomInterceptor::new(Arc::clone(&recorder)));
    let client = Client::connect(config).await?;

    // The ergonomic path, `client.set(..).await`.
    client.set("interceptor_key", "value").await?;
    // The generic path, `client.send(..)`.
    client.send::<()>(cmd("PING"), None).await?;
    // A command that fails: the interceptor must be told, not only the caller.
    let failed: Result<()> = client.send(cmd("NOTACOMMAND"), None).await;
    assert!(failed.is_err());

    let sent = recorder.sent.lock().expect("poisoned").clone();
    assert!(sent.contains(&"SET".to_owned()), "{sent:?}");
    assert!(sent.contains(&"PING".to_owned()), "{sent:?}");
    assert!(sent.contains(&"NOTACOMMAND".to_owned()), "{sent:?}");
    assert_eq!(3, recorder.completed.load(Ordering::SeqCst));
    assert_eq!(1, recorder.failed.load(Ordering::SeqCst));

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn a_client_reports_what_it_is_connected_to() -> Result<()> {
    log_try_init();
    let client = get_test_client().await?;

    // A health endpoint has to answer without sending a command of its own.
    assert!(client.is_connected());
    let version = client
        .server_version()
        .expect("a standalone connection knows its server version");
    assert!(
        version.split('.').count() == 3,
        "expected a three-part version, got {version}"
    );

    // The config a client was built from stays readable: a pool wrapper or a
    // telemetry exporter cannot ask the server what it was configured with.
    assert!(matches!(
        client.config().server,
        crate::client::ServerConfig::Standalone { .. }
    ));
    assert_eq!(
        crate::tests::get_default_port(),
        match &client.config().server {
            crate::client::ServerConfig::Standalone { port, .. } => *port,
            _ => unreachable!(),
        }
    );

    let stats = client.stats();
    assert_eq!(0, stats.shed_commands);
    assert_eq!(0, stats.reconnections);

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn the_send_queue_depth_is_readable_at_runtime() -> Result<()> {
    log_try_init();
    let client = get_test_client().await?;

    client.send::<()>(cmd("PING"), None).await?;
    let stats = client.stats();
    assert_eq!(
        0, stats.queued_commands,
        "an idle connection holds nothing: {stats:?}"
    );
    assert!(
        stats.queued_bytes_high_water > 0,
        "a command that went out must have been charged: {stats:?}"
    );

    client.close().await?;
    Ok(())
}
