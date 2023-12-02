use crate::{
    client::{prepare_command, PreparedCommand},
    resp::{
        cmd, CollectionResponse, CommandArgs, KeyValueArgsCollection, KeyValueCollectionResponse,
        PrimitiveResponse, SingleArg, SingleArgCollection, ToArgs, Value,
    },
    Error, Result,
};
use serde::{
    de::{self, DeserializeOwned, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{collections::HashMap, fmt, str::FromStr};

/// A group of Redis commands related to Server Management
/// # See Also
/// [Redis Server Management Commands](https://redis.io/commands/?group=server)
/// [ACL guide](https://redis.io/docs/manual/security/acl/)
pub trait ServerCommands<'a> {
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
    fn acl_cat<C, CC>(self, options: AclCatOptions) -> PreparedCommand<'a, Self, CC>
    where
        Self: Sized,
        C: PrimitiveResponse + DeserializeOwned,
        CC: CollectionResponse<C>,
    {
        prepare_command(self, cmd("ACL").arg("CAT").arg(options))
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
    fn acl_deluser<U, UU>(self, usernames: UU) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        U: SingleArg,
        UU: SingleArgCollection<U>,
    {
        prepare_command(self, cmd("ACL").arg("DELUSER").arg(usernames))
    }

    /// Simulate the execution of a given command by a given user.
    ///
    /// # Return
    /// OK on success.
    /// An error describing why the user can't execute the command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-dryrun/>](https://redis.io/commands/acl-dryrun/)
    fn acl_dryrun<U, C, R>(
        self,
        username: U,
        command: C,
        options: AclDryRunOptions,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        U: SingleArg,
        C: SingleArg,
        R: PrimitiveResponse,
    {
        prepare_command(
            self,
            cmd("ACL")
                .arg("DRYRUN")
                .arg(username)
                .arg(command)
                .arg(options),
        )
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
    fn acl_genpass<R: PrimitiveResponse>(
        self,
        options: AclGenPassOptions,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("ACL").arg("GENPASS").arg(options))
    }

    /// The command returns all the rules defined for an existing ACL user.
    ///
    /// # Return
    /// A collection of ACL rule definitions for the user.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-getuser/>](https://redis.io/commands/acl-getuser/)
    fn acl_getuser<U, RR>(self, username: U) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        U: SingleArg,
        RR: KeyValueCollectionResponse<String, Value>,
    {
        prepare_command(self, cmd("ACL").arg("GETUSER").arg(username))
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
    fn acl_list(self) -> PreparedCommand<'a, Self, Vec<String>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("ACL").arg("LIST"))
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
    fn acl_load(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("ACL").arg("LOAD"))
    }

    /// The command shows a list of recent ACL security events
    ///
    /// # Return
    /// A key/value collection of ACL security events.
    /// Empty collection when called with the [`reset`](AclLogOptions::reset) option
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-log/>](https://redis.io/commands/acl-log/)
    fn acl_log<EE>(self, options: AclLogOptions) -> PreparedCommand<'a, Self, Vec<EE>>
    where
        Self: Sized,
        EE: KeyValueCollectionResponse<String, Value> + DeserializeOwned,
    {
        prepare_command(self, cmd("ACL").arg("LOG").arg(options))
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
    fn acl_save(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("ACL").arg("SAVE"))
    }

    /// Create an ACL user with the specified rules or modify the rules of an existing user.
    ///
    /// # Errors
    /// If the rules contain errors, the error is returned.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-setuser/>](https://redis.io/commands/acl-setuser/)
    fn acl_setuser<U, R, RR>(self, username: U, rules: RR) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        U: SingleArg,
        R: SingleArg,
        RR: SingleArgCollection<R>,
    {
        prepare_command(self, cmd("ACL").arg("SETUSER").arg(username).arg(rules))
    }

    /// The command shows a list of all the usernames of the currently configured users in the Redis ACL system.
    ///
    /// # Return
    /// A collection of usernames
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-users/>](https://redis.io/commands/acl-users/)
    fn acl_users<U, UU>(self) -> PreparedCommand<'a, Self, UU>
    where
        Self: Sized,
        U: PrimitiveResponse + DeserializeOwned,
        UU: CollectionResponse<U>,
    {
        prepare_command(self, cmd("ACL").arg("USERS"))
    }

    /// Return the username the current connection is authenticated with.
    ///
    /// # Return
    /// The username of the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/acl-whoami/>](https://redis.io/commands/acl-whoami/)
    fn acl_whoami<U: PrimitiveResponse>(self) -> PreparedCommand<'a, Self, U>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("ACL").arg("WHOAMI"))
    }

    /// Return an array with details about every Redis command.
    ///
    /// # Return
    /// A nested list of command details.
    /// The order of commands in the array is random.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command/>](https://redis.io/commands/command/)
    fn command(self) -> PreparedCommand<'a, Self, Vec<CommandInfo>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("COMMAND"))
    }

    /// Number of total commands in this Redis server.
    ///
    /// # Return
    /// number of commands returned by [`command`](ServerCommands::command)
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-count/>](https://redis.io/commands/command-count/)
    fn command_count(self) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("COMMAND").arg("COUNT"))
    }

    /// Number of total commands in this Redis server.
    ///
    /// # Return
    /// map key=command name, value=command doc
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-docs/>](https://redis.io/commands/command-docs/)
    fn command_docs<N, NN, DD>(self, command_names: NN) -> PreparedCommand<'a, Self, DD>
    where
        Self: Sized,
        N: SingleArg,
        NN: SingleArgCollection<N>,
        DD: KeyValueCollectionResponse<String, CommandDoc>,
    {
        prepare_command(self, cmd("COMMAND").arg("DOCS").arg(command_names))
    }

    /// A helper command to let you find the keys from a full Redis command.
    ///
    /// # Return
    /// list of keys from your command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-_getkeys/>](https://redis.io/commands/command-_getkeys/)
    fn command_getkeys<A, AA, KK>(self, args: AA) -> PreparedCommand<'a, Self, KK>
    where
        Self: Sized,
        A: SingleArg,
        AA: SingleArgCollection<A>,
        KK: CollectionResponse<String>,
    {
        prepare_command(self, cmd("COMMAND").arg("GETKEYS").arg(args))
    }

    /// A helper command to let you find the keys from a full Redis command together with flags indicating what each key is used for.
    ///
    /// # Return
    /// map of keys with their flags from your command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-getkeysandflags/>](https://redis.io/commands/command-getkeysandflags/)
    fn command_getkeysandflags<A, AA, KK>(self, args: AA) -> PreparedCommand<'a, Self, KK>
    where
        Self: Sized,
        A: SingleArg,
        AA: SingleArgCollection<A>,
        KK: KeyValueCollectionResponse<String, Vec<String>>,
    {
        prepare_command(self, cmd("COMMAND").arg("GETKEYSANDFLAGS").arg(args))
    }

    /// Return an array with details about multiple Redis command.
    ///
    /// # Return
    /// A nested list of command details.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-info/>](https://redis.io/commands/command-info/)
    fn command_info<N, NN>(self, command_names: NN) -> PreparedCommand<'a, Self, Vec<CommandInfo>>
    where
        Self: Sized,
        N: SingleArg,
        NN: SingleArgCollection<N>,
    {
        prepare_command(self, cmd("COMMAND").arg("INFO").arg(command_names))
    }

    /// Return an array of the server's command names based on optional filters
    ///
    /// # Return
    /// an array of the server's command names.
    ///
    /// # See Also
    /// [<https://redis.io/commands/command-list/>](https://redis.io/commands/command-list/)
    fn command_list<CC>(self, options: CommandListOptions) -> PreparedCommand<'a, Self, CC>
    where
        Self: Sized,
        CC: CollectionResponse<String>,
    {
        prepare_command(self, cmd("COMMAND").arg("LIST").arg(options))
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
    /// [<https://redis.io/commands/config-get/>](https://redis.io/commands/config-get/)
    #[must_use]
    fn config_get<P, PP, V, VV>(self, params: PP) -> PreparedCommand<'a, Self, VV>
    where
        Self: Sized,
        P: SingleArg,
        PP: SingleArgCollection<P>,
        V: PrimitiveResponse,
        VV: KeyValueCollectionResponse<String, V>,
    {
        prepare_command(self, cmd("CONFIG").arg("GET").arg(params))
    }

    /// Resets the statistics reported by Redis using the [`info`](ServerCommands::info) command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-resetstat/>](https://redis.io/commands/config-resetstat/)
    #[must_use]
    fn config_resetstat(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CONFIG").arg("RESETSTAT"))
    }

    /// Rewrites the redis.conf file the server was started with,
    /// applying the minimal changes needed to make it reflect the configuration currently used by the server,
    /// which may be different compared to the original one because of the use of the
    /// [`config_set`](ServerCommands::config_set) command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-rewrite/>](https://redis.io/commands/config-rewrite/)
    #[must_use]
    fn config_rewrite(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("CONFIG").arg("REWRITE"))
    }

    /// Used in order to reconfigure the server at run time without the need to restart Redis.
    ///
    /// # See Also
    /// [<https://redis.io/commands/config-set/>](https://redis.io/commands/config-set/)
    #[must_use]
    fn config_set<P, V, C>(self, configs: C) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        P: SingleArg,
        V: SingleArg,
        C: KeyValueArgsCollection<P, V>,
    {
        prepare_command(self, cmd("CONFIG").arg("SET").arg(configs))
    }

    /// Return the number of keys in the currently-selected database.
    ///
    /// # See Also
    /// [<https://redis.io/commands/dbsize/>](https://redis.io/commands/dbsize/)
    #[must_use]
    fn dbsize(self) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("DBSIZE"))
    }

    /// This command will start a coordinated failover between
    /// the currently-connected-to master and one of its replicas.
    ///
    /// # See Also
    /// [<https://redis.io/commands/failover/>](https://redis.io/commands/failover/)
    #[must_use]
    fn failover(self, options: FailOverOptions) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FAILOVER").arg(options))
    }

    /// Delete all the keys of the currently selected DB.
    ///
    /// # See Also
    /// [<https://redis.io/commands/flushdb/>](https://redis.io/commands/flushdb/)
    #[must_use]
    fn flushdb(self, flushing_mode: FlushingMode) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FLUSHDB").arg(flushing_mode))
    }

    /// Delete all the keys of all the existing databases, not just the currently selected one.
    ///
    /// # See Also
    /// [<https://redis.io/commands/flushall/>](https://redis.io/commands/flushall/)
    #[must_use]
    fn flushall(self, flushing_mode: FlushingMode) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FLUSHALL").arg(flushing_mode))
    }

    /// This command returns information and statistics about the server
    /// in a format that is simple to parse by computers and easy to read by humans.
    ///
    /// # See Also
    /// [<https://redis.io/commands/info/>](https://redis.io/commands/info/)
    #[must_use]
    fn info<SS>(self, sections: SS) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
        SS: SingleArgCollection<InfoSection>,
    {
        prepare_command(self, cmd("INFO").arg(sections))
    }

    /// Return the UNIX TIME of the last DB save executed with success.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lastsave/>](https://redis.io/commands/lastsave/)
    #[must_use]
    fn lastsave(self) -> PreparedCommand<'a, Self, u64>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("LASTSAVE"))
    }

    /// This command reports about different latency-related issues and advises about possible remedies.
    ///
    /// # Return
    /// String report
    ///
    /// # See Also
    /// [<https://redis.io/commands/latency-doctor/>](https://redis.io/commands/latency-doctor/)
    #[must_use]
    fn latency_doctor(self) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("LATENCY").arg("DOCTOR"))
    }

    /// Produces an ASCII-art style graph for the specified event.
    ///
    /// # Return
    /// String graph
    ///
    /// # See Also
    /// [<https://redis.io/commands/latency-graph/>](https://redis.io/commands/latency-graph/)
    #[must_use]
    fn latency_graph(self, event: LatencyHistoryEvent) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("LATENCY").arg("GRAPH").arg(event))
    }

    /// This command reports a cumulative distribution of latencies
    /// in the format of a histogram for each of the specified command names.
    ///
    /// # Return
    /// The command returns a map where each key is a command name, and each value is a CommandHistogram instance.
    ///
    /// # See Also
    /// [<https://redis.io/commands/latency-histogram/>](https://redis.io/commands/latency-histogram/)
    #[must_use]
    fn latency_histogram<C, CC, RR>(self, commands: CC) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        C: SingleArg,
        CC: SingleArgCollection<C>,
        RR: KeyValueCollectionResponse<String, CommandHistogram>,
    {
        prepare_command(self, cmd("LATENCY").arg("HISTOGRAM").arg(commands))
    }

    /// This command returns the raw data of the event's latency spikes time series.
    ///
    /// # Return
    /// The command returns a collection where each element is a two elements tuple representing
    /// - the unix timestamp in seconds
    /// - the latency of the event in milliseconds
    ///
    /// # See Also
    /// [<https://redis.io/commands/latency-history/>](https://redis.io/commands/latency-history/)
    #[must_use]
    fn latency_history<RR>(self, event: LatencyHistoryEvent) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        RR: CollectionResponse<(u32, u32)>,
    {
        prepare_command(self, cmd("LATENCY").arg("HISTORY").arg(event))
    }

    /// This command reports the latest latency events logged.
    ///
    /// # Return
    /// A collection of the latest latency events logged.
    /// Each reported event has the following fields:
    /// - Event name.
    /// - Unix timestamp of the latest latency spike for the event.
    /// - Latest event latency in millisecond.
    /// - All-time maximum latency for this event.
    ///
    /// "All-time" means the maximum latency since the Redis instance was started,
    /// or the time that events were [`reset`](crate::commands::ConnectionCommands::reset).
    ///
    /// # See Also
    /// [<https://redis.io/commands/latency-latest/>](https://redis.io/commands/latency-latest/)
    #[must_use]
    fn latency_latest<RR>(self) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        RR: CollectionResponse<(String, u32, u32, u32)>,
    {
        prepare_command(self, cmd("LATENCY").arg("LATEST"))
    }

    /// This command resets the latency spikes time series of all, or only some, events.
    ///
    /// # Return
    /// the number of event time series that were reset.
    ///
    /// # See Also
    /// [<https://redis.io/commands/latency-latest/>](https://redis.io/commands/latency-latest/)
    #[must_use]
    fn latency_reset<EE>(self, events: EE) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        EE: SingleArgCollection<LatencyHistoryEvent>,
    {
        prepare_command(self, cmd("LATENCY").arg("RESET").arg(events))
    }

    /// The LOLWUT command displays the Redis version: however as a side effect of doing so,
    /// it also creates a piece of generative computer art that is different with each version of Redis.
    ///
    /// # Return
    /// the string containing the generative computer art, and a text with the Redis version.
    ///
    /// # See Also
    /// [<https://redis.io/commands/lolwut/>](https://redis.io/commands/lolwut/)
    #[must_use]
    fn lolwut(self, options: LolWutOptions) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("LOLWUT").arg(options))
    }

    /// This command reports about different memory-related issues that
    /// the Redis server experiences, and advises about possible remedies.
    ///
    /// # Return
    /// the string report.
    ///
    /// # See Also
    /// [<https://redis.io/commands/memory-doctor/>](https://redis.io/commands/memory-doctor/)
    #[must_use]
    fn memory_doctor(self) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("MEMORY").arg("DOCTOR"))
    }

    /// This command provides an internal statistics report from the memory allocator.
    ///
    /// # Return
    /// the memory allocator's internal statistics report.
    ///
    /// # See Also
    /// [<https://redis.io/commands/memory-malloc-stats/>](https://redis.io/commands/memory-malloc-stats/)
    #[must_use]
    fn memory_malloc_stats(self) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("MEMORY").arg("MALLOC-STATS"))
    }

    /// This command attempts to purge dirty pages so these can be reclaimed by the allocator.
    ///
    /// # See Also
    /// [<https://redis.io/commands/memory-purge/>](https://redis.io/commands/memory-purge/)
    #[must_use]
    fn memory_purge(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("MEMORY").arg("PURGE"))
    }

    /// This command returns information about the memory usage of the server.
    ///
    /// # Return
    /// the memory allocator's internal statistics report.
    ///
    /// # See Also
    /// [<https://redis.io/commands/memory-stats/>](https://redis.io/commands/memory-stats/)
    #[must_use]
    fn memory_stats(self) -> PreparedCommand<'a, Self, MemoryStats>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("MEMORY").arg("STATS"))
    }

    /// This command reports the number of bytes that a key and its value require to be stored in RAM.
    ///
    /// # Return
    /// the memory usage in bytes, or None when the key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/memory-usage/>](https://redis.io/commands/memory-usage/)
    #[must_use]
    fn memory_usage<K>(
        self,
        key: K,
        options: MemoryUsageOptions,
    ) -> PreparedCommand<'a, Self, Option<usize>>
    where
        Self: Sized,
        K: SingleArg,
    {
        prepare_command(self, cmd("MEMORY").arg("USAGE").arg(key).arg(options))
    }

    /// Returns information about the modules loaded to the server.
    ///
    /// # Return
    /// list of loaded modules.
    /// Each element in the list represents a module as an instance of [`ModuleInfo`](ModuleInfo)
    ///
    /// # See Also
    /// [<https://redis.io/commands/module-list/>](https://redis.io/commands/module-list/)
    #[must_use]
    fn module_list<MM>(self) -> PreparedCommand<'a, Self, MM>
    where
        Self: Sized,
        MM: CollectionResponse<ModuleInfo>,
    {
        prepare_command(self, cmd("MODULE").arg("LIST"))
    }

    /// Loads a module from a dynamic library at runtime.
    ///
    /// # See Also
    /// [<https://redis.io/commands/module-load/>](https://redis.io/commands/module-load/)
    #[must_use]
    fn module_load<P>(self, path: P, options: ModuleLoadOptions) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        P: SingleArg,
    {
        prepare_command(self, cmd("MODULE").arg("LOADEX").arg(path).arg(options))
    }

    /// Unloads a module.
    ///
    /// # See Also
    /// [<https://redis.io/commands/module-unload/>](https://redis.io/commands/module-unload/)
    #[must_use]
    fn module_unload<N>(self, name: N) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("MODULE").arg("UNLOAD").arg(name))
    }

    /// This command can change the replication settings of a replica on the fly.
    ///
    /// # See Also
    /// [<https://redis.io/commands/replicaof/>](https://redis.io/commands/replicaof/)
    #[must_use]
    fn replicaof(self, options: ReplicaOfOptions) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("REPLICAOF").arg(options))
    }

    /// Provide information on the role of a Redis instance in the context of replication,
    /// by returning if the instance is currently a `master`, `slave`, or `sentinel`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/role/>](https://redis.io/commands/role/)
    #[must_use]
    fn role(self) -> PreparedCommand<'a, Self, RoleResult>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("ROLE"))
    }

    /// This command performs a synchronous save of the dataset producing a point in time snapshot
    /// of all the data inside the Redis instance, in the form of an RDB file.
    ///
    /// # See Also
    /// [<https://redis.io/commands/save/>](https://redis.io/commands/save/)
    #[must_use]
    fn save(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SAVE"))
    }

    /// Shutdown the server
    ///
    /// # See Also
    /// [<https://redis.io/commands/shutdown/>](https://redis.io/commands/shutdown/)
    #[must_use]
    fn shutdown(self, options: ShutdownOptions) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SHUTDOWN").arg(options))
    }

    /// This command returns entries from the slow log in chronological order.
    ///
    /// # See Also
    /// [<https://redis.io/commands/slowlog-get/>](https://redis.io/commands/slowlog-get/)
    #[must_use]
    fn slowlog_get(self, options: SlowLogOptions) -> PreparedCommand<'a, Self, Vec<SlowLogEntry>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SLOWLOG").arg("GET").arg(options))
    }

    /// This command returns the current number of entries in the slow log.
    ///
    /// # See Also
    /// [<https://redis.io/commands/slowlog-len/>](https://redis.io/commands/slowlog-len/)
    #[must_use]
    fn slowlog_len(self) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SLOWLOG").arg("LEN"))
    }

    /// This command resets the slow log, clearing all entries in it.
    ///
    /// # See Also
    /// [<https://redis.io/commands/slowlog-reset/>](https://redis.io/commands/slowlog-reset/)
    #[must_use]
    fn slowlog_reset(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SLOWLOG").arg("RESET"))
    }

    /// This command swaps two Redis databases,
    /// so that immediately all the clients connected to a given database
    /// will see the data of the other database, and the other way around.
    ///
    /// # See Also
    /// [<https://redis.io/commands/swapdb/>](https://redis.io/commands/swapdb/)
    #[must_use]
    fn swapdb(self, index1: usize, index2: usize) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SWAPDB").arg(index1).arg(index2))
    }

    /// The TIME command returns the current server time as a two items lists:
    /// a Unix timestamp and the amount of microseconds already elapsed in the current second.
    ///
    /// # See Also
    /// [<https://redis.io/commands/time/>](https://redis.io/commands/time/)
    #[must_use]
    fn time(self) -> PreparedCommand<'a, Self, (u32, u32)>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("TIME"))
    }
}

