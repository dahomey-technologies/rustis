use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{HelloOptions, HelloResult},
    resp::{Response, cmd},
};

/// The commands that drive the connection itself, and which the client sends on
/// the caller's behalf rather than at the caller's request.
///
/// These are not hidden for tidiness. A `Client` is clonable and multiplexed, so
/// one connection carries every clone's commands, and each of these either
/// reconfigures that shared connection underneath the others or contradicts a
/// decision the client has already made:
///
/// - `HELLO` sets the protocol version. The handshake fixes it at RESP3 and every
///   deserializer is written against RESP3 — push frames for pub/sub, maps,
///   doubles, `_` for nil. A caller switching to RESP2 mid-session leaves the
///   decoder reading a protocol the server is no longer speaking, and since the
///   switch is not connection state the client records, a reconnection silently
///   returns to RESP3.
/// - `READONLY` / `READWRITE` set the read mode of a cluster connection, which is
///   the mechanism behind
///   [`ClusterConfig::read_preference`](crate::client::ClusterConfig::read_preference).
///   A caller flipping it takes a replica out of the mode the routing depends on.
/// - `ASKING` arms one node for the redirection that follows it, so it is only
///   correct immediately before the redirected command, on that node.
/// - `CLUSTER SLOTS` is the pre-7.0 spelling of `CLUSTER SHARDS`, kept because
///   topology discovery falls back to it against an older server. Redis marks it
///   deprecated; a caller wanting the topology wants `cluster_shards`.
///
/// Refusing them at run time was the alternative. Not exposing them is better: the
/// caller learns from the compiler instead of from a production incident. The
/// generic command API can still send any of them, which is the documented escape
/// hatch and stays the caller's responsibility.
pub(crate) trait InternalCommands<'a>: Sized {
    /// When a cluster client receives an -ASK redirect,
    /// the ASKING command is sent to the target node followed by the command which was redirected.
    ///
    /// # See Also
    /// [<https://redis.io/commands/asking/>](https://redis.io/commands/asking/)
    #[must_use]
    fn asking(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("ASKING"))
    }

    /// This command returns details about which cluster slots map to which Redis instances.
    ///
    /// Deprecated by Redis 7.0 in favour of `CLUSTER SHARDS`, and used only where
    /// that one does not exist: topology discovery against a pre-7.0 server.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-slots/>](https://redis.io/commands/cluster-slots/)
    fn cluster_slots<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("SLOTS"))
    }

    /// Switch to a different protocol,
    /// optionally authenticating and setting the connection's name,
    /// or provide a contextual client report.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hello/>](https://redis.io/commands/hello/)
    #[must_use]
    fn hello(self, options: HelloOptions) -> PreparedCommand<'a, Self, HelloResult> {
        prepare_command(self, cmd("HELLO").arg(options))
    }

    /// Enables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readonly/>](https://redis.io/commands/readonly/)
    #[must_use]
    fn readonly(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("READONLY"))
    }

    /// Disables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readwrite/>](https://redis.io/commands/readwrite/)
    #[must_use]
    fn readwrite(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("READWRITE"))
    }
}
