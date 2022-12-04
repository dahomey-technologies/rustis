use std::collections::HashMap;

use crate::{
    prepare_command,
    resp::{
        cmd, ArgsOrCollection, CommandArg, CommandArgs, FromKeyValueValueArray, FromValue,
        HashMapExt, IntoArgs, KeyValueArgOrCollection, Value,
    },
    PreparedCommand, Result,
};

/// A group of Redis commands related to [Sentinel](https://redis.io/docs/management/sentinel/)
/// # See Also
/// [Sentinel Commands](https://redis.io/docs/management/sentinel/#sentinel-commands)
pub trait SentinelCommands {
    /// Get the current value of a global Sentinel configuration parameter.
    ///
    /// The specified name may be a wildcard.
    /// Similar to the Redis [`config_get`](crate::ServerCommands::config_get) command.
    #[must_use]
    fn sentinel_config_get<N, RN, RV, R>(&mut self, name: N) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        N: Into<CommandArg>,
        RN: FromValue,
        RV: FromValue,
        R: FromKeyValueValueArray<RN, RV>,
    {
        prepare_command(self, cmd("SENTINEL").arg("CONFIG").arg("GET").arg(name))
    }

    /// Set the value of a global Sentinel configuration parameter.
    #[must_use]
    fn sentinel_config_set<N, V>(&mut self, name: N, value: V) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<CommandArg>,
        V: Into<CommandArg>,
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
    fn sentinel_ckquorum<N>(&mut self, master_name: N) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<CommandArg>,
    {
        prepare_command(self, cmd("SENTINEL").arg("CKQUORUM").arg(master_name))
    }

    /// Force a failover as if the master was not reachable,
    /// and without asking for agreement to other Sentinels
    /// (however a new version of the configuration will be published
    /// so that the other Sentinels will update their configurations).
    #[must_use]
    fn sentinel_failover<N>(&mut self, master_name: N) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<CommandArg>,
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
    fn sentinel_flushconfig(&mut self) -> PreparedCommand<Self, ()>
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
        &mut self,
        master_name: N,
    ) -> PreparedCommand<Self, Option<(String, u16)>>
    where
        Self: Sized,
        N: Into<CommandArg>,
    {
        prepare_command(
            self,
            cmd("SENTINEL")
                .arg("GET-MASTER-ADDR-BY-NAME")
                .arg(master_name),
        )
    }

    /// Return cached [`info`](crate::ServerCommands::info) output from masters and replicas.
    #[must_use]
    fn sentinel_info_cache<N, NN, R>(&mut self, master_names: NN) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        N: Into<CommandArg>,
        NN: ArgsOrCollection<N>,
        R: FromKeyValueValueArray<String, Vec<(u64, String)>>,
    {
        prepare_command(self, cmd("SENTINEL").arg("INFO-CACHE").arg(master_names))
    }

    /// Show the state and info of the specified master.
    #[must_use]
    fn sentinel_master<N>(&mut self, master_name: N) -> PreparedCommand<Self, SentinelMasterInfo>
    where
        Self: Sized,
        N: Into<CommandArg>,
    {
        prepare_command(self, cmd("SENTINEL").arg("MASTER").arg(master_name))
    }

    /// Show a list of monitored masters and their state.
    #[must_use]
    fn sentinel_masters(&mut self) -> PreparedCommand<Self, Vec<SentinelMasterInfo>>
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
        &mut self,
        name: N,
        ip: I,
        port: u16,
        quorum: usize,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<CommandArg>,
        I: Into<CommandArg>,
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
    /// so it will no longer listed by [`sentinel_masters`](crate::SentinelCommands::sentinel_masters) and so forth.
    #[must_use]
    fn sentinel_remove<N>(&mut self, name: N) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<CommandArg>,
    {
        prepare_command(self, cmd("SENTINEL").arg("REMOVE").arg(name))
    }

    /// The SET command is very similar to the [`config_set`](crate::ServerCommands::config_set) command of Redis,
    /// and is used in order to change configuration parameters of a specific master.
    ///
    /// Multiple option / value pairs can be specified (or none at all).
    /// All the configuration parameters that can be configured via `sentinel.conf`
    /// are also configurable using this command.
    #[must_use]
    fn sentinel_set<N, O, V, C>(&mut self, name: N, configs: C) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<CommandArg>,
        O: Into<CommandArg>,
        V: Into<CommandArg>,
        C: KeyValueArgOrCollection<O, V>,
    {
        prepare_command(self, cmd("SENTINEL").arg("SET").arg(name).arg(configs))
    }

    /// Return the ID of the Sentinel instance.
    #[must_use]
    fn sentinel_myid(&mut self) -> PreparedCommand<Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("MYID"))
    }

    /// This command returns information about pending scripts.
    #[must_use]
    fn sentinel_pending_scripts(&mut self) -> PreparedCommand<Self, Vec<Value>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("PENDING-SCRIPTS"))
    }

    /// Show a list of replicas for this master, and their state.
    #[must_use]
    fn sentinel_replicas<N>(
        &mut self,
        master_name: N,
    ) -> PreparedCommand<Self, Vec<SentinelReplicaInfo>>
    where
        Self: Sized,
        N: Into<CommandArg>,
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
    fn sentinel_reset<P>(&mut self, pattern: P) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        P: Into<CommandArg>,
    {
        prepare_command(self, cmd("SENTINEL").arg("RESET").arg(pattern))
    }

    ///  Show a list of sentinel instances for this master, and their state.
    #[must_use]
    fn sentinel_sentinels<N>(&mut self, master_name: N) -> PreparedCommand<Self, Vec<SentinelInfo>>
    where
        Self: Sized,
        N: Into<CommandArg>,
    {
        prepare_command(self, cmd("SENTINEL").arg("SENTINELS").arg(master_name))
    }

    ///  This command simulates different Sentinel crash scenarios.
    #[must_use]
    fn sentinel_simulate_failure(
        &mut self,
        mode: SentinelSimulateFailureMode,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("SENTINEL").arg("SIMULATE-FAILURE").arg(mode))
    }
}

