use crate::Result;
use atoi::atoi;
use bytes::Bytes;
use futures_channel::mpsc;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    num::{ParseFloatError, ParseIntError},
    str::Utf8Error,
    string::FromUtf8Error,
    sync::Arc,
};
use thiserror::Error;

/// `Internal Use`
///
/// Gives a reason to retry sending a command to the Redis Server
#[doc(hidden)]
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RetryReason {
    /// Received an ASK error from the Redis Server
    Ask {
        hash_slot: u16,
        address: (String, u16),
    },
    /// Received a MOVED error from the Redis Server
    Moved {
        hash_slot: u16,
        address: (String, u16),
    },
    /// Received a transient cluster error (`TRYAGAIN`, `CLUSTERDOWN`) from the
    /// Redis Server: the command was not executed and the cluster spec asks the
    /// client to replay it after a short delay.
    TryAgain {
        /// How long to wait before the command is sent again.
        delay: std::time::Duration,
        /// Whether the local topology is suspect and has to be reloaded before
        /// the replay: true for `CLUSTERDOWN`, which follows a topology change,
        /// false for `TRYAGAIN`, which only reports a slot in migration.
        refresh_topology: bool,
    },
}

/// Errors issued by the client
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum ClientError {
    /// Raised when an invalid RESP tag is encountered
    #[error("protocol: invalid tag")]
    InvalidTag,
    /// Raised when an expected array result is not received for MGET command
    #[error("protocol: expected array result for MGET")]
    ExpectedArrayForMGet,
    /// Raised when cannot parse integer from the RESP buffer
    #[error("protocol: cannot parse integer")]
    CannotParseInteger,
    /// Raised when cannot parse double from the RESP buffer
    #[error("protocol: cannot parse double")]
    CannotParseDouble,
    /// Raised when cannot parse bulk string from the RESP buffer
    #[error("protocol: cannot parse bulk string")]
    CannotParseBulkString,
    /// Raised when cannot parse bulk error from the RESP buffer
    #[error("protocol: cannot parse bulk error")]
    CannotParseBulkError,
    /// Raised when cannot parse verbartim string from the RESP buffer
    #[error("protocol: cannot parse verbartim string")]
    CannotParseVerbatimString,
    /// Raised when cannot parse nil from the RESP buffer
    #[error("protocol: cannot parse nil")]
    CannotParseNil,
    /// Raised when cannot parse boolean from the RESP buffer
    #[error("protocol: cannot parse boolean")]
    CannotParseBoolean,
    /// Raised when cannot parse char from the RESP buffer
    #[error("protocol: cannot parse char")]
    CannotParseChar,
    /// Raised when cannot parse str from the RESP buffer
    #[error("protocol: cannot parse str")]
    CannotParseStr,
    /// Raised when cannot parse string from the RESP buffer
    #[error("protocol: cannot parse string")]
    CannotParseString,
    /// Raised when cannot parse sequence from the RESP buffer
    #[error("protocol: cannot parse sequence")]
    CannotParseSequence,
    /// Raised when cannot parse map from the RESP buffer
    #[error("protocol: cannot parse map")]
    CannotParseMap,
    /// Raised when cannot parse struct from the RESP buffer
    #[error("protocol: cannot parse struct")]
    CannotParseStruct,
    /// Raised when cannot parse bytes from the RESP buffer
    #[error("protocol: cannot parse bytes")]
    CannotParseBytes,
    /// Raised when cannot parse enum from the RESP buffer
    #[error("protocol: cannot parse enum")]
    CannotParseEnum,
    /// Raised when verbatim string is too short
    #[error("protocol: verbatim string too short")]
    VerbatimStringTooShort,
    /// Raised when an unknown RESP tag is encountered
    #[error("protocol: unknown RESP tag {0}")]
    UnknownRespTag(char),
    /// Raised when disconnected from the server
    #[error("disconnected from server")]
    DisconnectedFromServer,
    /// Raised when an invalid channel to send messages to the network handler is used
    #[error("invalid channel to send messages to the network handler")]
    InvalidChannel,
    /// Raised when [`Client::into_exclusive`](crate::client::Client::into_exclusive)
    /// is called while another handle on the same connection is still alive, so
    /// the connection an [`ExclusiveClient`](crate::client::ExclusiveClient)
    /// would claim as its own is in fact shared
    #[error("client is not the sole handle on its connection")]
    NotExclusive,
    /// Raised when client is already subscribed to the given channel/pattern
    #[error("client is already subscribed to the given channel/pattern")]
    AlreadySubscribed,
    /// Raised when the server sends a subscription confirmation that does not
    /// match the pending subscription request (out-of-order or spurious ack)
    #[error("unexpected subscription confirmation from server")]
    UnexpectedSubscriptionConfirmation,
    /// Raised when a push frame routed to a pub/sub stream is not one of the
    /// three message shapes a subscriber can be handed (`message`, `smessage`
    /// or `pmessage`)
    #[error("unexpected pub/sub message from server")]
    UnexpectedPubSubMessage,
    /// Raised when serde serialization error occurs
    #[error("Serde deserialization error: {0}")]
    SerdeDeserialize(String),
    /// Raised when serde serialization error occurs
    #[error("Serde serialization error: {0}")]
    SerdeSerialize(String),
    /// Raised when a command groups its arguments by a step of zero, which names
    /// no group at all. The step is a caller-supplied width on the builder's
    /// `*_with_count_and_step` methods, so the command carries the error instead of
    /// dividing its argument count by zero.
    #[error("command args: a group step of zero is not a valid grouping")]
    InvalidArgumentGroupStep,
    /// Raised when a command has been retried up to `Config::max_command_attempts`
    /// without succeeding, so it is failed instead of retried indefinitely.
    #[error("command failed after reaching the maximum number of attempts")]
    MaxCommandAttemptsReached,
    /// Raised when the send queue has reached `Config::backpressure.max_queued_bytes`,
    /// so an incoming command is shed instead of growing the queue further.
    ///
    /// This means the connection is down and the queue of commands waiting for it
    /// is full. It is distinct from
    /// [`DisconnectedByPeer`](crate::ErrorKind::DisconnectedByPeer), which means the
    /// command was dropped because it opted out of retries, and from
    /// [`MaxCommandAttemptsReached`](Self::MaxCommandAttemptsReached), which means
    /// the command was retried and kept failing. Only a *new* command is refused:
    /// one already queued, or replayed after a reconnection or a redirection,
    /// never is.
    #[error("send queue is full")]
    SendQueueFull,
    #[error("a client-side cache key must serialize to exactly one argument")]
    InvalidCacheKey,
    /// Raised when a key argument does not serialize to the number of command
    /// arguments its position allows.
    ///
    /// Command arguments are `impl Serialize`, so the compiler cannot check how
    /// many arguments a value produces: `None` and an empty collection produce
    /// none, a struct or a sequence produces one per element. A key that
    /// produced none carries no hash slot, which in Cluster mode routes the
    /// command to a random node instead of the node that owns it. The arity is
    /// therefore checked where the key is added, and the message names the
    /// command and the count.
    #[error("{command}: a key argument serialized to {written} command arguments, but {expected}")]
    InvalidKeyArity {
        /// Name of the command being built.
        command: String,
        /// How many command arguments the key actually serialized to.
        written: usize,
        /// What the key's position allows, as it reads in the message.
        expected: &'static str,
    },
    /// Raised when cannot parse hash slot
    #[error("cannot parse hash slot")]
    CannotParseHashSlot,
    /// Raised when cannot parse address
    #[error("cannot parse address")]
    CannotParseAddress,
    /// Raised when cannot parse port
    #[error("cannot parse port")]
    CannotParsePort,
    /// Raised when cannot parse RequestPolicy
    #[error("Cannot parse RequestPolicy")]
    CannotParseRequestPolicy,
    /// Raised when cannot parse ResponsePolicy
    #[error("Cannot parse ResponsePolicy")]
    CannotParseResponsePolicy,
    /// Raised if an error occurs in the [`Config`](crate::client::Config) parsing
    #[error("Cannot parse config")]
    ConfigParseError,
    /// Raised when a connection URI carries a query parameter that is unknown or
    /// whose value cannot be parsed. The message names the offending parameter.
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    /// Raised at connection time when a [`Config`](crate::client::Config) knob
    /// holds a value that would disable behavior rather than tune it — a zero
    /// buffer capacity, a zero loop bound. The message names the offending knob.
    #[error("Invalid config: {0}")]
    InvalidConfig(&'static str),
    /// Raised when the client's own routing state stops agreeing with itself — a
    /// node index or a pending-request index that no longer addresses anything.
    /// Unreachable by construction: every such index is produced by scanning the
    /// very collection it is then used on. It is reported rather than asserted
    /// because these lookups happen on the network task, where a panic would take
    /// down every in-flight command and the reconnection loop with them.
    #[error("inconsistent internal routing state")]
    InconsistentRoutingState,
    /// Raised if an error occurs in the [`ClusterConfig`](crate::client::ClusterConfig)
    #[error("Cluster misconfiguration")]
    ClusterConfig,
    /// Raised when EXEC is called without MULTI
    #[error("EXEC called without MULTI")]
    ExecCalledWithoutMulti,
    /// Raised when a transaction mixes keys belonging to different hash slots,
    /// which Redis Cluster cannot execute atomically
    #[error("CROSSSLOT Keys in request don't hash to the same slot")]
    CrossSlot,
    /// Raised when a command is not supported in cluster mode
    #[error("Command not supported in cluster mode")]
    CommandNotSupportedInCluster,
    /// Raised when an unexpected message is received
    #[error("Unexpected message received")]
    UnexpectedMessageReceived,
    /// Raised when keys hash slots do not match
    #[error("Keys hash slots do not match")]
    MismatchedKeySlots,
    /// Raised when cannot parse Redis server version
    #[error("Cannot parse Redis server version")]
    CannotParseRedisServerVersion,
    /// Raised when a RESP frame nests collections deeper than the parser allows,
    /// guarding against a crafted reply driving the parser into a stack overflow.
    #[error("protocol: maximum nesting depth exceeded")]
    MaxNestingDepthExceeded,
    /// Raised when a bulk string / bulk error / verbatim string declares a
    /// length beyond the parser's configured ceiling, before the payload is
    /// trusted — stops a crafted header from driving unbounded buffering.
    #[error("protocol: bulk length exceeds the maximum allowed")]
    BulkLengthTooLarge,
    /// Raised when a collection (array / set / push / map) declares a cardinality
    /// beyond the parser's configured ceiling.
    #[error("protocol: collection length exceeds the maximum allowed")]
    CollectionLengthTooLarge,
    /// Raised when the frame parser ends in a state its own grammar forbids —
    /// a closed frame that is not a collection, or a parse loop that ran out of
    /// work with no frame produced. The reader is at an unknown offset, so the
    /// connection is what fails, not the command.
    #[error("protocol: the frame parser ended in an impossible state")]
    MalformedFrame,
    /// Raised when a decoded frame indexes outside the buffer or the tape that
    /// describes it. The frame is bounded, so this fails one command.
    #[error("protocol: the decoded frame and its tape disagree")]
    InconsistentRespTape,
    /// Raised when a reply is read as a collection and is a scalar.
    #[error("protocol: the reply is not a collection")]
    NotACollection,
    /// Raised when a transaction's reply batch holds no answer to `EXEC`.
    #[error("the transaction reply carries no answer to EXEC")]
    MissingTransactionReply,
    /// Raised when the nodes of a cluster answer shapes that the command's
    /// response policy cannot combine — an integer against an array, or arrays
    /// of different lengths.
    #[error("cluster: the shards answered shapes that cannot be aggregated")]
    IncompatibleShardReplies,
    /// Raised when an enum is deserialized as a unit variant from a reply that
    /// carries a payload.
    #[error("the reply carries a payload, so it is not a unit variant")]
    NotAUnitVariant,
    /// Raised when a map is deserialized and a field arrives with no value.
    #[error("the map holds a field with no value")]
    MissingMapValue,
}

impl ClientError {
    /// Whether this error was raised while framing the byte stream, as opposed
    /// to while decoding an already-framed reply into the caller's type.
    ///
    /// A framing failure leaves the reader at an unknown offset: the bytes that
    /// follow can no longer be attributed to any command, so the connection —
    /// not the caller at the head of the receive queue — is what the error
    /// belongs to. A decode failure happens past that point, on a frame whose
    /// bounds are known, and fails exactly one command.
    ///
    /// The match is exhaustive on purpose, with no wildcard arm: a new variant
    /// does not compile until it has been classified here. An allow-list with a
    /// default would silently make every future framing failure a per-command
    /// one, which is the reading that risks leaving the stream desynchronised.
    #[inline]
    pub(crate) fn is_framing_error(&self) -> bool {
        match self {
            // Raised by the frame parser, on bytes whose boundaries are not yet
            // known: the reader cannot be placed at the next reply.
            ClientError::CannotParseInteger
            | ClientError::CannotParseDouble
            | ClientError::CannotParseBulkString
            | ClientError::CannotParseBulkError
            | ClientError::CannotParseVerbatimString
            | ClientError::CannotParseBoolean
            | ClientError::CannotParseMap
            | ClientError::CannotParseSequence
            | ClientError::UnknownRespTag(_)
            | ClientError::InvalidTag
            | ClientError::BulkLengthTooLarge
            | ClientError::CollectionLengthTooLarge
            | ClientError::MaxNestingDepthExceeded
            | ClientError::VerbatimStringTooShort
            | ClientError::MalformedFrame => true,

            // Raised past framing — while decoding a bounded frame into the
            // caller's type, while routing, or on the caller's own input — so
            // exactly one command fails and the stream stays usable.
            ClientError::ExpectedArrayForMGet
            | ClientError::CannotParseNil
            | ClientError::CannotParseChar
            | ClientError::CannotParseStr
            | ClientError::CannotParseString
            | ClientError::CannotParseStruct
            | ClientError::CannotParseBytes
            | ClientError::CannotParseEnum
            | ClientError::DisconnectedFromServer
            | ClientError::InvalidChannel
            | ClientError::NotExclusive
            | ClientError::AlreadySubscribed
            | ClientError::UnexpectedSubscriptionConfirmation
            | ClientError::UnexpectedPubSubMessage
            | ClientError::SerdeDeserialize(_)
            | ClientError::SerdeSerialize(_)
            | ClientError::InvalidArgumentGroupStep
            | ClientError::MaxCommandAttemptsReached
            | ClientError::SendQueueFull
            | ClientError::InvalidCacheKey
            | ClientError::InvalidKeyArity { .. }
            | ClientError::CannotParseHashSlot
            | ClientError::CannotParseAddress
            | ClientError::CannotParsePort
            | ClientError::CannotParseRequestPolicy
            | ClientError::CannotParseResponsePolicy
            | ClientError::ConfigParseError
            | ClientError::InvalidUri(_)
            | ClientError::InvalidConfig(_)
            | ClientError::InconsistentRoutingState
            | ClientError::ClusterConfig
            | ClientError::ExecCalledWithoutMulti
            | ClientError::CrossSlot
            | ClientError::CommandNotSupportedInCluster
            | ClientError::UnexpectedMessageReceived
            | ClientError::MismatchedKeySlots
            | ClientError::CannotParseRedisServerVersion
            | ClientError::InconsistentRespTape
            | ClientError::NotACollection
            | ClientError::MissingTransactionReply
            | ClientError::IncompatibleShardReplies
            | ClientError::NotAUnitVariant
            | ClientError::MissingMapValue => false,
        }
    }
}

/// Which deadline expired, in an [`ErrorKind::Timeout`].
///
/// The two demand opposite answers, so they are told apart by the type rather
/// than by whether the error happens to name a command: a connect timeout says
/// this server did not become usable — try another one, or wait for the
/// reconnection — while a command timeout says this one request did not get its
/// reply in time, on a connection that may well be healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TimeoutKind {
    /// [`Config::connect_timeout`](crate::client::Config::connect_timeout)
    /// expired: the connection, or its handshake, did not complete.
    #[error("the connect timeout expired before the connection was usable")]
    Connect,
    /// [`Config::command_timeout`](crate::client::Config::command_timeout)
    /// expired: the command was sent and its reply did not arrive in time. The
    /// command may still have run.
    #[error("the command timeout expired before the reply arrived")]
    Command,
}

