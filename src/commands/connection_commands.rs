use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CommandArg, CommandArgs, FromValue, IntoArgs, SingleArgOrCollection, Value,
    },
    Error, Result,
};
use std::collections::HashMap;

/// A group of Redis commands related to connection management
///
/// # See Also
/// [Redis Connection Management Commands](https://redis.io/commands/?group=connection)
pub trait ConnectionCommands {
    /// Authenticates the current connection.
    ///
    /// # Errors
    /// a Redis error if the password, or username/password pair, is invalid.
    ///
    /// # See Also
    /// [<https://redis.io/commands/auth/>](https://redis.io/commands/auth/)
    #[must_use]
    fn auth<U, P>(&mut self, username: Option<U>, password: P) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        U: Into<CommandArg>,
        P: Into<CommandArg>,
    {
        prepare_command(self, cmd("AUTH").arg(username).arg(password))
    }

    /// This command controls the tracking of the keys in the next command executed by the connection,
    /// when tracking is enabled in OPTIN or OPTOUT mode.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-caching/>](https://redis.io/commands/client-caching/)
    #[must_use]
    fn client_caching(&mut self, mode: ClientCachingMode) -> PreparedCommand<Self, Option<()>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("CACHING").arg(mode))
    }

    /// Returns the name of the current connection as set by [CLIENT SETNAME].
    ///
    /// # Return
    /// The connection name, or a None if no name is set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-getname/>](https://redis.io/commands/client-getname/)
    #[must_use]
    fn client_getname<CN>(&mut self) -> PreparedCommand<Self, Option<CN>>
    where
        Self: Sized,
        CN: FromValue,
    {
        prepare_command(self, cmd("CLIENT").arg("GETNAME"))
    }

    /// This command returns the client ID we are redirecting our tracking notifications to.
    ///
    /// # Return
    /// the ID of the client we are redirecting the notifications to.
    /// The command returns -1 if client tracking is not enabled,
    /// or 0 if client tracking is enabled but we are not redirecting the notifications to any client.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-getredir/>](https://redis.io/commands/client-getredir/)
    #[must_use]
    fn client_getredir(&mut self) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("GETREDIR"))
    }

    /// The command just returns the ID of the current connection.
    ///
    /// # Return
    /// The id of the client.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-id/>](https://redis.io/commands/client-id/)
    #[must_use]
    fn client_id(&mut self) -> PreparedCommand<Self, i64>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("ID"))
    }

    /// The command returns information and statistics about the current client connection
    /// in a mostly human readable format.
    ///
    /// # Return
    /// A ClientInfo struct with additional properties
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-info/>](https://redis.io/commands/client-info/)
    #[must_use]
    fn client_info(&mut self) -> PreparedCommand<Self, ClientInfo>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("INFO"))
    }

    /// Closes a given clients connection based on a filter list
    ///
    /// # Return
    /// the number of clients killed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-kill/>](https://redis.io/commands/client-kill/)
    #[must_use]
    fn client_kill(&mut self, options: ClientKillOptions) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("KILL").arg(options))
    }

    /// Returns information and statistics about the client connections server in a mostly human readable format.
    ///
    /// # Return
    /// A Vec of ClientInfo structs with additional properties
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-list/>](https://redis.io/commands/client-list/)
    #[must_use]
    fn client_list(&mut self, options: ClientListOptions) -> PreparedCommand<Self, ClientListResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("LIST").arg(options))
    }

    ///  sets the [`client eviction`](https://redis.io/docs/reference/clients/#client-eviction) mode for the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-no-evict/>](https://redis.io/commands/client-no-evict/)
    #[must_use]
    fn client_no_evict(&mut self, no_evict: bool) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("CLIENT")
                .arg("NO-EVICT")
                .arg(if no_evict { "ON" } else { "OFF" }),
        )
    }

    /// Connections control command able to suspend all the Redis clients
    /// for the specified amount of time (in milliseconds).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-pause/>](https://redis.io/commands/client-pause/)
    #[must_use]
    fn client_pause(&mut self, timeout: u64, mode: ClientPauseMode) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("PAUSE").arg(timeout).arg(mode))
    }

    /// Sometimes it can be useful for clients to completely disable replies from the Redis server.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-reply/>](https://redis.io/commands/client-reply/)
    #[must_use]
    fn client_reply(&mut self, mode: ClientReplyMode) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("REPLY").arg(mode))
    }

    /// Assigns a name to the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-setname/>](https://redis.io/commands/client-setname/)
    #[must_use]
    fn client_setname<CN>(&mut self, connection_name: CN) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        CN: Into<CommandArg>,
    {
        prepare_command(self, cmd("CLIENT").arg("SETNAME").arg(connection_name))
    }

    /// This command enables the tracking feature of the Redis server,
    /// that is used for [`server assisted client side caching`](https://redis.io/topics/client-side-caching).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-tracking/>](https://redis.io/commands/client-tracking/)
    #[must_use]
    fn client_tracking(
        &mut self,
        status: ClientTrackingStatus,
        options: ClientTrackingOptions,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("TRACKING").arg(status).arg(options))
    }

    /// This command enables the tracking feature of the Redis server,
    /// that is used for [`server assisted client side caching`](https://redis.io/topics/client-side-caching).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-tracking/>](https://redis.io/commands/client-tracking/)
    #[must_use]
    fn client_trackinginfo(&mut self) -> PreparedCommand<Self, ClientTrackingInfo>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("TRACKINGINFO"))
    }

    /// This command can unblock, from a different connection,
    /// a client blocked in a blocking operation,
    /// such as for instance `BRPOP` or `XREAD` or `WAIT`.
    ///
    /// # Return
    /// * `true` - This command can unblock, from a different connection, a client blocked in a blocking operation, such as for instance BRPOP or XREAD or WAIT.
    /// * `false` - if the client wasn't unblocked.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-unblock/>](https://redis.io/commands/client-unblock/)
    #[must_use]
    fn client_unblock(
        &mut self,
        client_id: i64,
        mode: ClientUnblockMode,
    ) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("UNBLOCK").arg(client_id).arg(mode))
    }

    /// Used to resume command processing for all clients that were
    /// paused by [`client_pause`](ConnectionCommands::client_pause).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-unpause/>](https://redis.io/commands/client-unpause/)
    #[must_use]
    fn client_unpause(&mut self) -> PreparedCommand<Self, bool>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CLIENT").arg("UNPAUSE"))
    }

    /// Returns `message`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/echo/>](https://redis.io/commands/echo/)
    #[must_use]
    fn echo<M, R>(&mut self, message: M) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        M: Into<CommandArg>,
        R: FromValue,
    {
        prepare_command(self, cmd("ECHO").arg(message))
    }

    /// Switch to a different protocol,
    /// optionally authenticating and setting the connection's name,
    /// or provide a contextual client report.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hello/>](https://redis.io/commands/hello/)
    #[must_use]
    fn hello(&mut self, options: HelloOptions) -> PreparedCommand<Self, HelloResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("HELLO").arg(options))
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ping/>](https://redis.io/commands/ping/)
    #[must_use]
    fn ping<R>(&mut self, options: PingOptions) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        R: FromValue,
    {
        prepare_command(self, cmd("PING").arg(options))
    }

    /// Ask the server to close the connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/quit/>](https://redis.io/commands/quit/)
    #[must_use]
    fn quit(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("QUIT"))
    }

    /// This command performs a full reset of the connection's server-side context,
    /// mimicking the effect of disconnecting and reconnecting again.
    ///
    /// # See Also
    /// [<https://redis.io/commands/reset/>](https://redis.io/commands/reset/)
    #[must_use]
    fn reset(&mut self) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("RESET"))
    }

    /// Select the Redis logical database having the specified zero-based numeric index.
    ///
    /// # See Also
    /// [<https://redis.io/commands/reset/>](https://redis.io/commands/reset/)
    #[must_use]
    fn select(&mut self, index: usize) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SELECT").arg(index))
    }
}