/// Result for the [`sentinel_master`](SentinelCommands::sentinel_master) command.
#[derive(Debug)]
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

impl FromValue for SentinelMasterInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            name: values.remove_or_default("name").into()?,
            ip: values.remove_or_default("ip").into()?,
            port: values.remove_or_default("port").into()?,
            runid: values.remove_or_default("runid").into()?,
            flags: values.remove_or_default("flags").into()?,
            link_pending_commands: values.remove_or_default("link-pending-commands").into()?,
            link_refcount: values.remove_or_default("link-refcount").into()?,
            last_ping_sent: values.remove_or_default("last-ping-sent").into()?,
            last_ok_ping_reply: values.remove_or_default("last-ok-ping-reply").into()?,
            last_ping_reply: values.remove_or_default("last-ping-reply").into()?,
            down_after_milliseconds: values.remove_or_default("down-after-milliseconds").into()?,
            info_refresh: values.remove_or_default("info-refresh").into()?,
            role_reported: values.remove_or_default("role-reported").into()?,
            role_reported_time: values.remove_or_default("role-reported-time").into()?,
            config_epoch: values.remove_or_default("config-epoch").into()?,
            num_slaves: values.remove_or_default("num-slaves").into()?,
            num_other_sentinels: values.remove_or_default("num-other-sentinels").into()?,
            quorum: values.remove_or_default("quorum").into()?,
            failover_timeout: values.remove_or_default("failover-timeout").into()?,
            parallel_syncs: values.remove_or_default("parallel-syncs").into()?,
        })
    }
}

/// /// Result for the [`sentinel_replicas`](SentinelCommands::sentinel_replicas) command.
#[derive(Debug)]
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
    pub slave_replica_offset: u64,
    pub replica_announed: usize,
}

impl FromValue for SentinelReplicaInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            name: values.remove_or_default("name").into()?,
            ip: values.remove_or_default("ip").into()?,
            port: values.remove_or_default("port").into()?,
            runid: values.remove_or_default("runid").into()?,
            flags: values.remove_or_default("flags").into()?,
            link_pending_commands: values.remove_or_default("link-pending-commands").into()?,
            link_refcount: values.remove_or_default("link-refcount").into()?,
            last_ping_sent: values.remove_or_default("last-ping-sent").into()?,
            last_ok_ping_reply: values.remove_or_default("last-ok-ping-reply").into()?,
            last_ping_reply: values.remove_or_default("last-ping-reply").into()?,
            down_after_milliseconds: values.remove_or_default("down-after-milliseconds").into()?,
            info_refresh: values.remove_or_default("info-refresh").into()?,
            role_reported: values.remove_or_default("role-reported").into()?,
            role_reported_time: values.remove_or_default("role-reported-time").into()?,
            master_link_down_time: values.remove_or_default("master-link-down-time").into()?,
            master_link_status: values.remove_or_default("master-link-status").into()?,
            master_host: values.remove_or_default("master-host").into()?,
            master_port: values.remove_or_default("master-port").into()?,
            slave_priority: values.remove_or_default("slave-priority").into()?,
            slave_replica_offset: values.remove_or_default("slave-replica-offset").into()?,
            replica_announed: values.remove_or_default("replica-announed").into()?,
        })
    }
}

/// Result for the [`sentinel_sentinels`](SentinelCommands::sentinel_sentinels) command.
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

impl FromValue for SentinelInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            name: values.remove_or_default("name").into()?,
            ip: values.remove_or_default("ip").into()?,
            port: values.remove_or_default("port").into()?,
            runid: values.remove_or_default("runid").into()?,
            flags: values.remove_or_default("flags").into()?,
            link_pending_commands: values.remove_or_default("link-pending-commands").into()?,
            link_refcount: values.remove_or_default("link-refcount").into()?,
            last_ping_sent: values.remove_or_default("last-ping-sent").into()?,
            last_ok_ping_reply: values.remove_or_default("last-ok-ping-reply").into()?,
            last_ping_reply: values.remove_or_default("last-ping-reply").into()?,
            down_after_milliseconds: values.remove_or_default("down-after-milliseconds").into()?,
            last_hello_message: values.remove_or_default("last-hello-message").into()?,
            voted_leader: values.remove_or_default("voted-leader").into()?,
            voted_leader_epoch: values.remove_or_default("voted-leader-epoch").into()?,
        })
    }
}

/// Different crash simulation scenario modes for
/// the [`sentinel_simulate_failure`](crate::SentinelCommands::sentinel_simulate_failure) command
pub enum SentinelSimulateFailureMode {
    CrashAfterElection,
    CrashAfterPromotion,
}

impl IntoArgs for SentinelSimulateFailureMode {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            SentinelSimulateFailureMode::CrashAfterElection => "CRASH-AFTER-ELECTION",
            SentinelSimulateFailureMode::CrashAfterPromotion => "CRASH-AFTER-PROMOTION",
        })
    }
}
