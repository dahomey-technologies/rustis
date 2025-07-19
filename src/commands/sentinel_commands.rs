use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{
        CommandArgs, KeyValueArgsCollection, KeyValueCollectionResponse, MultipleArgsCollection,
        PrimitiveResponse, SingleArg, ToArgs, Value, cmd,
    },
};
use serde::Deserialize;

/// A group of Redis commands related to [Sentinel](https://redis.io/docs/management/sentinel/)
/// # See Also
/// [Sentinel Commands](https://redis.io/docs/management/sentinel/#sentinel-commands)
pub trait SentinelCommands<'a> {
    /// Get the current value of a global Sentinel configuration parameter.
    ///
    /// The specified name may be a wildcard.
    /// Similar to the Redis [`config_get`](crate::commands::ServerCommands::config_get) command.
    #[must_use]
    fn sentinel_config_get<N, RN, RV, R>(self, name: N) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        N: SingleArg,
        RN: PrimitiveResponse,
        RV: PrimitiveResponse,
        R: KeyValueCollectionResponse<RN, RV>,
    {
        prepare_command(self, cmd("SENTINEL").arg("CONFIG").arg("GET").arg(name))
    }

    /// Set the value of a global Sentinel configuration parameter.
    #[must_use]
    fn sentinel_config_set<N, V>(self, name: N, value: V) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
        V: SingleArg,
    {
        prepare_command(
            self,
            cmd("SENTINEL")
                .arg("CONFIG")
                .arg("SET")
                .arg(name)
                .arg(value),
        )
    }

    /// Check if the current Sentinel configuration is able to reach the quorum needed to failover a master,
    /// and the majority needed to authorize the failover.
    ///
    /// This command should be used in monitoring systems to check if a Sentinel deployment is ok.
    #[must_use]
    fn sentinel_ckquorum<N>(self, master_name: N) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("CKQUORUM").arg(master_name))
    }

    /// Force a failover as if the master was not reachable,
    /// and without asking for agreement to other Sentinels
    /// (however a new version of the configuration will be published
    /// so that the other Sentinels will update their configurations).
    #[must_use]
    fn sentinel_failover<N>(self, master_name: N) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("FAILOVER").arg(master_name))
    }

    /// Force Sentinel to rewrite its configuration on disk, including the current Sentinel state.
    ///
    /// Normally Sentinel rewrites the configuration every time something changes in its state
    /// (in the context of the subset of the state which is persisted on disk across restart).
    ///  However sometimes it is possible that the configuration file is lost because of operation errors,
    /// disk failures, package upgrade scripts or configuration managers.
    /// In those cases a way to force Sentinel to rewrite the configuration file is handy.
    /// This command works even if the previous configuration file is completely missing.
    #[must_use]
    fn sentinel_flushconfig(self) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("FLUSHCONFIG"))
    }

    /// Return the ip and port number of the master with that name.
    ///
    /// If a failover is in progress or terminated successfully for this master,
    /// it returns the address and port of the promoted replica.
    ///
    /// # Return
    /// * `None` if sentinel does not know this master
    /// * A tuple made up of
    ///     * The IP of the master
    ///     * The port of the master
    #[must_use]
    fn sentinel_get_master_addr_by_name<N>(
        self,
        master_name: N,
    ) -> PreparedCommand<'a, Self, Option<(String, u16)>>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(
            self,
            cmd("SENTINEL")
                .arg("GET-MASTER-ADDR-BY-NAME")
                .arg(master_name),
        )
    }

    /// Return cached [`info`](crate::commands::ServerCommands::info) output from masters and replicas.
    #[must_use]
    fn sentinel_info_cache<N, NN, R>(self, master_names: NN) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        N: SingleArg,
        NN: MultipleArgsCollection<N>,
        R: KeyValueCollectionResponse<String, Vec<(u64, String)>>,
    {
        prepare_command(self, cmd("SENTINEL").arg("INFO-CACHE").arg(master_names))
    }

    /// Show the state and info of the specified master.
    #[must_use]
    fn sentinel_master<N>(self, master_name: N) -> PreparedCommand<'a, Self, SentinelMasterInfo>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("MASTER").arg(master_name))
    }

    /// Show a list of monitored masters and their state.
    #[must_use]
    fn sentinel_masters(self) -> PreparedCommand<'a, Self, Vec<SentinelMasterInfo>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("MASTERS"))
    }

    /// This command tells the Sentinel to start monitoring a new master
    /// with the specified name, ip, port, and quorum.
    ///
    /// It is identical to the sentinel monitor configuration directive in `sentinel.conf` configuration file,
    /// with the difference that you can't use a hostname in as ip,
    /// but you need to provide an IPv4 or IPv6 address.
    #[must_use]
    fn sentinel_monitor<N, I>(
        self,
        name: N,
        ip: I,
        port: u16,
        quorum: usize,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
        I: SingleArg,
    {
        prepare_command(
            self,
            cmd("SENTINEL")
                .arg("MONITOR")
                .arg(name)
                .arg(ip)
                .arg(port)
                .arg(quorum),
        )
    }

    /// This command is used in order to remove the specified master.
    ///
    /// The master will no longer be monitored,
    /// and will totally be removed from the internal state of the Sentinel,
    /// so it will no longer listed by [`sentinel_masters`](SentinelCommands::sentinel_masters) and so forth.
    #[must_use]
    fn sentinel_remove<N>(self, name: N) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("REMOVE").arg(name))
    }

    /// The SET command is very similar to the [`config_set`](crate::commands::ServerCommands::config_set) command of Redis,
    /// and is used in order to change configuration parameters of a specific master.
    ///
    /// Multiple option / value pairs can be specified (or none at all).
    /// All the configuration parameters that can be configured via `sentinel.conf`
    /// are also configurable using this command.
    #[must_use]
    fn sentinel_set<N, O, V, C>(self, name: N, configs: C) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        N: SingleArg,
        O: SingleArg,
        V: SingleArg,
        C: KeyValueArgsCollection<O, V>,
    {
        prepare_command(self, cmd("SENTINEL").arg("SET").arg(name).arg(configs))
    }

    /// Return the ID of the Sentinel instance.
    #[must_use]
    fn sentinel_myid(self) -> PreparedCommand<'a, Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("MYID"))
    }

    /// This command returns information about pending scripts.
    #[must_use]
    fn sentinel_pending_scripts(self) -> PreparedCommand<'a, Self, Vec<Value>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("PENDING-SCRIPTS"))
    }

    /// Show a list of replicas for this master, and their state.
    #[must_use]
    fn sentinel_replicas<N>(
        self,
        master_name: N,
    ) -> PreparedCommand<'a, Self, Vec<SentinelReplicaInfo>>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("REPLICAS").arg(master_name))
    }

    /// This command will reset all the masters with matching name.
    ///
    /// The pattern argument is a glob-style pattern.
    /// The reset process clears any previous state in a master (including a failover in progress),
    /// and removes every replica and sentinel already discovered and associated with the master.
    ///
    /// # Return
    /// The number of reset masters
    #[must_use]
    fn sentinel_reset<P>(self, pattern: P) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        P: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("RESET").arg(pattern))
    }

    ///  Show a list of sentinel instances for this master, and their state.
    #[must_use]
    fn sentinel_sentinels<N>(self, master_name: N) -> PreparedCommand<'a, Self, Vec<SentinelInfo>>
    where
        Self: Sized,
        N: SingleArg,
    {
        prepare_command(self, cmd("SENTINEL").arg("SENTINELS").arg(master_name))
    }

    ///  This command simulates different Sentinel crash scenarios.
    #[must_use]
    fn sentinel_simulate_failure(
        self,
        mode: SentinelSimulateFailureMode,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("SIMULATE-FAILURE").arg(mode))
    }
}