/// Client caching mode for the [`client_caching`](ConnectionCommands::client_caching) command.
pub enum ClientCachingMode {
    Yes,
    No,
}

impl IntoArgs for ClientCachingMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            ClientCachingMode::Yes => CommandArg::Str("YES"),
            ClientCachingMode::No => CommandArg::Str("NO"),
        })
    }
}

/// Client info results for the [`client_info`](ConnectionCommands::client_info)
/// & [`client_list`](ConnectionCommands::client_list) commands.
#[derive(Debug)]
pub struct ClientInfo {
    /// a unique 64-bit client ID
    pub id: i64,

    /// address/port of the client
    pub addr: String,

    /// address/port of local address client connected to (bind address)
    pub laddr: String,

    /// file descriptor corresponding to the socket
    pub fd: u32,

    /// the name set by the client with [`client_setname`](ConnectionCommands::client_setname)
    pub name: String,

    /// total duration of the connection in seconds
    pub age: u32,

    /// idle time of the connection in seconds
    pub idle: u32,

    /// client flags (see [`client-list`](https://redis.io/commands/client-list/))
    pub flags: String,

    /// current database ID
    pub db: usize,

    /// number of channel subscriptions
    pub sub: usize,

    /// number of pattern matching subscriptions
    pub psub: usize,