/// Database flushing mode
#[derive(Default)]
pub enum FlushingMode {
    #[default]
    Default,
    /// Flushes the database(s) asynchronously
    Async,
    /// Flushed the database(s) synchronously
    Sync,
}

impl ToArgs for FlushingMode {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            FlushingMode::Default => {
            }
            FlushingMode::Async => {
                args.arg("ASYNC");
            }
            FlushingMode::Sync => {
                args.arg("SYNC");
            }
        }
    }
}

/// Options for the [`acl_cat`](ServerCommands::acl_cat) command
#[derive(Default)]
pub struct AclCatOptions {
    command_args: CommandArgs,
}

impl AclCatOptions {
    #[must_use]
    pub fn category_name<C: SingleArg>(mut self, category_name: C) -> Self {
        Self {
            command_args: self.command_args.arg(category_name).build(),
        }
    }
}

impl ToArgs for AclCatOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`acl_dryrun`](ServerCommands::acl_dryrun) command
#[derive(Default)]
pub struct AclDryRunOptions {
    command_args: CommandArgs,
}

impl AclDryRunOptions {
    #[must_use]
    pub fn arg<A, AA>(mut self, args: AA) -> Self
    where
        A: SingleArg,
        AA: SingleArgCollection<A>,
    {
        Self {
            command_args: self.command_args.arg(args).build(),
        }
    }
}

