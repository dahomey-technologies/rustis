use crate::{
    ConnectionState, Error, ErrorKind, Result, RetryReason, StandaloneConnection,
    client::{Config, SentinelConfig},
    commands::{RoleResult, SentinelCommands, SentinelInfo, ServerCommands},
    deadline_after,
    resp::{Command, RespResponse},
    sleep,
};
use std::{sync::Arc, task::Poll};
use tokio::time::Instant;
use tracing::{debug, info, warn};

pub(crate) struct SentinelConnection {
    sentinel_config: SentinelConfig,
    config: Config,
    pub inner_connection: StandaloneConnection,
    /// When the master address is next polled from the Sentinels, `None` when
    /// polling is off.
    next_master_check: Option<Instant>,
}

impl SentinelConnection {
    #[inline]
    pub(crate) async fn feed(
        &mut self,
        command: &Command,
        retry_reasons: &[RetryReason],
    ) -> Result<()> {
        self.inner_connection.feed(command, retry_reasons).await
    }

    #[inline]
    pub(crate) async fn flush(&mut self) -> Result<()> {
        self.inner_connection.flush().await
    }

    #[inline]
    pub(crate) async fn read(&mut self) -> Option<Result<RespResponse>> {
        self.inner_connection.read().await
    }

    #[inline]
    pub(crate) fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        self.inner_connection.try_read()
    }

    #[inline]
    pub(crate) async fn reconnect(&mut self, connection_state: &mut ConnectionState) -> Result<()> {
        self.inner_connection =
            Self::connect_to_sentinel(&mut self.sentinel_config, &self.config, connection_state)
                .await?;

        Ok(())
    }

    /// When the master address is next polled, `None` when polling is off.
    #[inline]
    pub(crate) fn next_maintenance(&self) -> Option<Instant> {
        self.next_master_check
    }

    /// Polls the Sentinels for the master address and reports whether it moved.
    ///
    /// This is the net under the `+switch-master` subscription: an announcement
    /// published while that subscription was itself redialling is gone, and
    /// nothing else would notice the new master until a write came back
    /// `READONLY`. Reconnecting is the caller's to do — swapping the socket from
    /// here would drop the requests in flight on it.
    ///
    /// A round that reaches no Sentinel reports no move: the master this
    /// connection is on is still the best thing known about the service, and
    /// churning the connection over an unreachable fleet would only lose it.
    /// That outcome is warned about rather than logged at `debug!` — it is the
    /// state where the failover safety net is off, and it is indistinguishable
    /// from a healthy round in the value returned.
    pub(crate) async fn run_maintenance(&mut self) -> bool {
        self.next_master_check = self
            .sentinel_config
            .master_check_interval
            .map(deadline_after);

        self.master_moved().await
    }

    /// Whether the Sentinels announce a master this connection is not on.
    ///
    /// The comparison is against the address actually connected, not against a
    /// remembered announcement: a rediscovery the subscription already triggered
    /// leaves this finding nothing to do, which is what keeps one failover from
    /// costing two.
    async fn master_moved(&mut self) -> bool {
        let Some((current_host, current_port)) = self.inner_connection.tcp_address() else {
            return false;
        };
        let (current_host, current_port) = (current_host.to_owned(), current_port);

        let probe_config = Self::probe_config(&self.sentinel_config, &self.config);
        let instances = self.sentinel_config.instances.clone();
        // Which of the two silent outcomes this round ended on, so the warning
        // below names the one that happened. The same distinction
        // `DiscoveryOutcome` makes: a fleet that is down and a fleet that is up
        // and unhelpful call for opposite things from whoever reads the log.
        let mut reached_a_sentinel = false;

        for (host, port) in &instances {
            let mut sentinel_connection =
                match StandaloneConnection::connect_control(host, *port, &probe_config).await {
                    Ok(sentinel_connection) => sentinel_connection,
                    Err(e) => {
                        info!("Cannot reach Sentinel {host}:{port} to check the master: {e}");
                        continue;
                    }
                };

            match sentinel_connection
                .sentinel_get_master_addr_by_name(self.sentinel_config.service_name.clone())
                .await
            {
                Ok(Some((master_host, master_port))) => {
                    let moved = master_host != current_host || master_port != current_port;
                    if moved {
                        info!(
                            "Sentinel {host}:{port} announces master {master_host}:{master_port}, \
                             this connection is on {current_host}:{current_port}"
                        );
                    }
                    return moved;
                }
                Ok(None) => {
                    reached_a_sentinel = true;
                    continue;
                }
                Err(e) => {
                    // Answered, and refused: an ACL or a handshake, not a node
                    // that is down. It is counted as reached so the round says so.
                    reached_a_sentinel = true;
                    info!("Cannot check the master with Sentinel {host}:{port}: {e}");
                    continue;
                }
            }
        }

        // The poll ran and learned nothing, so the net under the subscription is
        // off. Which of the two reasons it is decides what is to be done about
        // it, so it is what the line says.
        let service_name = &self.sentinel_config.service_name;
        if reached_a_sentinel {
            warn!("No Sentinel knows the master of `{service_name}`");
        } else {
            warn!("No Sentinel could be reached to check the master of `{service_name}`");
        }
        false
    }

    /// The Sentinel instances this connection knows, the one that last answered
    /// first.
    #[cfg(test)]
    pub(crate) fn known_instances(&self) -> &[(String, u16)] {
        &self.sentinel_config.instances
    }

    /// Follow `Redis service discovery via Sentinel` documentation
    /// #See <https://redis.io/docs/reference/sentinel-clients/#redis-service-discovery-via-sentinel>
    ///
    /// # Remark
    /// this function must be desugared because of async recursion:
    /// <https://doc.rust-lang.org/error-index.html#E0733>
    pub(crate) async fn connect(
        sentinel_config: &SentinelConfig,
        config: &Config,
        connection_state: &mut ConnectionState,
    ) -> Result<SentinelConnection> {
        // The discovery updates the instance list, so it runs against the copy
        // this connection keeps rather than against the caller's configuration.
        let mut sentinel_config = sentinel_config.clone();
        let inner_connection =
            Self::connect_to_sentinel(&mut sentinel_config, config, connection_state).await?;

        Ok(SentinelConnection {
            next_master_check: sentinel_config.master_check_interval.map(deadline_after),
            sentinel_config,
            config: config.clone(),
            inner_connection,
        })
    }

    /// The config a probe to a Sentinel instance connects with.
    ///
    /// A Sentinel is a different server with its own ACLs, so it gets the
    /// Sentinel credentials — static or from its own provider — and never the
    /// master's.
    pub(crate) fn probe_config(sentinel_config: &SentinelConfig, config: &Config) -> Config {
        let mut probe_config = config.clone();
        probe_config.username.clone_from(&sentinel_config.username);
        probe_config.password.clone_from(&sentinel_config.password);
        probe_config
            .credentials_provider
            .clone_from(&sentinel_config.credentials_provider);
        probe_config
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the loop returns unless the round counter is still below \
                  `max_discovery_rounds`, so the increment is bounded by it."
    )]
    async fn connect_to_sentinel(
        sentinel_config: &mut SentinelConfig,
        config: &Config,
        connection_state: &mut ConnectionState,
    ) -> Result<StandaloneConnection> {
        let mut restart = false;
        let mut unreachable_sentinel = true;
        // A step-3 failure (master connect / ROLE) is a distinct outcome from a
        // sentinel being unreachable: it means we did learn a master address but
        // could not use it, typically mid-failover.
        let mut master_unreachable = false;

        let sentinel_node_config = Self::probe_config(sentinel_config, config);

        let mut rounds = 0;
        loop {
            // Bound the restart loop: a stale Sentinel stuck announcing a
            // non-master would otherwise spin forever.
            if rounds >= sentinel_config.max_discovery_rounds {
                return Err(DiscoveryOutcome::RoundsExhausted.into_error(
                    &sentinel_config.service_name,
                    sentinel_config.max_discovery_rounds,
                ));
            }
            rounds += 1;

            // The list grows while it is walked, so the walk is by index over a
            // snapshot of the instances known at the start of this round.
            let instances = sentinel_config.instances.clone();
            for sentinel_instance in &instances {
                // Step 1: connecting to Sentinel
                let (host, port) = sentinel_instance;

                // A probe to a Sentinel is a control connection, not the caller's:
                // it must not replay their database, name or tracking mode onto a
                // node they never addressed.
                let mut sentinel_connection =
                    match StandaloneConnection::connect_control(host, *port, &sentinel_node_config)
                        .await
                    {
                        Ok(sentinel_connection) => sentinel_connection,
                        Err(e) => {
                            debug!("Cannot connect to Sentinel {}:{} : {}", *host, *port, e);
                            continue;
                        }
                    };

                // Step 2: ask for master address
                let (master_host, master_port) = match sentinel_connection
                    .sentinel_get_master_addr_by_name(sentinel_config.service_name.clone())
                    .await
                {
                    Ok(Some((master_host, master_port))) => (master_host, master_port),
                    Ok(None) => {
                        debug!(
                            "Sentinel {}:{} does not know master `{}`",
                            *host, *port, sentinel_config.service_name
                        );
                        unreachable_sentinel = false;
                        continue;
                    }
                    Err(e) => {
                        debug!(
                            "Cannot execute command `SENTINEL get-master-addr-by-name` with Sentinel {}:{}: {}",
                            *host, *port, e
                        );
                        continue;
                    }
                };

                // This Sentinel answered, so it is not the source of any later
                // failure; a master address is now known.
                unreachable_sentinel = false;

                // Step 3: call the ROLE command in the target instance. An
                // unreachable announced master is exactly the failover scenario
                // Sentinel exists for, so fall through to the next Sentinel — which
                // may know the newly promoted master — instead of aborting.
                let mut master_connection = match StandaloneConnection::connect(
                    &master_host,
                    master_port,
                    config,
                    connection_state,
                )
                .await
                {
                    Ok(connection) => connection,
                    Err(e) => {
                        debug!("Cannot connect to master {master_host}:{master_port}: {e}");
                        master_unreachable = true;
                        continue;
                    }
                };

                let role: RoleResult = match master_connection.role().await {
                    Ok(role) => role,
                    Err(e) => {
                        debug!("Cannot execute command `ROLE` on {master_host}:{master_port}: {e}");
                        master_unreachable = true;
                        continue;
                    }
                };

                if let RoleResult::Master {
                    master_replication_offset: _,
                    replica_infos: _,
                } = role
                {
                    // The fleet is learned only once a master is confirmed, so a
                    // failing discovery does not pay for the extra round trip.
                    Self::learn_fleet(&mut sentinel_connection, sentinel_config, (host, *port))
                        .await;

                    return Ok(master_connection);
                } else {
                    sleep(sentinel_config.wait_between_failures).await;
                    // restart from the beginning
                    restart = true;
                    break;
                }
            }

            if !restart {
                break;
            } else {
                restart = false;
            }
        }

        let outcome = if unreachable_sentinel {
            DiscoveryOutcome::AllUnreachable
        } else if master_unreachable {
            DiscoveryOutcome::MasterUnreachable
        } else {
            DiscoveryOutcome::MasterUnknown
        };
        Err(outcome.into_error(
            &sentinel_config.service_name,
            sentinel_config.max_discovery_rounds,
        ))
    }

    pub(crate) fn get_version(&self) -> &str {
        self.inner_connection.get_version()
    }

    pub(crate) fn tag(&self) -> Arc<str> {
        self.inner_connection.tag()
    }

    /// Refreshes the instance list from the Sentinel that just answered.
    ///
    /// A configuration names the Sentinels that existed when it was written. The
    /// client spec requires the list to be maintained from the fleet itself,
    /// otherwise replacing every named instance — a redeployment, a scale-out —
    /// leaves the client with nothing reachable and no way to learn better.
    ///
    /// The answering instance is moved to the front, so the next discovery starts
    /// on the one known to work instead of walking the dead ones again.
    ///
    /// A failure here is not a connection failure: the master is already
    /// confirmed. The list simply stays as it was.
    pub(crate) async fn learn_fleet(
        sentinel_connection: &mut StandaloneConnection,
        sentinel_config: &mut SentinelConfig,
        answered: (&String, u16),
    ) {
        let peers: Vec<SentinelInfo> = match sentinel_connection
            .sentinel_sentinels(sentinel_config.service_name.clone())
            .await
        {
            Ok(peers) => peers,
            Err(e) => {
                debug!("Cannot refresh the Sentinel instance list: {e}");
                return;
            }
        };

        let (answered_host, answered_port) = answered;
        let mut instances = vec![(answered_host.clone(), answered_port)];
        instances.extend(
            sentinel_config
                .instances
                .drain(..)
                .filter(|instance| instance != &(answered_host.clone(), answered_port)),
        );

        for peer in peers {
            let instance = (peer.ip, peer.port);
            if !instances.contains(&instance) {
                debug!("Learned Sentinel {}:{}", instance.0, instance.1);
                instances.push(instance);
            }
        }

        sentinel_config.instances = instances;
    }
}