    /// number of shard channel subscriptions. Added in Redis 7.0.3
    pub ssub: usize,

    /// number of commands in a MULTI/EXEC context
    pub multi: usize,

    /// query buffer length (0 means no query pending)
    pub qbuf: usize,

    /// free space of the query buffer (0 means the buffer is full)
    pub qbuf_free: usize,

    /// incomplete arguments for the next command (already extracted from query buffer)
    pub argv_mem: usize,

    /// memory is used up by buffered multi commands. Added in Redis 7.0
    pub multi_mem: usize,

    /// output buffer length
    pub obl: usize,

    /// output list length (replies are queued in this list when the buffer is full)
    pub oll: usize,

    /// output buffer memory usage
    pub omem: usize,

    ///  total memory consumed by this client in its various buffers
    pub tot_mem: usize,

    /// file descriptor events (r or w)
    pub events: String,

    /// last command played
    pub cmd: String,

    /// the authenticated username of the client
    pub user: String,

    /// client id of current client tracking redirection
    pub redir: i64,

    /// client RESP protocol version
    pub resp: i32,

    /// additional arguments that may be added in future versions of Redis
    pub additional_arguments: HashMap<String, String>,
}

impl ClientInfo {
    pub fn from_line(line: &str) -> Result<ClientInfo> {
        // Each line is composed of a succession of property=value fields separated by a space character.
        let mut values: HashMap<String, String> = line
            .trim_end()
            .split(' ')
            .map(|kvp| {
                let mut iter = kvp.split('=');
                match (iter.next(), iter.next()) {
                    (Some(key), None) => (key.to_owned(), "".to_owned()),
                    (Some(key), Some(value)) => (key.to_owned(), value.to_owned()),
                    _ => ("".to_owned(), "".to_owned()),
                }
            })
            .collect();

        Ok(ClientInfo {
            id: values
                .remove("id")
                .map(|id| id.parse::<i64>().unwrap_or_default())
                .unwrap_or_default(),
            addr: values.remove("addr").unwrap_or_default(),
            laddr: values.remove("laddr").unwrap_or_default(),
            fd: values
                .remove("fd")
                .map(|id| id.parse::<u32>().unwrap_or_default())
                .unwrap_or_default(),
            name: values.remove("name").unwrap_or_default(),
            age: values
                .remove("age")
                .map(|id| id.parse::<u32>().unwrap_or_default())
                .unwrap_or_default(),
            idle: values
                .remove("idle")
                .map(|id| id.parse::<u32>().unwrap_or_default())
                .unwrap_or_default(),
            flags: values.remove("flags").unwrap_or_default(),
            db: values
                .remove("db")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            sub: values
                .remove("sub")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            psub: values
                .remove("psub")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            ssub: values
                .remove("ssub")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            multi: values
                .remove("multi")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            qbuf: values
                .remove("qbuf")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            qbuf_free: values
                .remove("qbuf-free")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            argv_mem: values
                .remove("argv-mem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            multi_mem: values
                .remove("multi-mem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            obl: values
                .remove("obl")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            oll: values
                .remove("oll")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            omem: values
                .remove("omem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            tot_mem: values
                .remove("tot-mem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            events: values.remove("events").unwrap_or_default(),
            cmd: values.remove("cmd").unwrap_or_default(),
            user: values.remove("user").unwrap_or_default(),
            redir: values
                .remove("redir")
                .map(|id| id.parse::<i64>().unwrap_or_default())
                .unwrap_or_default(),
            resp: values
                .remove("resp")
                .map(|id| id.parse::<i32>().unwrap_or_default())
                .unwrap_or_default(),
            additional_arguments: values,
        })
    }
}

impl FromValue for ClientInfo {
    fn from_value(value: Value) -> Result<Self> {
        ClientInfo::from_line(&value.into::<String>()?)
    }
}

/// Client type options for the [`client_list`](ConnectionCommands::client_list) command.
pub enum ClientType {
    Normal,
    Master,
    Replica,
    PubSub,
}

impl IntoArgs for ClientType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            ClientType::Normal => CommandArg::Str("NORMAL"),
            ClientType::Master => CommandArg::Str("MASTER"),
            ClientType::Replica => CommandArg::Str("REPLICA"),
            ClientType::PubSub => CommandArg::Str("PUBSUB"),
        })
    }
}