impl ToArgs for AclDryRunOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`acl_genpass`](ServerCommands::acl_genpass) command
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
    pub fn bits(mut self, bits: usize) -> Self {
        Self {
            command_args: self.command_args.arg(bits).build(),
        }
    }
}

impl ToArgs for AclGenPassOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`acl_log`](ServerCommands::acl_log) command
#[derive(Default)]
pub struct AclLogOptions {
    command_args: CommandArgs,
}

impl AclLogOptions {
    /// This optional argument specifies how many entries to show.
    /// By default up to ten failures are returned.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg(count).build(),
        }
    }

    /// The special RESET argument clears the log.
    #[must_use]
    pub fn reset(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("RESET").build(),
        }
    }
}

impl ToArgs for AclLogOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Command info result for the [`command`](ServerCommands::command) command.
#[derive(Debug, Clone, Deserialize)]
pub struct CommandInfo {
    /// This is the command's name in lowercase.
    pub name: String,
    /// Arity is the number of arguments a command expects. It follows a simple pattern:
    /// - A positive integer means a fixed number of arguments.
    /// - A negative integer means a minimal number of arguments.
    pub arity: isize,
    /// Command flags are an array.
    /// See [COMMAND documentation](https://redis.io/commands/command/) for the list of flags
    pub flags: Vec<String>,
    /// The position of the command's first key name argument.
    /// For most commands, the first key's position is 1. Position 0 is always the command name itself.
    pub first_key: usize,
    /// The position of the command's last key name argument.
    pub last_key: isize,
    /// The step, or increment, between the first key and the position of the next key.
    pub step: usize,
    /// [From Redis 6.0] This is an array of simple strings that are the ACL categories to which the command belongs.
    pub acl_categories: Vec<String>,
    /// [From Redis 7.0] Helpful information about the command. To be used by clients/proxies.
    /// See [<https://redis.io/docs/reference/command-tips/>](https://redis.io/docs/reference/command-tips/)
    #[serde(default)]
    pub command_tips: Vec<CommandTip>,
    /// [From Redis 7.0] This is an array consisting of the command's key specifications.
    /// See [<https://redis.io/docs/reference/key-specs/>](https://redis.io/docs/reference/key-specs/)
    #[serde(default)]
    pub key_specifications: Vec<KeySpecification>,
    #[serde(default)]
    pub sub_commands: Vec<CommandInfo>,
}