/// What an [`struct@Error`] is, independently of the command it belongs to.
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Raised if an error occurs within the driver
    #[error("client error: {0}")]
    Client(#[from] ClientError),
    /// Raised if a required cache key is in the wrong type
    #[error("cache wrong key type")]
    CacheWrongKeyType,
    /// A transaction has been aborted
    #[error("transaction aborted")]
    Aborted,
    /// Raised if an error occurs when contacting Sentinel instances
    #[error("sentinel error: {0}")]
    Sentinel(String),
    /// Error returned by the Redis server
    #[error("redis server error: {0}")]
    Redis(#[from] RedisError),
    /// IO error when connecting the Redis server
    #[error("io error: {0}")]
    IO(Arc<std::io::Error>),
    /// Raised by the TLS library
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    #[error("tls error: {0}")]
    Tls(Arc<native_tls::Error>),
    /// Raised by the TLS library
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(feature = "rustls")]
    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),
    /// Invalid Dns name (rustls)
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(feature = "rustls")]
    #[error("invalid dns name: {0}")]
    InvalidDnsName(Arc<rustls::pki_types::InvalidDnsNameError>),
    /// A deadline expired before the operation completed
    #[error("{0}")]
    Timeout(TimeoutKind),
    /// Internal error to trigger retry sending the command
    #[doc(hidden)]
    #[error("Retry")]
    Retry(SmallVec<[RetryReason; 1]>),
    /// Raised when end of stream is reached
    #[error("End of stream reached")]
    EOF,
    /// Raised when a tokio join error occurs
    #[cfg(feature = "tokio-runtime")]
    #[error("tokio join error: {0}")]
    TokioJoin(Arc<tokio::task::JoinError>),
    /// Raised when oneshot channel is canceled
    #[error("oneshot channel canceled")]
    OneshotCanceled(#[from] tokio::sync::oneshot::error::RecvError),
    /// Raised when mpsc send error occurs
    #[error("mpsc send error: {0}")]
    MpscSend(#[from] mpsc::SendError),
    /// Raised when UTF-8 error occurs
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] Utf8Error),
    /// Raised when FromUtf8 error occurs
    #[error("FromUtf8 error: {0}")]
    FromUtf8(#[from] FromUtf8Error),
    /// Raised when parse float error occurs
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] ParseFloatError),
    /// Raised when parse int error occurs
    #[error("Parse int error: {0}")]
    ParseInt(#[from] ParseIntError),
    /// Raised when tokio broadcast send error occurs
    #[error("Tokio broadcast send error: {0}")]
    TokioBroadcastSend(Arc<tokio::sync::broadcast::error::SendError<()>>),
    /// Disconnected by peer
    #[error("Disconnected by peer")]
    DisconnectedByPeer,
}

