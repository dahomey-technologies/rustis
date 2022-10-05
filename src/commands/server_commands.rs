use crate::{
    resp::{
        cmd, BulkString, CommandArgs, FromKeyValueValueArray, FromSingleValueArray, FromValue,
        IntoArgs, KeyValueArgOrCollection, SingleArgOrCollection, Value,
    },
    CommandResult, PrepareCommand,
};

/// A group of Redis commands related to Server Management
/// # See Also
/// [Redis Server Management Commands](https://redis.io/commands/?group=server)
/// [ACL guide](https://redis.io/docs/manual/security/acl/)
pub trait ServerCommands<T>: PrepareCommand<T> {
    /// The command shows the available ACL categories if called without arguments.
    /// If a category name is given, the command shows all the Redis commands in the specified category.
    ///
    /// # Return
    /// A collection of ACL categories or a collection of commands inside a given category.
    ///
    /// # Errors
    /// The command may return an error if an invalid category name is given as argument.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-cat/>](https://redis.io/commands/acl-cat/)
    fn acl_cat<C, CC>(&self, options: AclCatOptions) -> CommandResult<T, CC>
    where
        C: FromValue,
        CC: FromSingleValueArray<C>,
    {
        self.prepare_command(cmd("ACL").arg("CAT").arg(options))
    }

    /// Delete all the specified ACL users and terminate all 
    /// the connections that are authenticated with such users.
    ///
    /// # Return
    /// The number of users that were deleted. 
    /// This number will not always match the number of arguments since certain users may not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-deluser/>](https://redis.io/commands/acl-deluser/)
    fn acl_deluser<U, UU>(&self, usernames: UU) -> CommandResult<T, usize>
    where
        U: Into<BulkString>,
        UU: SingleArgOrCollection<U>,
    {
        self.prepare_command(cmd("ACL").arg("DELUSER").arg(usernames))
    }

    /// Simulate the execution of a given command by a given user. 
    ///
    /// # Return
    /// OK on success.
    /// An error describing why the user can't execute the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-dryrun/>](https://redis.io/commands/acl-dryrun/)
    fn acl_dryrun<U, C, R>(&self, username: U, command: C, options: AclDryRunOptions) -> CommandResult<T, R>
    where
        U: Into<BulkString>,
        C: Into<BulkString>,
        R: FromValue
    {
        self.prepare_command(cmd("ACL").arg("DRYRUN").arg(username).arg(command).arg(options))
    }

    /// Generates a password starting from /dev/urandom if available, 
    /// otherwise (in systems without /dev/urandom) it uses a weaker 
    /// system that is likely still better than picking a weak password by hand.
    ///
    /// # Return
    /// by default 64 bytes string representing 256 bits of pseudorandom data. 
    /// Otherwise if an argument if needed, the output string length is the number 
    /// of specified bits (rounded to the next multiple of 4) divided by 4.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-genpass/>](https://redis.io/commands/acl-genpass/)
    fn acl_genpass<R: FromValue>(&self, options: AclGenPassOptions) -> CommandResult<T, R>
    {
        self.prepare_command(cmd("ACL").arg("GENPASS").arg(options))
    }

    /// The command returns all the rules defined for an existing ACL user.
    ///
    /// # Return
    /// A collection of ACL rule definitions for the user.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-getuser/>](https://redis.io/commands/acl-getuser/)
    fn acl_getuser<U, RR>(&self, username: U) -> CommandResult<T, RR>
    where
        U: Into<BulkString>,
        RR: FromKeyValueValueArray<String, Value>

    {
        self.prepare_command(cmd("ACL").arg("GETUSER").arg(username))
    }