/// Options for the [client_list](ConnectionCommands::client_list) command.
#[derive(Default)]
pub struct ClientListOptions {
    command_args: CommandArgs,
}

impl IntoArgs for ClientListOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

impl ClientListOptions {
    #[must_use]
    pub fn client_type(self, client_type: ClientType) -> Self {
        Self {
            command_args: self.command_args.arg("TYPE").arg(client_type),
        }
    }

    pub fn client_ids<II>(self, client_ids: II) -> Self
    where
        II: SingleArgOrCollection<i64>,
    {
        Self {
            command_args: self.command_args.arg("ID").arg(client_ids),
        }
    }
}

/// Result for the [`client_list`](ConnectionCommands::client_list) command.
#[derive(Debug)]
pub struct ClientListResult {
    pub client_infos: Vec<ClientInfo>,
}

impl FromValue for ClientListResult {
    fn from_value(value: Value) -> Result<Self> {
        let lines: String = value.into()?;

        let client_infos: Result<Vec<ClientInfo>> =
            lines.split('\n').map(ClientInfo::from_line).collect();

        Ok(Self {
            client_infos: client_infos?,
        })
    }
}

/// Options for the [`client-kill`](ConnectionCommands::client-kill) command.
#[derive(Default)]
pub struct ClientKillOptions {
    command_args: CommandArgs,
}

impl ClientKillOptions {
    #[must_use]
    pub fn id(self, client_id: i64) -> Self {
        Self {
            command_args: self.command_args.arg("ID").arg(client_id),
        }
    }

    #[must_use]
    pub fn client_type(self, client_type: ClientType) -> Self {
        Self {
            command_args: self.command_args.arg("TYPE").arg(client_type),
        }
    }

    #[must_use]
    pub fn user<U: Into<CommandArg>>(self, username: U) -> Self {
        Self {
            command_args: self.command_args.arg("USER").arg(username),
        }
    }

    /// Address in the format of `ip:port`
    ///
    /// The ip:port should match a line returned by the
    /// [`client_list`](ConnectionCommands::client_list) command (addr field).
    #[must_use]
    pub fn addr<A: Into<CommandArg>>(self, addr: A) -> Self {
        Self {
            command_args: self.command_args.arg("ADDR").arg(addr),
        }
    }