impl From<tokio::sync::broadcast::error::SendError<()>> for ErrorKind {
    fn from(value: tokio::sync::broadcast::error::SendError<()>) -> Self {
        ErrorKind::TokioBroadcastSend(Arc::new(value))
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(value: std::io::Error) -> Self {
        ErrorKind::IO(Arc::new(value))
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
#[cfg(feature = "native-tls")]
impl From<native_tls::Error> for ErrorKind {
    fn from(value: native_tls::Error) -> Self {
        ErrorKind::Tls(Arc::new(value))
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
#[cfg(feature = "rustls")]
impl From<rustls::pki_types::InvalidDnsNameError> for ErrorKind {
    fn from(value: rustls::pki_types::InvalidDnsNameError) -> Self {
        ErrorKind::InvalidDnsName(Arc::new(value))
    }
}

#[cfg(feature = "tokio-runtime")]
impl From<tokio::task::JoinError> for ErrorKind {
    fn from(value: tokio::task::JoinError) -> Self {
        ErrorKind::TokioJoin(Arc::new(value))
    }
}

/// Identifies the command an [`struct@Error`] belongs to.
///
/// A multiplexed client has many commands in flight at once, so an error that
/// names none of them cannot be correlated to the application code that issued
/// it. Every error the client raises on behalf of a command carries one of
/// these.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The command name, as a slice of the command buffer it was sent from.
    command: Bytes,
}

impl ErrorContext {
    /// The name of the command the error belongs to, as sent on the wire
    /// (`GET`, `EVALSHA`…).
    ///
    /// In a pipeline or a transaction, an error that fails the batch as a whole
    /// — a timeout, a lost connection, a full send queue — is named after the
    /// batch's first command, since all of them died together. An error born in
    /// the reply of one queued command is named after that command, and only
    /// when a single reply is awaited: past that, the batch deserializer reports
    /// on the whole tuple and does not say which element it stumbled on, so the
    /// error carries no command rather than the wrong one.
    #[must_use]
    pub fn command(&self) -> &str {
        // Command names come from `&'static str` literals, so this never fails;
        // an empty name is nonetheless a better answer here than a panic.
        std::str::from_utf8(&self.command).unwrap_or_default()
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.command())
    }
}

/// Any error raised by the client, and the command it belongs to.
///
/// Match on [`kind`](Error::kind) to tell errors apart, and read
/// [`command`](Error::command) to know which command produced it:
///
/// ```
/// # use rustis::{Error, ErrorKind, Result};
/// # fn handle(result: Result<String>) {
/// if let Err(e) = result {
///     if matches!(e.kind(), ErrorKind::Timeout(_)) {
///         eprintln!("{:?} timed out", e.command());
///     }
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    /// Boxed so an `Error` costs one pointer more than its kind, and so the
    /// common case — an error raised before any command is known — allocates
    /// nothing.
    context: Option<Box<ErrorContext>>,
}

impl Error {
    /// What the error is.
    #[must_use]
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Consumes the error and yields its kind, for matching by value.
    #[must_use]
    pub fn into_kind(self) -> ErrorKind {
        self.kind
    }

    /// The command the error belongs to, when the client knows it.
    ///
    /// `None` for an error raised outside any command, such as a connection
    /// timeout.
    #[must_use]
    pub fn context(&self) -> Option<&ErrorContext> {
        self.context.as_deref()
    }

    /// The name of the command the error belongs to — shorthand for
    /// [`context().map(ErrorContext::command)`](Error::context).
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        self.context.as_ref().map(|c| c.command())
    }

    /// Whether the connection to the server is what failed: the transport
    /// broke, the peer went away, or a reply could not be framed.
    ///
    /// True for a transport or TLS failure, an end of stream, a disconnection,
    /// the loss of the network task, and a RESP framing failure — the last one
    /// because a stream the parser lost track of cannot carry another command,
    /// so the client drops the connection and reconnects. False for anything the
    /// server answered ([`is_server_error`](Error::is_server_error)), for a
    /// timeout ([`is_timeout`](Error::is_timeout)), and for a decode failure on
    /// a well-framed reply, which fails one command only.
    ///
    /// The command may or may not have run: the answer, if any, was lost with
    /// the connection.
    #[must_use]
    pub fn is_connection_error(&self) -> bool {
        match &self.kind {
            ErrorKind::IO(_)
            | ErrorKind::EOF
            | ErrorKind::DisconnectedByPeer
            | ErrorKind::OneshotCanceled(_)
            | ErrorKind::MpscSend(_) => true,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            ErrorKind::Tls(_) => true,
            #[cfg(feature = "rustls")]
            ErrorKind::InvalidDnsName(_) => true,
            #[cfg(feature = "tokio-runtime")]
            ErrorKind::TokioJoin(_) => true,
            ErrorKind::Client(client_error) => client_error.is_framing_error(),
            _ => false,
        }
    }

    /// Whether a deadline expired before the operation completed.
    ///
    /// Covers both [`Config::connect_timeout`](crate::client::Config::connect_timeout)
    /// and [`Config::command_timeout`](crate::client::Config::command_timeout);
    /// [`command`](Error::command) tells them apart, being `None` for a
    /// connection that never got to send anything. A blocking command reaching
    /// its own server-side timeout is not an error at all — it replies nil, so
    /// it arrives as `None`.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        matches!(self.kind, ErrorKind::Timeout(_))
    }

