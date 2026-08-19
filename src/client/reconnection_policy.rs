use std::{fmt, sync::Arc, time::Duration};

/// How long to wait before the next reconnection attempt, and when to stop
/// trying.
///
/// The three built-in shapes —
/// [`Constant`](crate::client::ReconnectionConfig::Constant),
/// [`Linear`](crate::client::ReconnectionConfig::Linear) and
/// [`Exponential`](crate::client::ReconnectionConfig::Exponential) — cover a
/// delay that depends on nothing but the attempt number. This is for a delay
/// that depends on something else: a circuit breaker, a health signal read from
/// elsewhere, a backoff coordinated across a pool, a schedule that refuses to
/// reconnect during a maintenance window.
///
/// The trait is implemented for any `Fn(u32) -> Option<Duration>`, so a closure
/// is enough when the policy only shapes the delay:
///
/// ```
/// use rustis::client::{Config, CustomReconnectionPolicy, ReconnectionConfig};
/// use std::time::Duration;
///
/// let mut config = Config::default();
/// config.reconnection = ReconnectionConfig::Custom(CustomReconnectionPolicy::new(
///     |attempt: u32| (attempt <= 10).then(|| Duration::from_millis(250)),
/// ));
/// ```
///
/// # What the client guarantees
///
/// [`next_delay`](Self::next_delay) is called once per attempt, from the network
/// task, with `attempt` counting from `1` and reset to `1` by every successful
/// reconnection. It must not block: it runs on the task that also drives the
/// connection.
///
/// Answering `None` ends the client for good — the network task stops and every
/// later command fails, which
/// [`Client::is_terminated`](crate::client::Client::is_terminated) reports. A
/// long-lived service should return a capped delay rather than `None`.
///
/// Jitter is the policy's own business here: the built-in shapes add theirs
/// because their delay is otherwise identical across a fleet, and nothing is
/// added to what this returns.
pub trait ReconnectionPolicy: Send + Sync + 'static {
    /// How long to wait before attempt number `attempt`, or `None` to stop
    /// reconnecting for good.
    fn next_delay(&self, attempt: u32) -> Option<Duration>;

    /// Called when a reconnection succeeded, before the attempt counter goes
    /// back to `1`.
    ///
    /// This is where an adaptive policy closes its circuit or clears the state
    /// it built up during the outage. The default does nothing.
    fn reset(&self) {}
}

impl<F> ReconnectionPolicy for F
where
    F: Fn(u32) -> Option<Duration> + Send + Sync + 'static,
{
    fn next_delay(&self, attempt: u32) -> Option<Duration> {
        self(attempt)
    }
}

/// A [`ReconnectionPolicy`] as held by
/// [`ReconnectionConfig::Custom`](crate::client::ReconnectionConfig::Custom).
///
/// The wrapper exists so a [`Config`](crate::client::Config) stays `Clone` and
/// `Debug`: a policy is neither, and its `Debug` says only that one is
/// injected.
#[derive(Clone)]
pub struct CustomReconnectionPolicy(Arc<dyn ReconnectionPolicy>);

impl CustomReconnectionPolicy {
    /// Wraps `policy`, which may be a closure.
    pub fn new(policy: impl ReconnectionPolicy) -> Self {
        Self(Arc::new(policy))
    }

    pub(crate) fn policy(&self) -> &Arc<dyn ReconnectionPolicy> {
        &self.0
    }
}

impl fmt::Debug for CustomReconnectionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CustomReconnectionPolicy")
    }
}
