use crate::{
    PubSubPush, SentinelConnection, StandaloneConnection,
    client::{Config, SentinelConfig},
    resp::{Command, RespResponse, cmd},
    sleep,
};
use std::time::Duration;
use tracing::{info, warn};

/// The Sentinel channel that names the new master of a service.
const SWITCH_MASTER_CHANNEL: &[u8] = b"+switch-master";

/// How long the watch waits before its first reattempt, and the ceiling that
/// doubling stops at.
///
/// The floor keeps a Sentinel fleet that is entirely down from being dialled in
/// a tight loop; the ceiling keeps the watch from going quiet for minutes after
/// a long outage, which is when a failover is most likely to be the reason it
/// was down.
const FIRST_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// A subscription to `+switch-master`, held on a Sentinel of its own.
///
/// Discovery answers *where the master is now*; this answers *that it changed*,
/// which is the difference between rediscovering when a write is refused and
/// rediscovering when the failover happens. The Redis client spec asks for it
/// for that reason.
///
/// The connection is the watch's own, and separate from the one commands travel
/// on: the caller's connection is on the master, and the master is not what
/// announces its own replacement. Losing it is not a client failure — the watch
/// redials the fleet behind a backoff while commands keep flowing, and the
/// `master_check_interval` poll covers the gap.
pub(crate) struct MasterWatch {
    /// The instance list, refreshed from the fleet whenever a Sentinel answers.
    sentinel_config: SentinelConfig,
    /// The credentials a Sentinel is dialled with, which are never the master's.
    probe_config: Config,
    /// The subscribed connection, `None` while the watch is between Sentinels.
    connection: Option<StandaloneConnection>,
    /// What the next reattempt waits, doubling up to [`MAX_BACKOFF`].
    backoff: Duration,
}

impl MasterWatch {
    pub(crate) fn new(sentinel_config: &SentinelConfig, config: &Config) -> Self {
        Self {
            probe_config: SentinelConnection::probe_config(sentinel_config, config),
            sentinel_config: sentinel_config.clone(),
            connection: None,
            backoff: Duration::ZERO,
        }
    }

    /// Resolves once a Sentinel announces that this service switched master.
    ///
    /// Everything else — another service's failover, the subscription
    /// confirmation, a Sentinel that dies — is handled without resolving, so the
    /// caller's `select!` branch fires on the event and on nothing else.
    ///
    /// Cancel-safe: every piece of state lives in `self`, so a branch that loses
    /// the race is re-entered on the same connection rather than redialling.
    pub(crate) async fn switched(&mut self) {
        loop {
            let Some(connection) = &mut self.connection else {
                sleep(self.backoff).await;
                self.subscribe_to_a_sentinel().await;
                continue;
            };

            match connection.read().await {
                Some(Ok(response)) => {
                    if announces_switch(&response, &self.sentinel_config.service_name) {
                        return;
                    }
                }
                Some(Err(e)) => {
                    info!("The `+switch-master` subscription failed to read: {e}");
                    self.lose_connection();
                }
                None => {
                    info!("The Sentinel holding the `+switch-master` subscription closed it");
                    self.lose_connection();
                }
            }
        }
    }

    /// Drops the subscription and arms the backoff the next attempt waits.
    fn lose_connection(&mut self) {
        self.connection = None;
        self.backoff = next_backoff(self.backoff);
    }