/// Get additional information about a command
///
/// See <https://redis.io/docs/reference/command-tips/>
#[derive(Debug, Clone)]
pub enum CommandTip {
    NonDeterministricOutput,
    NonDeterministricOutputOrder,
    RequestPolicy(RequestPolicy),
    ResponsePolicy(ResponsePolicy),
}

impl<'de> Deserialize<'de> for CommandTip {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tip = <&str>::deserialize(deserializer)?;
        match tip {
            "nondeterministic_output" => Ok(CommandTip::NonDeterministricOutput),
            "nondeterministic_output_order" => Ok(CommandTip::NonDeterministricOutputOrder),
            _ => {
                let mut parts = tip.split(':');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some("request_policy"), Some(policy), None) => {
                        match RequestPolicy::from_str(policy) {
                            Ok(request_policy) => Ok(CommandTip::RequestPolicy(request_policy)),
                            Err(_) => Err(de::Error::invalid_value(
                                de::Unexpected::Str(policy),
                                &"a valid RequestPolicy value",
                            )),
                        }
                    }
                    (Some("response_policy"), Some(policy), None) => {
                        match ResponsePolicy::from_str(policy) {
                            Ok(response_policy) => Ok(CommandTip::ResponsePolicy(response_policy)),
                            Err(_) => Err(de::Error::invalid_value(
                                de::Unexpected::Str(policy),
                                &"a valid ResponsePolicy value",
                            )),
                        }
                    }
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(tip),
                        &"a valid CommandTip value",
                    )),
                }
            }
        }
    }
}

/// This tip can help clients determine the shards to send the command in clustering mode.
///
/// The default behavior a client should implement for commands without the request_policy tip is as follows:
/// 1. The command doesn't accept key name arguments: the client can execute the command on an arbitrary shard.
/// 2. For commands that accept one or more key name arguments: the client should route the command to a single shard,
/// as determined by the hash slot of the input keys.
#[derive(Debug, Clone, Deserialize)]
pub enum RequestPolicy {
    /// the client should execute the command on all nodes - masters and replicas alike.
    ///
    /// An example is the [`config_set`](ServerCommands::config_set) command.
    /// This tip is in-use by commands that don't accept key name arguments. The command operates atomically per shard.
    AllNodes,
    /// the client should execute the command on all master shards (e.g., the [`dbsize`](ServerCommands::dbsize) command).
    ///
    /// This tip is in-use by commands that don't accept key name arguments. The command operates atomically per shard.
    AllShards,
    /// the client should execute the command on several shards.
    ///
    /// The shards that execute the command are determined by the hash slots of its input key name arguments.
    /// Examples for such commands include [`mset`](crate::commands::StringCommands::mset), [`mget`](crate::commands::StringCommands::mget)
    /// and [`del`](crate::commands::GenericCommands::del).
    /// However, note that [`sunionstore`](crate::commands::SetCommands::sunionstore) isn't considered
    /// as multi_shard because all of its keys must belong to the same hash slot.
    MultiShard,
    /// indicates a non-trivial form of the client's request policy, such as the [`scan`](crate::commands::GenericCommands::scan) command.
    Special,
}

impl FromStr for RequestPolicy {
    type Err = Error;

    fn from_str(str: &str) -> Result<Self> {
        match str {
            "all_nodes" => Ok(RequestPolicy::AllNodes),
            "all_shards" => Ok(RequestPolicy::AllShards),
            "multi_shard" => Ok(RequestPolicy::MultiShard),
            "special" => Ok(RequestPolicy::Special),
            _ => Err(Error::Client(
                "Cannot parse RequestPolicy from result".to_owned(),
            )),
        }
    }
}