    /// Whether the server answered, and answered an error.
    ///
    /// The connection is healthy and the command reached the server: what
    /// failed is the command itself — a wrong type, a missing script, a refused
    /// authentication. Match on
    /// [`RedisError::kind`](crate::RedisError) for the exact code.
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Redis(_))
    }

    /// Whether the failure is transient, so that sending the command again may
    /// succeed.
    ///
    /// True for every [connection error](Error::is_connection_error), for a
    /// [timeout](Error::is_timeout), and for the server codes that ask for a
    /// replay: `TRYAGAIN`, `CLUSTERDOWN`, `MASTERDOWN` and `NOMASTERLINK`.
    ///
    /// # Warning
    ///
    /// Transient does not mean the command did not run. A connection that dies
    /// or a deadline that expires after the server applied the write leaves no
    /// way to tell it apart from one that died before. Replay only commands
    /// that are safe to apply twice, or make them idempotent first — `INCR`
    /// replayed on a lost reply counts twice.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        if self.is_connection_error() || self.is_timeout() {
            return true;
        }

        matches!(
            &self.kind,
            ErrorKind::Redis(RedisError {
                kind: RedisErrorKind::TryAgain
                    | RedisErrorKind::ClusterDown
                    | RedisErrorKind::MasterDown
                    | RedisErrorKind::NoMasterLink,
                ..
            })
        )
    }

    /// Names the command this error belongs to, unless one is already named.
    ///
    /// The site closest to the cause holds the most precise command, so the
    /// outer layers it bubbles through leave it alone.
    #[must_use]
    pub(crate) fn with_command(mut self, command: Bytes) -> Self {
        if self.context.is_none() {
            self.context = Some(Box::new(ErrorContext { command }));
        }
        self
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.kind, f)?;
        if let Some(context) = &self.context {
            write!(f, " (while executing {context})")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error {
            kind,
            context: None,
        }
    }
}