    /// Subscribes on the first Sentinel that accepts it, leaving the watch
    /// unconnected — and its backoff grown — when none does.
    async fn subscribe_to_a_sentinel(&mut self) {
        let instances = self.sentinel_config.instances.clone();

        for (host, port) in &instances {
            let mut connection = match StandaloneConnection::connect_control(
                host,
                *port,
                &self.probe_config,
            )
            .await
            {
                Ok(connection) => connection,
                Err(e) => {
                    info!("Cannot connect to Sentinel {host}:{port} to watch it: {e}");
                    continue;
                }
            };

            // The fleet is learned before the subscription, not after: once the
            // connection is subscribed the only frames read off it are pushes,
            // and a reply threaded between them would be read as one. The list
            // this walked names the Sentinels that existed when the config was
            // written, and refreshing it is what lets the watch survive a fleet
            // that was replaced under it.
            SentinelConnection::learn_fleet(
                &mut connection,
                &mut self.sentinel_config,
                (host, *port),
            )
            .await;

            let subscribe = Command::from(cmd("SUBSCRIBE").arg(SWITCH_MASTER_CHANNEL));
            if let Err(e) = connection
                .feed(&subscribe, &[])
                .await
                .and(connection.flush().await)
            {
                info!("Cannot subscribe to `+switch-master` on {host}:{port}: {e}");
                continue;
            }
            // The confirmation is read here rather than left to `switched`, so a
            // Sentinel that accepts the socket and refuses the subscription is
            // rejected now instead of counting as a live watch.
            match connection.read().await {
                Some(Ok(response)) if !response.is_error() => (),
                _ => {
                    info!("Sentinel {host}:{port} refused the `+switch-master` subscription");
                    continue;
                }
            }

            info!("Watching `+switch-master` on Sentinel {host}:{port}");
            self.connection = Some(connection);
            self.backoff = Duration::ZERO;
            return;
        }

        // Losing every Sentinel is what turns the subscription off, and the
        // `master_check_interval` poll is all that is left watching. Warned once,
        // on the way in: the retry runs at most `MAX_BACKOFF` apart and would
        // otherwise repeat this line for as long as the fleet is down.
        if self.backoff.is_zero() {
            warn!(
                "No Sentinel accepted the `+switch-master` subscription for `{}`",
                self.sentinel_config.service_name
            );
        }
        self.backoff = next_backoff(self.backoff);
    }
}

/// Whether this reply is a `+switch-master` announcement for `service`.
///
/// A Sentinel publishes one message per service it monitors, so the payload's
/// first field — the service name — is what says whether this failover is ours.
fn announces_switch(response: &RespResponse, service: &str) -> bool {
    match PubSubPush::try_from(response) {
        Ok(PubSubPush::Message(channel, payload)) => {
            channel == SWITCH_MASTER_CHANNEL && names_service(payload, service)
        }
        _ => false,
    }
}

/// Whether a `+switch-master` payload — `<name> <old-ip> <old-port> <new-ip>
/// <new-port>` — is about `service`.
///
/// Only the name is read. The addresses it carries are deliberately ignored:
/// acting on them would trust a message to say where to reconnect, where a
/// rediscovery polls the fleet and accepts a node only once `ROLE` confirms it
/// is the master.
fn names_service(payload: &[u8], service: &str) -> bool {
    payload.split(|byte| *byte == b' ').next() == Some(service.as_bytes())
}

/// The delay after a failed attempt: the floor first, then doubling to the cap.
fn next_backoff(current: Duration) -> Duration {
    if current.is_zero() {
        FIRST_BACKOFF
    } else {
        current.saturating_mul(2).min(MAX_BACKOFF)
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
    use super::{Duration, FIRST_BACKOFF, MAX_BACKOFF, names_service, next_backoff};

    #[test]
    fn an_announcement_for_this_service_is_ours() {
        assert!(names_service(
            b"mymaster 127.0.0.1 6379 127.0.0.1 6380",
            "mymaster"
        ));
    }

    #[test]
    fn an_announcement_for_another_service_is_not() {
        // One Sentinel monitors several services and publishes every failover on
        // the same channel, so the name is the only thing separating them.
        assert!(!names_service(
            b"othermaster 127.0.0.1 6379 127.0.0.1 6380",
            "mymaster"
        ));
    }

    #[test]
    fn a_service_name_is_matched_whole() {
        // A prefix must not pass: `my` and `mymaster` are different services.
        assert!(!names_service(
            b"mymaster2 127.0.0.1 6379 127.0.0.1 6380",
            "mymaster"
        ));
        assert!(!names_service(
            b"mymaster 127.0.0.1 6379 127.0.0.1 6380",
            "mymaster2"
        ));
    }

    #[test]
    fn an_empty_payload_announces_nothing() {
        assert!(!names_service(b"", "mymaster"));
    }

    #[test]
    fn the_first_reattempt_waits_the_floor_rather_than_nothing() {
        assert_eq!(FIRST_BACKOFF, next_backoff(Duration::ZERO));
    }

    #[test]
    fn a_repeated_failure_doubles_its_wait_up_to_the_cap() {
        assert_eq!(FIRST_BACKOFF * 2, next_backoff(FIRST_BACKOFF));
        assert_eq!(MAX_BACKOFF, next_backoff(MAX_BACKOFF));
        assert_eq!(MAX_BACKOFF, next_backoff(Duration::from_secs(3)));
    }
}
