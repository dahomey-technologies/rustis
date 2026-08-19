use crate::client::ReconnectionConfig;
use rand::{RngExt, rng};
use std::cmp;

pub(crate) struct ReconnectionState {
    config: ReconnectionConfig,
    attempts: u32,
}

impl ReconnectionState {
    pub(crate) fn new(config: ReconnectionConfig) -> Self {
        Self {
            config,
            attempts: 0,
        }
    }

    /// Reset the number of reconnection attempts.
    pub(crate) fn reset_attempts(&mut self) {
        // A custom policy is told before the counter moves: that is where an
        // adaptive one closes its circuit, and it can only relate the call to
        // the outage it just ended if the count is still the outage's.
        if let ReconnectionConfig::Custom(custom) = &self.config {
            custom.policy().reset();
        }
        self.attempts = 0;
    }

    /// Calculate the next delay, incrementing `attempts` in the process.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`incr_with_max` answered `Some` on the line above, so the attempt \
                  count is at least 1."
    )]
    pub(crate) fn next_delay(&mut self) -> Option<u64> {
        match &self.config {
            ReconnectionConfig::Constant {
                delay,
                max_attempts,
                jitter,
            } => {
                self.attempts = incr_with_max(self.attempts, *max_attempts)?;
                Some(add_jitter(u64::from(*delay), *jitter))
            }
            ReconnectionConfig::Linear {
                max_delay,
                max_attempts,
                delay,
                jitter,
            } => {
                self.attempts = incr_with_max(self.attempts, *max_attempts)?;
                let delay = u64::from(*delay).saturating_mul(u64::from(self.attempts));

                Some(add_jitter(cmp::min(u64::from(*max_delay), delay), *jitter))
            }
            ReconnectionConfig::Exponential {
                min_delay,
                max_delay,
                max_attempts,
                multiplicative_factor,
                jitter,
            } => {
                self.attempts = incr_with_max(self.attempts, *max_attempts)?;
                let delay = u64::from(*multiplicative_factor)
                    .saturating_pow(self.attempts - 1)
                    .saturating_mul(u64::from(*min_delay));

                Some(add_jitter(cmp::min(u64::from(*max_delay), delay), *jitter))
            }
            ReconnectionConfig::Custom(custom) => {
                // No `max_attempts` and no jitter: both are the policy's own
                // decision, and adding either would silently override it.
                self.attempts = self.attempts.saturating_add(1);
                custom
                    .policy()
                    .next_delay(self.attempts)
                    .map(|delay| u64::try_from(delay.as_millis()).unwrap_or(u64::MAX))
            }
        }
    }
}

fn incr_with_max(curr: u32, max: u32) -> Option<u32> {
    if max != 0 && curr >= max {
        None
    } else {
        Some(curr.saturating_add(1))
    }
}

/// Spreads a delay over `[delay, delay + jitter)`.
///
/// The caller clamps the delay to `max_delay` before this point. Clamping the
/// jittered value instead cancels the jitter once the backoff saturates, which
/// re-synchronises every client of a fleet on the same wake-up instant, exactly
/// when the outage is longest. The effective ceiling is therefore
/// `max_delay + jitter`.
fn add_jitter(delay: u64, jitter: u32) -> u64 {
    if jitter == 0 {
        delay
    } else {
        delay.saturating_add(rng().random_range(0..u64::from(jitter)))
    }
}