/// Forwards to `ErrorKind`'s own conversion, so `?` keeps working on the
/// foreign error types the client builds upon.
macro_rules! error_from {
    ($($(#[$meta:meta])* $ty:ty),* $(,)?) => {
        $(
            $(#[$meta])*
            impl From<$ty> for Error {
                fn from(value: $ty) -> Self {
                    Error::from(ErrorKind::from(value))
                }
            }
        )*
    };
}

error_from! {
    ClientError,
    RedisError,
    std::io::Error,
    Utf8Error,
    FromUtf8Error,
    ParseFloatError,
    ParseIntError,
    tokio::sync::oneshot::error::RecvError,
    mpsc::SendError,
    tokio::sync::broadcast::error::SendError<()>,
    #[cfg(feature = "tokio-runtime")]
    tokio::task::JoinError,
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    native_tls::Error,
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(feature = "rustls")]
    rustls::Error,
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(feature = "rustls")]
    rustls::pki_types::InvalidDnsNameError,
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::from(ClientError::SerdeDeserialize(msg.to_string()))
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::from(ClientError::SerdeSerialize(msg.to_string()))
    }
}

/// Redis server error kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RedisErrorKind {
    Ask {
        hash_slot: u16,
        address: (String, u16),
    },
    BusyGroup,
    ClusterDown,
    CrossSlot,
    Err,
    InProg,
    IoErr,
    MasterDown,
    MisConf,
    Moved {
        hash_slot: u16,
        address: (String, u16),
    },
    NoAuth,
    NoGoodSlave,
    NoMasterLink,
    NoPerm,
    NoProto,
    NoQuorum,
    NotBusy,
    NoScript,
    OutOfMemory,
    Readonly,
    TryAgain,
    UnKillable,
    Unblocked,
    WrongPass,
    WrongType,
    Other,
}