/// This tip can help clients determine the aggregate they need to compute from the replies of multiple shards in a cluster.
///
/// The default behavior for commands without a request_policy tip only applies to replies with of nested types
/// (i.e., an array, a set, or a map).
/// The client's implementation for the default behavior should be as follows:
/// 1. The command doesn't accept key name arguments: the client can aggregate all replies within a single nested data structure.
/// For example, the array replies we get from calling [`keys`](crate::commands::GenericCommands::keys) against all shards.
/// These should be packed in a single in no particular order.
/// 2. For commands that accept one or more key name arguments: the client needs to retain the same order of replies as the input key names.
/// For example, [`mget`](crate::commands::StringCommands::mget)'s aggregated reply.
#[derive(Debug, Clone, Deserialize)]
pub enum ResponsePolicy {
    /// the clients should return success if at least one shard didn't reply with an error.
    ///
    /// The client should reply with the first non-error reply it obtains.
    /// If all shards return an error, the client can reply with any one of these.
    /// For example, consider a [`script_kill`](crate::commands::ScriptingCommands::script_kill) command that's sent to all shards.
    /// Although the script should be loaded in all of the cluster's shards,
    /// the [`script_kill`](crate::commands::ScriptingCommands::script_kill) will typically run only on one at a given time.
    OneSucceeded,
    /// the client should return successfully only if there are no error replies.
    ///
    /// Even a single error reply should disqualify the aggregate and be returned.
    /// Otherwise, the client should return one of the non-error replies.
    /// As an example, consider the [`config_set`](ServerCommands::config_set),
    /// [`script_flush`](crate::commands::ScriptingCommands::script_flush) and
    /// [`script_load`](crate::commands::ScriptingCommands::script_load) commands.
    AllSucceeded,
    /// the client should return the result of a logical `AND` operation on all replies
    /// (only applies to integer replies, usually from commands that return either 0 or 1).
    ///
    /// Consider the [`script_exists`](crate::commands::ScriptingCommands::script_exists) command as an example.
    /// It returns an array of 0's and 1's that denote the existence of its given SHA1 sums in the script cache.
    /// The aggregated response should be 1 only when all shards had reported that a given script SHA1 sum is in their respective cache.
    AggLogicalAnd,
    /// the client should return the result of a logical `OR` operation on all replies
    /// (only applies to integer replies, usually from commands that return either 0 or 1).
    AggLogicalOr,
    /// the client should return the minimal value from the replies (only applies to numerical replies).
    ///
    /// The aggregate reply from a cluster-wide [`wait`](crate::commands::GenericCommands::wait) command, for example,
    /// should be the minimal value (number of synchronized replicas) from all shards
    AggMin,
    /// the client should return the maximal value from the replies (only applies to numerical replies).
    AggMax,
    /// the client should return the sum of replies (only applies to numerical replies).
    ///
    /// Example: [`dbsize`](ServerCommands::dbsize).
    AggSum,
    /// this type of tip indicates a non-trivial form of reply policy.
    ///
    /// [`info`](ServerCommands::info) is an excellent example of that.
    Special,
}

impl FromStr for ResponsePolicy {
    type Err = Error;

    fn from_str(str: &str) -> Result<Self> {
        match str {
            "one_succeeded" => Ok(ResponsePolicy::OneSucceeded),
            "all_succeeded" => Ok(ResponsePolicy::AllSucceeded),
            "agg_logical_and" => Ok(ResponsePolicy::AggLogicalAnd),
            "agg_logical_or" => Ok(ResponsePolicy::AggLogicalOr),
            "agg_min" => Ok(ResponsePolicy::AggMin),
            "agg_max" => Ok(ResponsePolicy::AggMax),
            "agg_sum" => Ok(ResponsePolicy::AggSum),
            "special" => Ok(ResponsePolicy::Special),
            _ => Err(Error::Client(
                "Cannot parse ResponsePolicy from result".to_owned(),
            )),
        }
    }
}

/// Key specifications of a command for the [`command`](ServerCommands::command) command.
#[derive(Debug, Clone, Deserialize)]
pub struct KeySpecification {
    pub begin_search: BeginSearch,
    pub find_keys: FindKeys,
    pub flags: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// The BeginSearch value of a specification informs
/// the client of the extraction's beginning
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "spec")]
#[serde(rename_all = "lowercase")]
pub enum BeginSearch {
    #[serde(deserialize_with = "deserialize_begin_search_idx")]
    Index(usize),
    Keyword {
        keyword: String,
        #[serde(rename = "startfrom")]
        start_from: isize,
    },
    #[serde(deserialize_with = "deserialize_begin_search_unknown")]
    Unknown,
}

fn deserialize_begin_search_idx<'de, D>(deserializer: D) -> std::result::Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let map = HashMap::<String, usize>::deserialize(deserializer)?;
    let index = map
        .get("index")
        .ok_or_else(|| de::Error::custom("Cannot parse BeginSearch index"))?;
    Ok(*index)
}

fn deserialize_begin_search_unknown<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    let map = HashMap::<String, ()>::deserialize(deserializer)?;
    assert!(map.is_empty());
    Ok(())
}

/// The FindKeys value of a key specification tells the client
/// how to continue the search for key names.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "spec")]
#[serde(rename_all = "lowercase")]
pub enum FindKeys {
    Range {
        #[serde(rename = "lastkey")]
        last_key: isize,
        #[serde(rename = "keystep")]
        key_step: usize,
        limit: usize,
    },
    KeyNum {
        #[serde(rename = "keynumidx")]
        key_num_idx: usize,
        #[serde(rename = "firstkey")]
        first_key: usize,
        #[serde(rename = "keystep")]
        key_step: usize,
    },
    Unknown {},
}

/// Command doc result for the [`command_docs`](ServerCommands::command_docs) command
#[derive(Debug, Default, Deserialize)]
pub struct CommandDoc {
    /// short command description.
    pub summary: String,
    /// the Redis version that added the command (or for module commands, the module version).
    pub since: String,
    /// he functional group to which the command belongs.
    pub group: String,
    /// a short explanation about the command's time complexity.
    pub complexity: String,
    /// an array of documentation flags. Possible values are:
    /// - `deprecated`: the command is deprecated.
    /// - `syscmd`: a system command that isn't meant to be called by users.
    #[serde(default)]
    pub doc_flags: Vec<CommandDocFlag>,
    /// the Redis version that deprecated the command (or for module commands, the module version).
    #[serde(default)]
    pub deprecated_since: String,
    /// the alternative for a deprecated command.
    #[serde(default)]
    pub replaced_by: String,
    /// an array of historical notes describing changes to the command's behavior or arguments.
    #[serde(default)]
    pub history: Vec<HistoricalNote>,
    /// an array of [`command arguments`](https://redis.io/docs/reference/command-arguments/)
    pub arguments: Vec<CommandArgument>,
}

/// Command documenation flag
#[derive(Debug, Deserialize)]
pub enum CommandDocFlag {
    /// the command is deprecated.
    Deprecated,
    /// a system command that isn't meant to be called by users.
    SystemCommand,
}

/// Sub-result for the [`command_docs`](ServerCommands::command_docs) command
#[derive(Debug, Deserialize)]
pub struct HistoricalNote {
    pub version: String,
    pub description: String,
}

