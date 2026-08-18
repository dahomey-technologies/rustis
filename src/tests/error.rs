use crate::{
    ClientError, Error, ErrorKind, RedisError, RedisErrorKind, Result, TimeoutKind,
    client::BatchPreparedCommand,
    commands::{
        ClientKillOptions, ConnectionCommands, GenericCommands, ListCommands, StringCommands,
    },
    resp::cmd,
    tests::{get_default_config, get_test_client, get_test_client_with_config},
};
use bytes::Bytes;
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

#[test]
fn moved_error() {
    let raw_error = b"MOVED 3999 127.0.0.1:6381";
    let error = RedisError::try_from(&raw_error[..]);
    println!("error: {error:?}");
    assert!(matches!(
        error,
        Ok(RedisError {
            kind: RedisErrorKind::Moved { hash_slot: 3999, address: (host, 6381) },
            description
        }) if description.is_empty() && host == "127.0.0.1"
    ));
}

#[test]
fn ask_error() {
    let raw_error = b"ASK 3999 127.0.0.1:6381";
    let error = RedisError::try_from(&raw_error[..]);
    assert!(matches!(
        error,
        Ok(RedisError {
            kind: RedisErrorKind::Ask { hash_slot: 3999, address: (host, 6381) },
            description
        }) if description.is_empty() && host == "127.0.0.1"
    ));
}

#[test]
fn moved_error_ipv6() {
    // The address must be split at the last colon (the port separator), so
    // that IPv6 hosts, which contain colons, are parsed correctly.
    let raw_error = b"MOVED 3999 2001:db8::1:6380";
    let error = RedisError::try_from(&raw_error[..]);
    println!("error: {error:?}");
    assert!(matches!(
        error,
        Ok(RedisError {
            kind: RedisErrorKind::Moved { hash_slot: 3999, address: (host, 6380) },
            description
        }) if description.is_empty() && host == "2001:db8::1"
    ));
}

#[test]
fn an_error_carries_no_command_until_one_is_attached() {
    let error = Error::from(ErrorKind::Timeout(TimeoutKind::Command));

    assert!(matches!(error.kind(), ErrorKind::Timeout(_)));
    assert_eq!(None, error.command());
    assert!(error.context().is_none());
    assert_eq!(
        ErrorKind::Timeout(TimeoutKind::Command).to_string(),
        error.to_string()
    );
}

#[test]
fn attaching_a_command_names_it_in_the_context_and_the_message() {
    let error = Error::from(ErrorKind::Timeout(TimeoutKind::Command))
        .with_command(Bytes::from_static(b"BLMPOP"));

    assert_eq!(Some("BLMPOP"), error.command());
    assert_eq!("BLMPOP", error.context().unwrap().command());
    assert!(
        error.to_string().contains("BLMPOP"),
        "the rendered message must name the command, got {error}"
    );
    // The variant stays reachable: attaching context is not a variant change.
    assert!(matches!(error.kind(), ErrorKind::Timeout(_)));
}