impl RedisErrorKind {
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`index` is a separator position found inside `address`, so stepping \
                  past it stays an offset into the slice."
    )]
    fn parse_hash_slot_and_address(
        hash_slot: &[u8],
        address: &[u8],
    ) -> Result<(u16, (String, u16))> {
        let hash_slot = atoi(hash_slot).ok_or(Error::from(ClientError::CannotParseHashSlot))?;
        // Split at the last colon: IPv6 hosts contain colons, and Redis emits
        // bare `host:port` with no brackets, so only the rightmost colon
        // reliably separates the port.
        let index = address
            .iter()
            .rposition(|b| *b == b':')
            .ok_or(Error::from(ClientError::CannotParseAddress))?;
        let (host, port) = (&address[..index], &address[index + 1..]);
        let port = atoi(port).ok_or(Error::from(ClientError::CannotParsePort))?;
        Ok((hash_slot, (String::from_utf8_lossy(host).to_string(), port)))
    }
}

impl<'a> TryFrom<&'a [u8]> for RedisErrorKind {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> std::result::Result<Self, Self::Error> {
        match value {
            b"BUSYGROUP" => Ok(Self::BusyGroup),
            b"CLUSTERDOWN" => Ok(Self::ClusterDown),
            b"CROSSSLOT" => Ok(Self::CrossSlot),
            b"ERR" => Ok(Self::Err),
            b"INPROG" => Ok(Self::InProg),
            b"IOERR" => Ok(Self::IoErr),
            b"MASTERDOWN" => Ok(Self::MasterDown),
            b"MISCONF" => Ok(Self::MisConf),
            b"NOAUTH" => Ok(Self::NoAuth),
            b"NOGOODSLAVE" => Ok(Self::NoGoodSlave),
            b"NOMASTERLINK" => Ok(Self::NoMasterLink),
            b"NOPERM" => Ok(Self::NoPerm),
            b"NOPROTO" => Ok(Self::NoProto),
            b"NOQUORUM" => Ok(Self::NoQuorum),
            b"NOTBUSY" => Ok(Self::NotBusy),
            b"NOSCRIPT" => Ok(Self::NoScript),
            b"OOM" => Ok(Self::OutOfMemory),
            b"READONLY" => Ok(Self::Readonly),
            b"TRYAGAIN" => Ok(Self::TryAgain),
            b"UNKILLABLE" => Ok(Self::UnKillable),
            b"UNBLOCKED" => Ok(Self::Unblocked),
            b"WRONGPASS" => Ok(Self::WrongPass),
            b"WRONGTYPE" => Ok(Self::WrongType),
            _ => {
                let mut iter = value.split(u8::is_ascii_whitespace);
                match (iter.next(), iter.next(), iter.next(), iter.next()) {
                    (Some(b"ASK"), Some(hash_slot), Some(address), None) => {
                        Self::parse_hash_slot_and_address(hash_slot, address)
                            .map(|(hash_slot, address)| Self::Ask { hash_slot, address })
                    }
                    (Some(b"MOVED"), Some(hash_slot), Some(address), None) => {
                        Self::parse_hash_slot_and_address(hash_slot, address)
                            .map(|(hash_slot, address)| Self::Moved { hash_slot, address })
                    }
                    _ => Ok(Self::Other),
                }
            }
        }
    }
}