/// [`command argument`](https://redis.io/docs/reference/command-arguments/)
#[derive(Debug, Deserialize)]
pub struct CommandArgument {
    ///  the argument's name, always present.
    pub name: String,
    /// the argument's display string, present in arguments that have a displayable representation
    #[serde(default)]
    pub display_text: String,
    ///  the argument's type, always present.
    #[serde(rename = "type")]
    pub type_: CommandArgumentType,
    /// this value is available for every argument of the `key` type.
    /// t is a 0-based index of the specification in the command's [`key specifications`](https://redis.io/topics/key-specs)
    /// that corresponds to the argument.
    #[serde(default)]
    pub key_spec_index: usize,
    /// a constant literal that precedes the argument (user input) itself.
    #[serde(default)]
    pub token: String,
    /// a short description of the argument.
    #[serde(default)]
    pub summary: String,
    /// the debut Redis version of the argument (or for module commands, the module version).
    #[serde(default)]
    pub since: String,
    /// the Redis version that deprecated the command (or for module commands, the module version).
    #[serde(default)]
    pub deprecated_since: String,
    /// an array of argument flags.
    #[serde(default)]
    pub flags: Vec<ArgumentFlag>,
    /// the argument's value.
    #[serde(default)]
    pub value: Vec<String>,
}

/// An argument must have one of the following types:
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandArgumentType {
    /// a string argument.
    String,
    /// an integer argument.
    Integer,
    /// a double-precision argument.
    Double,
    /// a string that represents the name of a key.
    Key,
    /// a string that represents a glob-like pattern.
    Pattern,
    /// an integer that represents a Unix timestamp.
    UnixTime,
    /// a token, i.e. a reserved keyword, which may or may not be provided.
    /// Not to be confused with free-text user input.
    PureToken,
    /// the argument is a container for nested arguments.
    /// This type enables choice among several nested arguments
    Oneof,
    /// the argument is a container for nested arguments.
    /// This type enables grouping arguments and applying a property (such as optional) to all
    Block,
}

/// Flag for a command argument
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArgumentFlag {
    /// denotes that the argument is optional (for example, the GET clause of the SET command).
    Optional,
    /// denotes that the argument may be repeated (such as the key argument of DEL).
    Multiple,
    ///  denotes the possible repetition of the argument with its preceding token (see SORT's GET pattern clause).
    MultipleToken,
}

/// Options for the [`command_list`](ServerCommands::command_list) command.
#[derive(Default)]
pub struct CommandListOptions {
    command_args: CommandArgs,
}

impl CommandListOptions {
    /// get the commands that belong to the module specified by `module-name`.
    #[must_use]
    pub fn filter_by_module_name<M: SingleArg>(mut self, module_name: M) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTERBY")
                .arg("MODULE")
                .arg(module_name)
                .build(),
        }
    }

    /// get the commands in the [`ACL category`](https://redis.io/docs/manual/security/acl/#command-categories) specified by `category`.
    #[must_use]
    pub fn filter_by_acl_category<C: SingleArg>(mut self, category: C) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTERBY")
                .arg("ACLCAT")
                .arg(category)
                .build(),
        }
    }

    /// get the commands that match the given glob-like `pattern`.
    #[must_use]
    pub fn filter_by_pattern<P: SingleArg>(mut self, pattern: P) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTERBY")
                .arg("PATTERN")
                .arg(pattern)
                .build(),
        }
    }
}

impl ToArgs for CommandListOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`failover`](ServerCommands::failover) command.
#[derive(Default)]
pub struct FailOverOptions {
    command_args: CommandArgs,
}

impl FailOverOptions {
    /// This option allows designating a specific replica, by its host and port, to failover to.
    #[must_use]
    pub fn to<H: SingleArg>(mut self, host: H, port: u16) -> Self {
        Self {
            command_args: self.command_args.arg("TO").arg(host).arg(port).build(),
        }
    }

    /// This option allows specifying a maximum time a master will wait in the waiting-for-sync state
    /// before aborting the failover attempt and rolling back.
    #[must_use]
    pub fn timeout(mut self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TIMEOUT").arg(milliseconds).build(),
        }
    }

    /// If both the [`timeout`](FailOverOptions::timeout) and [`to`](FailOverOptions::to) options are set,
    /// the force flag can also be used to designate that that once the timeout has elapsed,
    /// the master should failover to the target replica instead of rolling back.
    #[must_use]
    pub fn force(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("FORCE").build(),
        }
    }

    /// This command will abort an ongoing failover and return the master to its normal state.
    #[must_use]
    pub fn abort(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("ABORT").build(),
        }
    }
}

impl ToArgs for FailOverOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Section for the [`info`](ServerCommands::info) command.
pub enum InfoSection {
    Server,
    Clients,
    Memory,
    Persistence,
    Stats,
    Replication,
    Cpu,
    Commandstats,
    Latencystats,
    Cluster,
    Keyspace,
    Modules,
    Errorstats,
    All,
    Default,
    Everything,
}

impl SingleArg for InfoSection {}

impl ToArgs for InfoSection {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            InfoSection::Server => "server",
            InfoSection::Clients => "clients",
            InfoSection::Memory => "memory",
            InfoSection::Persistence => "persistence",
            InfoSection::Stats => "stats",
            InfoSection::Replication => "replication",
            InfoSection::Cpu => "cpu",
            InfoSection::Commandstats => "commandstats",
            InfoSection::Latencystats => "latencystats",
            InfoSection::Cluster => "cluster",
            InfoSection::Keyspace => "keyspace",
            InfoSection::Modules => "modules",
            InfoSection::Errorstats => "errorstats",
            InfoSection::All => "all",
            InfoSection::Default => "default",
            InfoSection::Everything => "everything",
        });
    }
}

/// Latency history event for the [`latency_graph`](ServerCommands::latency_graph)
/// & [`latency_history`](ServerCommands::latency_history) commands.
pub enum LatencyHistoryEvent {
    ActiveDefragCycle,
    AofFsyncAlways,
    AofStat,
    AofRewriteDiffWrite,
    AofRename,
    AofWrite,
    AofWriteActiveChild,
    AofWriteAlone,
    AofWritePendingFsync,
    Command,
    ExpireCycle,
    EvictionCycle,
    EvictionDel,
    FastCommand,
    Fork,
    RdbUnlinkTempFile,
}

impl SingleArg for LatencyHistoryEvent {}

impl ToArgs for LatencyHistoryEvent {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            LatencyHistoryEvent::ActiveDefragCycle => "active-defrag-cycle",
            LatencyHistoryEvent::AofFsyncAlways => "aof-fsync-always",
            LatencyHistoryEvent::AofStat => "aof-stat",
            LatencyHistoryEvent::AofRewriteDiffWrite => "aof-rewrite-diff-write",
            LatencyHistoryEvent::AofRename => "aof-rename",
            LatencyHistoryEvent::AofWrite => "aof-write",
            LatencyHistoryEvent::AofWriteActiveChild => "aof-write-active-child",
            LatencyHistoryEvent::AofWriteAlone => "aof-write-alone",
            LatencyHistoryEvent::AofWritePendingFsync => "aof-write-pending-fsync",
            LatencyHistoryEvent::Command => "command",
            LatencyHistoryEvent::ExpireCycle => "expire-cycle",
            LatencyHistoryEvent::EvictionCycle => "eviction-cycle",
            LatencyHistoryEvent::EvictionDel => "eviction-del",
            LatencyHistoryEvent::FastCommand => "fast-command",
            LatencyHistoryEvent::Fork => "fork",
            LatencyHistoryEvent::RdbUnlinkTempFile => "rdb-unlink-temp-file",
        });
    }
}