/// Result for the [`sentinel_master`](SentinelCommands::sentinel_master) command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SentinelMasterInfo {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub runid: String,
    pub flags: String,
    pub link_pending_commands: usize,
    pub link_refcount: usize,
    pub last_ping_sent: usize,
    pub last_ok_ping_reply: usize,
    pub last_ping_reply: usize,
    pub down_after_milliseconds: u64,
    pub info_refresh: u64,
    pub role_reported: String,
    pub role_reported_time: u64,
    pub config_epoch: usize,
    pub num_slaves: usize,
    pub num_other_sentinels: usize,
    pub quorum: usize,
    pub failover_timeout: u64,
    pub parallel_syncs: usize,
}

/// /// Result for the [`sentinel_replicas`](SentinelCommands::sentinel_replicas) command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SentinelReplicaInfo {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub runid: String,
    pub flags: String,
    pub link_pending_commands: usize,
    pub link_refcount: usize,
    pub last_ping_sent: usize,
    pub last_ok_ping_reply: usize,
    pub last_ping_reply: usize,
    pub down_after_milliseconds: u64,
    pub info_refresh: u64,
    pub role_reported: String,
    pub role_reported_time: u64,
    pub master_link_down_time: u64,
    pub master_link_status: String,
    pub master_host: String,
    pub master_port: u16,
    pub slave_priority: u64,
    pub slave_repl_offset: u64,
    pub replica_announced: usize,
}

/// Result for the [`sentinel_sentinels`](SentinelCommands::sentinel_sentinels) command.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SentinelInfo {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub runid: String,
    pub flags: String,
    pub link_pending_commands: usize,
    pub link_refcount: usize,
    pub last_ping_sent: usize,
    pub last_ok_ping_reply: usize,
    pub last_ping_reply: usize,
    pub down_after_milliseconds: u64,
    pub last_hello_message: u64,
    pub voted_leader: String,
    pub voted_leader_epoch: usize,
}

/// Different crash simulation scenario modes for
/// the [`sentinel_simulate_failure`](SentinelCommands::sentinel_simulate_failure) command
pub enum SentinelSimulateFailureMode {
    CrashAfterElection,
    CrashAfterPromotion,
}

impl ToArgs for SentinelSimulateFailureMode {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            SentinelSimulateFailureMode::CrashAfterElection => "CRASH-AFTER-ELECTION",
            SentinelSimulateFailureMode::CrashAfterPromotion => "CRASH-AFTER-PROMOTION",
        });
    }
}