    /// Kill all clients connected to specified local (bind) address.
    #[must_use]
    pub fn laddr<A: Into<CommandArg>>(self, laddr: A) -> Self {
        Self {
            command_args: self.command_args.arg("LADDR").arg(laddr),
        }
    }

    /// By default this option is set to yes, that is, the client calling the command will not get killed,
    /// however setting this option to no will have the effect of also killing the client calling the command.
    #[must_use]
    pub fn skip_me(self, skip_me: bool) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("SKIPME")
                .arg(if skip_me { "YES" } else { "NO" }),
        }
    }
}

impl IntoArgs for ClientKillOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Mode options for the [`client_pause`](ConnectionCommands::client_pause) command.
pub enum ClientPauseMode {
    /// Clients are only blocked if they attempt to execute a write command.
    Write,
    /// This is the default mode. All client commands are blocked.
    All,
}

impl IntoArgs for ClientPauseMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            ClientPauseMode::Write => CommandArg::Str("WRITE"),
            ClientPauseMode::All => CommandArg::Str("ALL"),
        })
    }
}

impl Default for ClientPauseMode {
    fn default() -> Self {
        ClientPauseMode::All
    }
}

/// Mode options for the [`client_reply`](ConnectionCommands::client_reply) command.
pub enum ClientReplyMode {
    On,
    Off,
    Skip,
}

impl IntoArgs for ClientReplyMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            ClientReplyMode::On => CommandArg::Str("ON"),
            ClientReplyMode::Off => CommandArg::Str("OFF"),
            ClientReplyMode::Skip => CommandArg::Str("SKIP"),
        })
    }
}

/// Status options for the [`client_tracking`](ConnectionCommands::client_tracking) command.
pub enum ClientTrackingStatus {
    On,
    Off,
}

impl IntoArgs for ClientTrackingStatus {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            ClientTrackingStatus::On => CommandArg::Str("ON"),
            ClientTrackingStatus::Off => CommandArg::Str("OFF"),
        })
    }
}

/// Options for the [`client_tracking`](ConnectionCommands::client_tracking) command.
#[derive(Default)]
pub struct ClientTrackingOptions {
    command_args: CommandArgs,
}

impl ClientTrackingOptions {
    #[must_use]
    /// send invalidation messages to the connection with the specified ID.
    pub fn redirect(self, client_id: i64) -> Self {
        Self {
            command_args: self.command_args.arg("REDIRECT").arg(client_id),
        }
    }

    /// enable tracking in broadcasting mode.
    pub fn broadcasting(self) -> Self {
        Self {
            command_args: self.command_args.arg("BCAST"),
        }
    }

    /// for broadcasting, register a given key prefix, so that notifications
    /// will be provided only for keys starting with this string.
    ///
    /// This option can be given multiple times to register multiple prefixes.
    pub fn prefix<P: Into<CommandArg>>(self, prefix: P) -> Self {
        Self {
            command_args: self.command_args.arg("PREFIX").arg(prefix),
        }
    }

    /// when broadcasting is NOT active, normally don't track keys in read only commands,
    /// unless they are called immediately after a `CLIENT CACHING yes` command.
    pub fn optin(self) -> Self {
        Self {
            command_args: self.command_args.arg("OPTIN"),
        }
    }

    /// when broadcasting is NOT active, normally track keys in read only commands,
    /// unless they are called immediately after a `CLIENT CACHING no` command.
    pub fn optout(self) -> Self {
        Self {
            command_args: self.command_args.arg("OPTOUT"),
        }
    }

    /// don't send notifications about keys modified by this connection itself.
    pub fn no_loop(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOLOOP"),
        }
    }
}

impl IntoArgs for ClientTrackingOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`client_trackinginfo`](ConnectionCommands::client_trackinginfo) command.
pub struct ClientTrackingInfo {
    /// A list of tracking flags used by the connection.
    pub flags: Vec<String>,