    /// The command shows the currently active ACL rules in the Redis server.
    ///
    /// # Return
    /// An array of strings.
    /// Each line in the returned array defines a different user, and the 
    /// format is the same used in the redis.conf file or the external ACL file
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-list/>](https://redis.io/commands/acl-list/)
    fn acl_list(&self) -> CommandResult<T, Vec<String>>
    {
        self.prepare_command(cmd("ACL").arg("LIST"))
    }

    /// When Redis is configured to use an ACL file (with the aclfile configuration option), 
    /// this command will reload the ACLs from the file, replacing all the current ACL rules 
    /// with the ones defined in the file. 
    ///
    /// # Return
    /// An array of strings.
    /// Each line in the returned array defines a different user, and the 
    /// format is the same used in the redis.conf file or the external ACL file
    /// 
    /// # Errors
    /// The command may fail with an error for several reasons: 
    /// - if the file is not readable, 
    /// - if there is an error inside the file, and in such case the error will be reported to the user in the error.
    /// - Finally the command will fail if the server is not configured to use an external ACL file.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-load/>](https://redis.io/commands/acl-load/)
    fn acl_load(&self) -> CommandResult<T, ()>
    {
        self.prepare_command(cmd("ACL").arg("LOAD"))
    }

    /// The command shows a list of recent ACL security events
    ///
    /// # Return
    /// A key/value collection of ACL security events.
    /// Empty collection when called with the [`reset`](crate::AclLogOptions::reset) option
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-log/>](https://redis.io/commands/acl-log/)
    fn acl_log<EE>(&self, options: AclLogOptions) -> CommandResult<T, Vec<EE>>
    where 
        EE: FromKeyValueValueArray<String, Value>
    {
        self.prepare_command(cmd("ACL").arg("LOG").arg(options))
    }

    /// When Redis is configured to use an ACL file (with the aclfile configuration option), 
    /// this command will save the currently defined ACLs from the server memory to the ACL file.
    /// 
    /// # Errors
    /// The command may fail with an error for several reasons: 
    /// - if the file cannot be written 
    /// - if the server is not configured to use an external ACL file.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-save/>](https://redis.io/commands/acl-save/)
    fn acl_save(&self) -> CommandResult<T, ()>
    {
        self.prepare_command(cmd("ACL").arg("SAVE"))
    }

    /// Create an ACL user with the specified rules or modify the rules of an existing user. 
    /// 
    /// # Errors
    /// If the rules contain errors, the error is returned.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-setuser/>](https://redis.io/commands/acl-setuser/)
    fn acl_setuser<U, R, RR>(&self, username: U, rules: RR) -> CommandResult<T, ()>
    where
        U: Into<BulkString>,
        R: Into<BulkString>,
        RR: SingleArgOrCollection<R>
    {
        self.prepare_command(cmd("ACL").arg("SETUSER").arg(username).arg(rules))
    }

    /// The command shows a list of all the usernames of the currently configured users in the Redis ACL system.
    /// 
    /// # Return
    /// A collection of usernames
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-users/>](https://redis.io/commands/acl-users/)
    fn acl_users<U, UU>(&self) -> CommandResult<T, UU>
    where
        U: FromValue,
        UU: FromSingleValueArray<U>,
    {
        self.prepare_command(cmd("ACL").arg("USERS"))
    }

    /// Return the username the current connection is authenticated with. 
    /// 
    /// # Return
    /// The username of the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-whoami/>](https://redis.io/commands/acl-whoami/)
    fn acl_whoami<U: FromValue>(&self) -> CommandResult<T, U>
    {
        self.prepare_command(cmd("ACL").arg("WHOAMI"))
    }
    
    /// Used to read the configuration parameters of a running Redis server.
    ///
    /// For every key that does not hold a string value or does not exist,
    /// the special value nil is returned. Because of this, the operation never fails.
    ///
    /// # Return
    /// Array reply: collection of the requested params with their matching values.
    ///
    /// # See Also
    /// [<https://redis.io/commands/mget/>](https://redis.io/commands/mget/)
    #[must_use]
    fn config_get<P, PP, V, VV>(&self, params: PP) -> CommandResult<T, VV>
    where
        P: Into<BulkString>,
        PP: SingleArgOrCollection<P>,
        V: FromValue,
        VV: FromKeyValueValueArray<String, V>,
    {
        self.prepare_command(cmd("CONFIG").arg("GET").arg(params))
    }

    /// Used in order to reconfigure the server at run time without the need to restart Redis.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-set/>](https://redis.io/commands/config-set/)
    #[must_use]
    fn config_set<P, V, C>(&self, configs: C) -> CommandResult<T, ()>
    where
        P: Into<BulkString>,
        V: Into<BulkString>,
        C: KeyValueArgOrCollection<P, V>,
    {
        self.prepare_command(cmd("CONFIG").arg("SET").arg(configs))
    }

    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [<https://redis.io/commands/flushdb/>](https://redis.io/commands/flushdb/)
    #[must_use]
    fn flushdb(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FLUSHDB").arg(flushing_mode))
    }

    /// Delete all the keys of all the existing databases, not just the currently selected one.
    ///
    /// # See Also
    /// [<https://redis.io/commands/flushall/>](https://redis.io/commands/flushall/)
    #[must_use]
    fn flushall(&self, flushing_mode: FlushingMode) -> CommandResult<T, ()> {
        self.prepare_command(cmd("FLUSHALL").arg(flushing_mode))
    }

    /// The TIME command returns the current server time as a two items lists:
    /// a Unix timestamp and the amount of microseconds already elapsed in the current second.
    ///
    /// # See Also
    /// [<https://redis.io/commands/time/>](https://redis.io/commands/time/)
    #[must_use]
    fn time(&self) -> CommandResult<T, (u32, u32)> {
        self.prepare_command(cmd("TIME"))
    }
}

/// Database flushing mode
pub enum FlushingMode {
    Default,
    /// Flushes the database(s) asynchronously
    Async,
    /// Flushed the database(s) synchronously
    Sync,
}

impl Default for FlushingMode {
    fn default() -> Self {
        FlushingMode::Default
    }
}

impl IntoArgs for FlushingMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            FlushingMode::Default => args,
            FlushingMode::Async => args.arg("ASYNC"),
            FlushingMode::Sync => args.arg("SYNC"),
        }
    }
}