/// The site closest to the cause holds the best command, so an outer layer
/// never overwrites what an inner one already attached.
#[test]
fn the_innermost_command_wins() {
    let error = Error::from(ErrorKind::Timeout(TimeoutKind::Command))
        .with_command(Bytes::from_static(b"GET"))
        .with_command(Bytes::from_static(b"SET"));

    assert_eq!(Some("GET"), error.command());
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

/// `Error` is what every fallible call in the crate returns, so it has to keep
/// slotting into the ecosystem that consumes errors: `?` into a `Box<dyn Error>`
/// or an `anyhow::Error`, and crossing a task boundary.
#[test]
fn the_error_type_keeps_its_bounds() {
    const fn assert_bounds<T: std::error::Error + Send + Sync + Clone + 'static>() {}
    assert_bounds::<Error>();

    let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(
        Error::from(ErrorKind::Timeout(TimeoutKind::Command))
            .with_command(Bytes::from_static(b"GET")),
    );
    assert!(boxed.to_string().contains("GET"));
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

fn redis(kind: RedisErrorKind) -> Error {
    Error::from(ErrorKind::Redis(RedisError {
        kind,
        description: String::new(),
    }))
}

fn client(client_error: ClientError) -> Error {
    Error::from(ErrorKind::Client(client_error))
}

fn timeout(kind: TimeoutKind) -> Error {
    Error::from(ErrorKind::Timeout(kind))
}

fn io() -> Error {
    Error::from(ErrorKind::IO(std::sync::Arc::new(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "reset",
    ))))
}

/// The connection is what died, so the command never got an answer and the
/// client will have to reconnect. A reply the parser could not decode belongs
/// here too: the byte stream is desynchronized, so the connection is done.
#[test]
fn a_connection_error_is_told_from_a_command_error() {
    assert!(io().is_connection_error());
    assert!(Error::from(ErrorKind::EOF).is_connection_error());
    assert!(Error::from(ErrorKind::DisconnectedByPeer).is_connection_error());
    assert!(client(ClientError::CannotParseInteger).is_connection_error());
    assert!(client(ClientError::UnknownRespTag('@')).is_connection_error());

    // The server answered, and answered an error: the connection is fine.
    assert!(!redis(RedisErrorKind::WrongType).is_connection_error());
    // A decode error raised past framing fails one command, not the stream.
    assert!(!client(ClientError::MismatchedKeySlots).is_connection_error());
    assert!(!client(ClientError::CannotParseBytes).is_connection_error());
    assert!(!timeout(TimeoutKind::Command).is_connection_error());
    assert!(!Error::from(ErrorKind::Aborted).is_connection_error());
}

/// A timeout is its own answer: the command may or may not have run, which is
/// neither a connection failure nor a server refusal.
#[test]
fn a_timeout_is_its_own_class() {
    assert!(timeout(TimeoutKind::Command).is_timeout());
    assert!(timeout(TimeoutKind::Connect).is_timeout());

    assert!(!io().is_timeout());
    assert!(!redis(RedisErrorKind::TryAgain).is_timeout());
    assert!(!timeout(TimeoutKind::Command).is_server_error());
    assert!(!timeout(TimeoutKind::Command).is_connection_error());
}

/// The two deadlines demand opposite answers — try another node, or fail this
/// request — so the error has to say which one expired. One variant with one
/// `Display` string left the caller guessing.
#[test]
fn a_timeout_names_the_deadline_that_expired() {
    assert!(matches!(
        timeout(TimeoutKind::Connect).kind(),
        ErrorKind::Timeout(TimeoutKind::Connect)
    ));
    assert!(matches!(
        timeout(TimeoutKind::Command).kind(),
        ErrorKind::Timeout(TimeoutKind::Command)
    ));

    assert_ne!(
        timeout(TimeoutKind::Connect).to_string(),
        timeout(TimeoutKind::Command).to_string()
    );
}

/// A deadline that expires while the connection is being made is a
/// `connect_timeout`, and one that expires while a command is in flight is a
/// `command_timeout`. Both are driven here against a server that accepts the
/// socket and answers nothing.
#[cfg(feature = "tokio-runtime")]
#[tokio::test]
#[serial]
async fn each_deadline_reports_its_own_kind() -> Result<()> {
    use crate::{
        client::{Client, Config, IntoConfig},
        tests::fake_server::HELLO_REPLY,
    };
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // `answer_handshake` false: the socket is accepted and nothing is ever sent,
    // so the connection itself cannot complete. True: the handshake is answered
    // and the first command is left waiting.
    async fn silent_server(
        answer_handshake: bool,
    ) -> Result<(std::net::SocketAddr, tokio::task::JoinHandle<()>)> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server = tokio::spawn(async move {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut chunk = [0u8; 1024];
            if answer_handshake {
                if stream.read(&mut chunk).await.is_err() {
                    return;
                }
                if stream.write_all(HELLO_REPLY).await.is_err() {
                    return;
                }
            }
            while stream.read(&mut chunk).await.is_ok_and(|n| n > 0) {}
        });
        Ok((addr, server))
    }

    let (addr, server) = silent_server(false).await?;
    let mut config: Config = format!("redis://{addr}").into_config()?;
    config.connect_timeout = Duration::from_millis(200);
    let Err(error) = Client::connect(config).await else {
        panic!("the handshake is never answered, so the connection cannot complete")
    };
    server.abort();
    assert!(
        matches!(error.kind(), ErrorKind::Timeout(TimeoutKind::Connect)),
        "a connection that never completes is a connect timeout: {error:?}"
    );

    let (addr, server) = silent_server(true).await?;
    let mut config: Config = format!("redis://{addr}").into_config()?;
    config.command_timeout = Duration::from_millis(200);
    let client = Client::connect(config).await?;
    let error = client
        .get::<String>("key")
        .await
        .expect_err("the command is never answered");
    server.abort();
    assert!(
        matches!(error.kind(), ErrorKind::Timeout(TimeoutKind::Command)),
        "a command that never gets its reply is a command timeout: {error:?}"
    );

    Ok(())
}

