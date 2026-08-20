use crate::{
    ClientError, Error, ErrorKind, RedisError, RedisErrorKind, Result, TimeoutKind,
    commands::StringCommands,
};
use bytes::Bytes;
use serial_test::serial;

fn redis(kind: RedisErrorKind) -> Error {
    Error::from(ErrorKind::Redis(RedisError {
        kind,
        description: Bytes::new(),
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

/// A server message is bytes, not text. Reading it as text is lossy, so the
/// bytes stay reachable: a key or an argument the server echoed back is
/// recoverable byte for byte, whatever it holds.
#[test]
fn a_non_utf8_server_message_keeps_its_bytes() -> Result<()> {
    let raw: &[u8] = b"ERR unknown command '\xff\xfe'";
    let error = RedisError::try_from(raw)?;

    assert_eq!(b"unknown command '\xff\xfe'", error.description_bytes());
    assert!(error.description().contains('\u{fffd}'));
    assert_eq!("unknown command '��'", error.description());

    Ok(())
}