/// Command Histogram for the [`latency_histogram`](ServerCommands::latency_histogram) commands.
#[derive(Default, Deserialize)]
pub struct CommandHistogram {
    /// The total calls for that command.
    pub calls: usize,

    /// A map of time buckets:
    /// - Each bucket represents a latency range.
    /// - Each bucket covers twice the previous bucket's range.
    /// - Empty buckets are not printed.
    /// - The tracked latencies are between 1 microsecond and roughly 1 second.
    /// - Everything above 1 sec is considered +Inf.
    /// - At max there will be log2(1000000000)=30 buckets.
    pub histogram_usec: HashMap<u32, u32>,
}

/// Options for the [`lolwut`](ServerCommands::lolwut) command
#[derive(Default)]
pub struct LolWutOptions {
    command_args: CommandArgs,
}

impl LolWutOptions {
    #[must_use]
    pub fn version(mut self, version: usize) -> Self {
        Self {
            command_args: self.command_args.arg("VERSION").arg(version).build(),
        }
    }

    #[must_use]
    pub fn optional_arg<A: SingleArg>(mut self, arg: A) -> Self {
        Self {
            command_args: self.command_args.arg(arg).build(),
        }
    }
}

impl ToArgs for LolWutOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`memory_stats`](ServerCommands::memory_stats) command.
#[derive(Debug, Deserialize)]
pub struct MemoryStats {
    /// Peak memory consumed by Redis in bytes
    /// (see [`INFO`](https://redis.io/commands/info)'s used_memory_peak)
    #[serde(rename = "peak.allocated")]
    pub peak_allocated: usize,

    /// Total number of bytes allocated by Redis using its allocator
    /// (see [`INFO`](https://redis.io/commands/info)'s used_memory)
    #[serde(rename = "total.allocated")]
    pub total_allocated: usize,

    /// Initial amount of memory consumed by Redis at startup in bytes
    /// (see [`INFO`](https://redis.io/commands/info)'s used_memory_startup)
    #[serde(rename = "startup.allocated")]
    #[serde(default)]
    pub startup_allocated: usize,

    /// Size in bytes of the replication backlog
    /// (see [`INFO`](https://redis.io/commands/info)'s repl_backlog_active)
    #[serde(rename = "replication.backlog")]
    #[serde(default)]
    pub replication_backlog: usize,

    /// The total size in bytes of all replicas overheads
    /// (output and query buffers, connection contexts)
    #[serde(rename = "clients.slaves")]
    #[serde(default)]
    pub clients_slaves: usize,

    /// The total size in bytes of all clients overheads
    /// (output and query buffers, connection contexts)
    #[serde(rename = "clients.normal")]
    #[serde(default)]
    pub clients_normal: usize,

    /// Memory usage by cluster links
    /// (Added in Redis 7.0, see [`INFO`](https://redis.io/commands/info)'s mem_cluster_links).
    #[serde(rename = "cluster.links")]
    #[serde(default)]
    pub cluster_links: usize,

    /// The summed size in bytes of AOF related buffers.
    #[serde(rename = "aof.buffer")]
    #[serde(default)]
    pub aof_buffer: usize,

    /// the summed size in bytes of the overheads of the Lua scripts' caches
    #[serde(rename = "lua.caches")]
    #[serde(default)]
    pub lua_caches: usize,

    /// the summed size in bytes of the overheads of the functions' caches
    #[serde(rename = "functions.caches")]
    #[serde(default)]
    pub functions_caches: usize,

    /// The sum of all overheads, i.e. `startup.allocated`, `replication.backlog`,
    /// `clients.slaves`, `clients.normal`, `aof.buffer` and those of the internal data structures
    /// that are used in managing the Redis keyspace (see [`INFO`](https://redis.io/commands/info)'s used_memory_overhead)
    #[serde(rename = "overhead.total")]
    #[serde(default)]
    pub overhead_total: usize,

    /// The total number of keys stored across all databases in the server
    #[serde(rename = "keys.count")]
    #[serde(default)]
    pub keys_count: usize,

    /// The ratio between net memory usage (`total.allocated` minus `startup.allocated`) and `keys.count`
    #[serde(rename = "keys.bytes-per-key")]
    #[serde(default)]
    pub keys_bytes_per_key: usize,

    /// The size in bytes of the dataset, i.e. `overhead.total` subtracted from `total.allocated`
    ///  (see [`INFO`](https://redis.io/commands/info)'s used_memory_dataset)
    #[serde(rename = "dataset.bytes")]
    #[serde(default)]
    pub dataset_bytes: usize,

    /// The percentage of `dataset.bytes` out of the net memory usage
    #[serde(rename = "dataset.percentage")]
    #[serde(default)]
    pub dataset_percentage: f64,

    /// The percentage of `peak.allocated` out of `total.allocated`
    #[serde(rename = "peak.percentage")]
    #[serde(default)]
    pub peak_percentage: f64,

    #[serde(rename = "allocator.allocated")]
    #[serde(default)]
    pub allocator_allocated: usize,

    #[serde(rename = "allocator.active")]
    #[serde(default)]
    pub allocator_active: usize,

    #[serde(rename = "allocator.resident")]
    #[serde(default)]
    pub allocator_resident: usize,

    #[serde(rename = "allocator-fragmentation.ratio")]
    #[serde(default)]
    pub allocator_fragmentation_ratio: f64,

    #[serde(rename = "allocator-fragmentation.bytes")]
    #[serde(default)]
    pub allocator_fragmentation_bytes: isize,

    #[serde(rename = "allocator-rss.ratio")]
    #[serde(default)]
    pub allocator_rss_ratio: f64,

    #[serde(rename = "allocator-rss.bytes")]
    #[serde(default)]
    pub allocator_rss_bytes: isize,

    #[serde(rename = "rss-overhead.ratio")]
    #[serde(default)]
    pub rss_overhead_ratio: f64,

    #[serde(rename = "rss-overhead.bytes")]
    #[serde(default)]
    pub rss_overhead_bytes: isize,

    /// See [`INFO`](https://redis.io/commands/info)'s mem_fragmentation_ratio
    #[serde(rename = "fragmentation")]
    #[serde(default)]
    pub fragmentation: f64,

    #[serde(rename = "fragmentation.bytes")]
    #[serde(default)]
    pub fragmentation_bytes: isize,
}

/// Sub-result for the [`memory_stats`](ServerCommands::memory_stats) command.
#[derive(Debug, Deserialize)]
pub struct DatabaseOverhead {
    pub overhead_hashtable_main: usize,
    pub overhead_hashtable_expires: usize,
    pub overhead_hashtable_slot_to_keys: usize,
}

/// Options for the [`memory_usage`](ServerCommands::memory_usage) command
#[derive(Default)]
pub struct MemoryUsageOptions {
    command_args: CommandArgs,
}

impl MemoryUsageOptions {
    /// For nested data types, the optional `samples` option can be provided,
    /// where count is the number of sampled nested values.
    /// By default, this option is set to 5.
    /// To sample the all of the nested values, use samples(0).
    #[must_use]
    pub fn samples(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("SAMPLES").arg(count).build(),
        }
    }
}