impl Display for RedisErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisErrorKind::Ask {
                hash_slot,
                address: (host, port),
            } => f.write_fmt(format_args!("ASK {} {}:{}", *hash_slot, *host, *port)),
            RedisErrorKind::BusyGroup => f.write_str("BUSYGROUP"),
            RedisErrorKind::ClusterDown => f.write_str("CLUSTERDOWN"),
            RedisErrorKind::CrossSlot => f.write_str("CROSSSLOT"),
            RedisErrorKind::Err => f.write_str("ERR"),
            RedisErrorKind::InProg => f.write_str("INPROG"),
            RedisErrorKind::IoErr => f.write_str("IOERR"),
            RedisErrorKind::MasterDown => f.write_str("MASTERDOWN"),
            RedisErrorKind::MisConf => f.write_str("MISCONF"),
            RedisErrorKind::Moved {
                hash_slot,
                address: (host, port),
            } => f.write_fmt(format_args!("MOVED {} {}:{}", *hash_slot, *host, *port)),
            RedisErrorKind::NoAuth => f.write_str("NOAUTH"),
            RedisErrorKind::NoGoodSlave => f.write_str("NOGOODSLAVE"),
            RedisErrorKind::NoMasterLink => f.write_str("NOMASTERLINK"),
            RedisErrorKind::NoPerm => f.write_str("NOPERM"),
            RedisErrorKind::NoProto => f.write_str("NOPROTO"),
            RedisErrorKind::NoQuorum => f.write_str("NOQUORUM"),
            RedisErrorKind::NotBusy => f.write_str("NOTBUSY"),
            RedisErrorKind::NoScript => f.write_str("NOSCRIPT"),
            RedisErrorKind::OutOfMemory => f.write_str("OOM"),
            RedisErrorKind::Readonly => f.write_str("READONLY"),
            RedisErrorKind::TryAgain => f.write_str("TRYAGAIN"),
            RedisErrorKind::UnKillable => f.write_str("UNKILLABLE"),
            RedisErrorKind::Unblocked => f.write_str("UNBLOCKED"),
            RedisErrorKind::WrongPass => f.write_str("WRONGPASS"),
            RedisErrorKind::WrongType => f.write_str("WRONGTYPE"),
            RedisErrorKind::Other => f.write_str(""),
        }
    }
}