/// `Unexpected` reported a dozen distinguishable conditions as
/// `client error: Unexpected error`, which tells the reader nothing and points
/// nowhere. Each one now names itself.
#[test]
fn an_internal_failure_names_the_condition() {
    for error in [
        ClientError::MalformedFrame,
        ClientError::InconsistentRespTape,
        ClientError::NotACollection,
        ClientError::MissingTransactionReply,
        ClientError::IncompatibleShardReplies,
        ClientError::NotAUnitVariant,
        ClientError::MissingMapValue,
    ] {
        let message = error.to_string();
        assert_ne!(
            "Unexpected error", message,
            "{error:?} must say what happened"
        );
        assert!(!message.is_empty());
    }
}

/// The frame parser raised `Unexpected`, which the framing list did not carry,
/// so a failure that leaves the reader at an unknown offset would have been
/// dispatched to a single caller with the stream possibly desynchronised.
#[test]
fn a_framing_failure_belongs_to_the_connection() {
    assert!(client(ClientError::MalformedFrame).is_connection_error());

    // Everything raised past framing still fails one command only.
    assert!(!client(ClientError::InconsistentRespTape).is_connection_error());
    assert!(!client(ClientError::NotACollection).is_connection_error());
    assert!(!client(ClientError::MissingTransactionReply).is_connection_error());
    assert!(!client(ClientError::IncompatibleShardReplies).is_connection_error());
}

/// The server error is the one class the application can act on by name — a
/// `WRONGTYPE` is a bug in the calling code, a `NOAUTH` a bug in the config.
#[test]
fn a_server_error_is_a_reply_the_server_chose_to_send() {
    assert!(redis(RedisErrorKind::WrongType).is_server_error());
    assert!(redis(RedisErrorKind::Other).is_server_error());

    assert!(!io().is_server_error());
    assert!(!client(ClientError::CannotParseInteger).is_server_error());
}

/// What a caller wanting to replay a command needs, in one predicate: the
/// transient failures, whatever layer they came from.
#[test]
fn a_retryable_error_covers_every_transient_layer() {
    assert!(io().is_retryable());
    assert!(Error::from(ErrorKind::EOF).is_retryable());
    assert!(timeout(TimeoutKind::Command).is_retryable());
    assert!(timeout(TimeoutKind::Connect).is_retryable());
    assert!(redis(RedisErrorKind::TryAgain).is_retryable());
    assert!(redis(RedisErrorKind::ClusterDown).is_retryable());
    assert!(redis(RedisErrorKind::MasterDown).is_retryable());
    assert!(redis(RedisErrorKind::NoMasterLink).is_retryable());

    // Replaying these produces the very same error.
    assert!(!redis(RedisErrorKind::WrongType).is_retryable());
    assert!(!redis(RedisErrorKind::NoAuth).is_retryable());
    assert!(!redis(RedisErrorKind::Err).is_retryable());
    assert!(!client(ClientError::MismatchedKeySlots).is_retryable());
    assert!(!Error::from(ErrorKind::Aborted).is_retryable());
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