/// Why sentinel discovery exhausted every instance, used to pick an accurate
/// error. Split out from the I/O loop so the message selection is unit-testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveryOutcome {
    /// No Sentinel could be reached at all.
    AllUnreachable,
    /// Sentinels were reached but none knew the requested master.
    MasterUnknown,
    /// A master address was obtained but connecting to it or confirming its role
    /// failed on every round (failover in progress, or the cap was hit).
    MasterUnreachable,
    /// The bounded restart loop hit its cap while still seeing non-master roles.
    RoundsExhausted,
}

impl DiscoveryOutcome {
    fn into_error(self, service_name: &str, max_discovery_rounds: usize) -> Error {
        match self {
            DiscoveryOutcome::AllUnreachable => Error::from(ErrorKind::Sentinel(
                "All Sentinel instances are unreachable".to_owned(),
            )),
            DiscoveryOutcome::MasterUnknown => Error::from(ErrorKind::Sentinel(format!(
                "master {service_name} is unknown by all Sentinel instances"
            ))),
            DiscoveryOutcome::MasterUnreachable => Error::from(ErrorKind::Sentinel(format!(
                "master {service_name} could not be reached through any Sentinel"
            ))),
            DiscoveryOutcome::RoundsExhausted => Error::from(ErrorKind::Sentinel(format!(
                "master {service_name} did not stabilize after {max_discovery_rounds} discovery rounds"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::DiscoveryOutcome;

    /// Deliberately not the configured default: the message must report the cap
    /// it was given, not a value baked into the format string.
    const MAX_DISCOVERY_ROUNDS: usize = 7;

    #[test]
    fn outcome_messages_are_distinct_and_named() {
        let all = DiscoveryOutcome::AllUnreachable.into_error("mymaster", MAX_DISCOVERY_ROUNDS);
        let unknown = DiscoveryOutcome::MasterUnknown.into_error("mymaster", MAX_DISCOVERY_ROUNDS);
        let unreachable =
            DiscoveryOutcome::MasterUnreachable.into_error("mymaster", MAX_DISCOVERY_ROUNDS);
        let exhausted =
            DiscoveryOutcome::RoundsExhausted.into_error("mymaster", MAX_DISCOVERY_ROUNDS);

        assert!(all.to_string().contains("unreachable"));
        // A step-3 failure must not be reported as "all Sentinels unreachable".
        assert_ne!(all.to_string(), unreachable.to_string());
        assert!(unknown.to_string().contains("unknown"));
        assert!(unreachable.to_string().contains("mymaster"));
        assert!(
            exhausted
                .to_string()
                .contains(&MAX_DISCOVERY_ROUNDS.to_string())
        );
    }
}