    /// The client ID used for notifications redirection, or -1 when none.
    pub redirect: i64,

    /// A list of key prefixes for which notifications are sent to the client.
    pub prefixes: Vec<String>,
}

impl FromValue for ClientTrackingInfo {
    fn from_value(value: Value) -> Result<Self> {
        fn into_result(values: &mut HashMap<String, Value>) -> Option<ClientTrackingInfo> {
            Some(ClientTrackingInfo {
                flags: values.remove("flags")?.into().ok()?,
                redirect: values.remove("redirect")?.into().ok()?,
                prefixes: values.remove("prefixes")?.into().ok()?,
            })
        }

        into_result(&mut value.into()?).ok_or_else(|| {
            Error::Client(
                "Cannot parse 
            "
                .to_owned(),
            )
        })
    }
}

/// Mode options for the [`client_unblock`](ConnectionCommands::client_unblock) command.
pub enum ClientUnblockMode {
    /// By default the client is unblocked as if the timeout of the command was reached,
    Timeout,
    /// the behavior is to unblock the client returning as error the fact that the client was force-unblocked.
    Error,
}

impl IntoArgs for ClientUnblockMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            ClientUnblockMode::Timeout => CommandArg::Str("TIMEOUT"),
            ClientUnblockMode::Error => CommandArg::Str("ERROR"),
        })
    }
}

impl Default for ClientUnblockMode {
    fn default() -> Self {
        ClientUnblockMode::Timeout
    }
}

/// Options for the [`hello`](ConnectionCommands::hello) command.
#[derive(Default)]
pub struct HelloOptions {
    command_args: CommandArgs,
}

impl HelloOptions {
    #[must_use]
    pub fn new(protover: usize) -> Self {
        Self {
            command_args: CommandArgs::default().arg(protover),
        }
    }

    #[must_use]
    pub fn auth<U, P>(self, username: U, password: P) -> Self
    where
        U: Into<CommandArg>,
        P: Into<CommandArg>,
    {
        Self {
            command_args: self.command_args.arg("AUTH").arg(username).arg(password),
        }
    }

    #[must_use]
    pub fn set_name<C>(self, client_name: C) -> Self
    where
        C: Into<CommandArg>,
    {
        Self {
            command_args: self.command_args.arg("SETNAME").arg(client_name),
        }
    }
}

impl IntoArgs for HelloOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`hello`](ConnectionCommands::hello) command
pub struct HelloResult {
    pub server: String,
    pub version: String,
    pub proto: usize,
    pub id: i64,
    pub mode: String,
    pub role: String,
    pub modules: Vec<String>,
}

impl FromValue for HelloResult {
    fn from_value(value: Value) -> Result<Self> {
        match &value {
            Value::Array(v) if v.len() == 14 => {
                fn into_result(values: &mut HashMap<String, Value>) -> Option<HelloResult> {
                    Some(HelloResult {
                        server: values.remove("server")?.into().ok()?,
                        version: values.remove("version")?.into().ok()?,
                        proto: values.remove("proto")?.into().ok()?,
                        id: values.remove("id")?.into().ok()?,
                        mode: values.remove("mode")?.into().ok()?,
                        role: values.remove("role")?.into().ok()?,
                        modules: values.remove("modules")?.into().ok()?,
                    })
                }

                into_result(&mut value.into()?)
                    .ok_or_else(|| Error::Client("Cannot parse HelloResult".to_owned()))
            }
            _ => Err(Error::Client("Cannot parse HelloResult".to_owned())),
        }
    }
}

/// Options for the [`ping`](ConnectionCommands::ping) command.
#[derive(Default)]
pub struct PingOptions {
    command_args: CommandArgs,
}

impl PingOptions {
    #[must_use]
    pub fn message<M: Into<CommandArg>>(self, message: M) -> Self {
        Self {
            command_args: self.command_args.arg(message),
        }
    }
}

impl IntoArgs for PingOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