/// Error issued by the Redis server
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
pub struct RedisError {
    pub kind: RedisErrorKind,
    pub description: String,
}

impl<'a> TryFrom<&'a [u8]> for RedisError {
    type Error = Error;

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`i` is a separator position found inside `error`, so stepping past \
                  it stays an offset into the slice."
    )]
    fn try_from(error: &'a [u8]) -> std::result::Result<Self, Self::Error> {
        match error
            .iter()
            .position(|b| *b == b' ')
            .map(|i| (&error[..i], &error[i + 1..]))
        {
            Some((b"ASK", _)) => Ok(Self {
                kind: RedisErrorKind::try_from(error)?,
                description: "".to_owned(),
            }),
            Some((b"MOVED", _)) => Ok(Self {
                kind: RedisErrorKind::try_from(error)?,
                description: "".to_owned(),
            }),
            Some((kind, description)) => {
                let kind = RedisErrorKind::try_from(kind)?;

                let description = if let RedisErrorKind::Other = kind {
                    error
                } else {
                    description
                };

                Ok(Self {
                    kind,
                    description: String::from_utf8_lossy(description).to_string(),
                })
            }
            None => Ok(Self {
                kind: RedisErrorKind::Other,
                description: String::from_utf8_lossy(error).to_string(),
            }),
        }
    }
}

impl Display for RedisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.kind, self.description))
    }
}
