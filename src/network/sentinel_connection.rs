use crate::{
    Error, Result, RetryReason, StandaloneConnection,
    client::{Config, SentinelConfig},
    commands::{RoleResult, SentinelCommands, ServerCommands},
    resp::{Command, RespResponse},
    sleep,
};
use log::debug;
use std::{sync::Arc, task::Poll};

pub struct SentinelConnection {
    sentinel_config: SentinelConfig,
    config: Config,
    pub inner_connection: StandaloneConnection,
}

impl SentinelConnection {
    #[inline]
    pub async fn feed(&mut self, command: &Command, retry_reasons: &[RetryReason]) -> Result<()> {
        self.inner_connection.feed(command, retry_reasons).await
    }

    #[inline]
    pub async fn flush(&mut self) -> Result<()> {
        self.inner_connection.flush().await
    }

    #[inline]
    pub async fn read(&mut self) -> Option<Result<RespResponse>> {
        self.inner_connection.read().await
    }

    #[inline]
    pub fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        self.inner_connection.try_read()
    }

    #[inline]
    pub async fn reconnect(&mut self) -> Result<()> {
        self.inner_connection =
            Self::connect_to_sentinel(&self.sentinel_config, &self.config).await?;

        Ok(())
    }

    /// Follow `Redis service discovery via Sentinel` documentation
    /// #See <https://redis.io/docs/reference/sentinel-clients/#redis-service-discovery-via-sentinel>
    ///
    /// # Remark
    /// this function must be desugared because of async recursion:
    /// <https://doc.rust-lang.org/error-index.html#E0733>
    pub async fn connect(
        sentinel_config: &SentinelConfig,
        config: &Config,
    ) -> Result<SentinelConnection> {
        let inner_connection = Self::connect_to_sentinel(sentinel_config, config).await?;

        Ok(SentinelConnection {
            sentinel_config: sentinel_config.clone(),
            config: config.clone(),
            inner_connection,
        })
    }

    async fn connect_to_sentinel(
        sentinel_config: &SentinelConfig,
        config: &Config,
    ) -> Result<StandaloneConnection> {
        let mut restart = false;
        let mut unreachable_sentinel = true;
        // A step-3 failure (master connect / ROLE) is a distinct outcome from a
        // sentinel being unreachable: it means we did learn a master address but
        // could not use it, typically mid-failover.
        let mut master_unreachable = false;

        let mut sentinel_node_config = config.clone();
        sentinel_node_config
            .username
            .clone_from(&sentinel_config.username);
        sentinel_node_config
            .password
            .clone_from(&sentinel_config.password);

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

            for sentinel_instance in &sentinel_config.instances {
                // Step 1: connecting to Sentinel
                let (host, port) = sentinel_instance;

                let mut sentinel_connection =
                    match StandaloneConnection::connect(host, *port, &sentinel_node_config).await {
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
                let mut master_connection =
                    match StandaloneConnection::connect(&master_host, master_port, config).await {
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

    pub(crate) fn tag(&self) -> Arc<str> {
        self.inner_connection.tag()
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
            DiscoveryOutcome::AllUnreachable => {
                Error::Sentinel("All Sentinel instances are unreachable".to_owned())
            }
            DiscoveryOutcome::MasterUnknown => Error::Sentinel(format!(
                "master {service_name} is unknown by all Sentinel instances"
            )),
            DiscoveryOutcome::MasterUnreachable => Error::Sentinel(format!(
                "master {service_name} could not be reached through any Sentinel"
            )),
            DiscoveryOutcome::RoundsExhausted => Error::Sentinel(format!(
                "master {service_name} did not stabilize after {max_discovery_rounds} discovery rounds"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
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
