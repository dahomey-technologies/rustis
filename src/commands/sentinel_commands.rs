use crate::{
    prepare_command,
    resp::{cmd, BulkString},
    PreparedCommand,
};

/// A group of Redis commands related to Sentinel
/// # See Also
/// [Sentinel Commands](https://redis.io/docs/manual/sentinel/#sentinel-commands)
pub trait SentinelCommands {
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
        N: Into<BulkString>,
    {
        prepare_command(
            self,
            cmd("SENTINEL")
                .arg("GET-MASTER-ADDR-BY-NAME")
                .arg(master_name),
        )
    }

    /// Force a failover as if the master was not reachable,
    /// and without asking for agreement to other Sentinels
    /// (however a new version of the configuration will be published
    /// so that the other Sentinels will update their configurations).
    #[must_use]
    fn sentinel_failover<N>(&mut self, master_name: N) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        N: Into<BulkString>,
    {
        prepare_command(self, cmd("SENTINEL").arg("FAILOVER").arg(master_name))
    }
}