/// Options for the [`acl_cat`](crate::ServerCommands::acl_cat) command
#[derive(Default)]
pub struct AclCatOptions {
    command_args: CommandArgs,
}

impl AclCatOptions {
    #[must_use]
    pub fn category_name<C: Into<BulkString>>(self, category_name: C) -> Self {
        Self {
            command_args: self.command_args.arg(category_name),
        }
    }
}

impl IntoArgs for AclCatOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`acl_dryrun`](crate::ServerCommands::acl_dryrun) command
#[derive(Default)]
pub struct AclDryRunOptions {
    command_args: CommandArgs,
}

impl AclDryRunOptions {
    #[must_use]
    pub fn arg<A, AA>(self, args: AA) -> Self 
    where
        A: Into<BulkString>,
        AA: SingleArgOrCollection<A>
    {
        Self {
            command_args: self.command_args.arg(args),
        }
    }
}

impl IntoArgs for AclDryRunOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`acl_genpass`](crate::ServerCommands::acl_genpass) command
#[derive(Default)]
pub struct AclGenPassOptions {
    command_args: CommandArgs,
}

impl AclGenPassOptions {
    /// The command output is a hexadecimal representation of a binary string. 
    /// By default it emits 256 bits (so 64 hex characters). 
    /// The user can provide an argument in form of number of bits to emit from 1 to 1024 to change the output length. 
    /// Note that the number of bits provided is always rounded to the next multiple of 4. 
    /// So for instance asking for just 1 bit password will result in 4 bits to be emitted, in the form of a single hex character.
    #[must_use]
    pub fn bits(self, bits: usize) -> Self 
    {
        Self {
            command_args: self.command_args.arg(bits),
        }
    }
}

impl IntoArgs for AclGenPassOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`acl_log`](crate::ServerCommands::acl_log) command
#[derive(Default)]
pub struct AclLogOptions {
    command_args: CommandArgs,
}

impl AclLogOptions {
    /// This optional argument specifies how many entries to show. 
    /// By default up to ten failures are returned.
    #[must_use]
    pub fn count(self, count: usize) -> Self 
    {
        Self {
            command_args: self.command_args.arg(count),
        }
    }

    /// The special RESET argument clears the log. 
    #[must_use]
    pub fn reset(self) -> Self 
    {
        Self {
            command_args: self.command_args.arg("RESET"),
        }
    }
}

impl IntoArgs for AclLogOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}