impl ToArgs for MemoryUsageOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Module information result for the [`module_list`](ServerCommands::module_list) command.
#[derive(Deserialize)]
pub struct ModuleInfo {
    /// Name of the module
    pub name: String,
    /// Version of the module
    #[serde(default)]
    pub version: u64,
}

/// Options for the [`module_load`](ServerCommands::module_load) command
#[derive(Default)]
pub struct ModuleLoadOptions {
    command_args: CommandArgs,
    args_added: bool,
}

impl ModuleLoadOptions {
    /// You can use this optional associated function to provide the module with configuration directives.
    /// This associated function can be called multiple times
    #[must_use]
    pub fn config<N, V>(mut self, name: N, value: V) -> Self
    where
        N: SingleArg,
        V: SingleArg,
    {
        if self.args_added {
            panic!(
                "associated function `config` should be called before associated function `arg`"
            );
        }

        Self {
            command_args: self.command_args.arg("CONFIG").arg(name).arg(value).build(),
            args_added: false,
        }
    }

    /// Any additional arguments are passed unmodified to the module.
    /// This associated function can be called multiple times
    #[must_use]
    pub fn arg<A: SingleArg>(mut self, arg: A) -> Self {
        if !self.args_added {
            Self {
                command_args: self.command_args.arg("ARGS").arg(arg).build(),
                args_added: true,
            }
        } else {
            Self {
                command_args: self.command_args.arg(arg).build(),
                args_added: false,
            }
        }
    }
}

impl ToArgs for ModuleLoadOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// options for the [`replicaof`](ServerCommands::replicaof) command.
pub struct ReplicaOfOptions {
    command_args: CommandArgs,
}

impl ReplicaOfOptions {
    /// If a Redis server is already acting as replica,
    /// the command REPLICAOF NO ONE will turn off the replication,
    /// turning the Redis server into a MASTER.
    #[must_use]
    pub fn no_one() -> Self {
        Self {
            command_args: CommandArgs::default().arg("NO").arg("ONE").build(),
        }
    }

    /// In the proper form REPLICAOF hostname port will make the server
    /// a replica of another server listening at the specified hostname and port.
    #[must_use]
    pub fn master<H: SingleArg>(host: H, port: u16) -> Self {
        Self {
            command_args: CommandArgs::default().arg(host).arg(port).build(),
        }
    }
}

impl ToArgs for ReplicaOfOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`role`](ServerCommands::role) command.
#[derive(Debug)]
pub enum RoleResult {
    Master {
        /// The current master replication offset,
        /// which is an offset that masters and replicas share to understand,
        /// in partial resynchronizations,
        /// the part of the replication stream the replicas needs to fetch to continue.
        master_replication_offset: usize,
        /// information av=bout the connected replicas
        replica_infos: Vec<ReplicaInfo>,
    },
    Replica {
        /// The IP of the master.
        master_ip: String,
        /// The port number of the master.
        master_port: u16,
        /// The state of the replication from the point of view of the master
        state: ReplicationState,
        /// The amount of data received from the replica
        /// so far in terms of master replication offset.
        amount_data_received: isize,
    },
    Sentinel {
        /// An array of master names monitored by this Sentinel instance.
        master_names: Vec<String>,
    },
}

impl<'de> Deserialize<'de> for RoleResult {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RoleResultVisitor;

        impl<'de> Visitor<'de> for RoleResultVisitor {
            type Value = RoleResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("RoleResult")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let Some(role): Option<&str> = seq.next_element()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                match role {
                    "master" => {
                        let Some(master_replication_offset): Option<usize> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        let Some(replica_infos): Option<Vec<ReplicaInfo>> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(2, &"more elements in sequence"));
                        };
                        Ok(RoleResult::Master {
                            master_replication_offset,
                            replica_infos,
                        })
                    }
                    "slave" => {
                        let Some(master_ip): Option<String> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        let Some(master_port): Option<u16> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(2, &"more elements in sequence"));
                        };
                        let Some(state): Option<ReplicationState> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(3, &"more elements in sequence"));
                        };
                        let Some(amount_data_received): Option<isize> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(4, &"more elements in sequence"));
                        };
                        Ok(RoleResult::Replica {
                            master_ip,
                            master_port,
                            state,
                            amount_data_received,
                        })
                    }
                    "sentinel" => {
                        let Some(master_names): Option<Vec<String>> = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        Ok(RoleResult::Sentinel { master_names })
                    }
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(role),
                        &"expected `master`, `slave` or `sentinel`",
                    )),
                }
            }
        }

        deserializer.deserialize_seq(RoleResultVisitor)
    }
}

/// Represents a connected replicas to a master
///
/// returned by the [`role`](ServerCommands::role) command.
#[derive(Debug, Deserialize)]
pub struct ReplicaInfo {
    /// the replica IP
    pub ip: String,
    /// the replica port
    pub port: u16,
    /// the last acknowledged replication offset.
    pub last_ack_offset: usize,
}

/// The state of the replication from the point of view of the master,
///
/// returned by the [`role`](ServerCommands::role) command.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ReplicationState {
    /// the instance is in handshake with its master
    Handshake,
    /// the instance needs to connect to its master
    None,
    /// the instance in not active
    Connect,
    /// the master-replica connection is in progress
    Connecting,
    /// the master and replica are trying to perform the synchronization
    Sync,
    /// the replica is online
    Connected,
    /// instance state is unknown
    Unknown,
}

/// options for the [`shutdown`](ServerCommands::shutdown) command.
#[derive(Default)]
pub struct ShutdownOptions {
    command_args: CommandArgs,
}

impl ShutdownOptions {
    /// - if save is true, will force a DB saving operation even if no save points are configured
    /// - if save is false, will prevent a DB saving operation even if one or more save points are configured.
    #[must_use]
    pub fn save(mut self, save: bool) -> Self {
        Self {
            command_args: self
                .command_args
                .arg(if save { "SAVE" } else { "NOSAVE" })
                .build(),
        }
    }

    /// skips waiting for lagging replicas, i.e. it bypasses the first step in the shutdown sequence.
    #[must_use]
    pub fn now(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOW").build(),
        }
    }

    /// ignores any errors that would normally prevent the server from exiting.
    #[must_use]
    pub fn force(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("FORCE").build(),
        }
    }

    /// cancels an ongoing shutdown and cannot be combined with other flags.
    #[must_use]
    pub fn abort(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("ABORT").build(),
        }
    }
}

impl ToArgs for ShutdownOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// options for the [`slowlog_get`](ServerCommands::slowlog_get) command.
#[derive(Default)]
pub struct SlowLogOptions {
    command_args: CommandArgs,
}

impl SlowLogOptions {
    /// limits the number of returned entries, so the command returns at most up to `count` entries.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg(count).build(),
        }
    }
}

impl ToArgs for SlowLogOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result [`slowlog_get`](ServerCommands::slowlog_get) for the command.
#[derive(Deserialize)]
pub struct SlowLogEntry {
    /// A unique progressive identifier for every slow log entry.
    pub id: i64,
    /// A unique progressive identifier for every slow log entry.
    pub unix_timestamp: u32,
    /// The amount of time needed for its execution, in microseconds.
    pub execution_time_micros: u64,
    /// The array composing the arguments of the command.
    pub command: Vec<String>,
    /// Client IP address and port.
    pub client_address: String,
    /// Client name if set via the CLIENT SETNAME command.
    pub client_name: String,
}
