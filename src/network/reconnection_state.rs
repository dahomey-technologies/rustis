use crate::client::ReconnectionConfig;
use rand::{thread_rng, Rng};
use std::cmp;

pub(crate) struct ReconnectionState {
    config: ReconnectionConfig,
    attempts: u32,
}

impl ReconnectionState {
    pub fn new(config: ReconnectionConfig) -> Self {
        Self {
            config,
            attempts: 0,
        }
    }

    /// Reset the number of reconnection attempts.
    pub fn reset_attempts(&mut self) {
        self.attempts = 0;
    }

    /// Calculate the next delay, incrementing `attempts` in the process.
    pub fn next_delay(&mut self) -> Option<u64> {
        match &self.config {
            ReconnectionConfig::Constant {
                delay,
                max_attempts,
                jitter,
            } => {
                self.attempts = match incr_with_max(self.attempts, *max_attempts) {
                    Some(a) => a,
                    None => return None,
                };

                Some(add_jitter(*delay as u64, *jitter))
            }
            ReconnectionConfig::Linear {
                max_delay,
                max_attempts,
                delay,
                jitter,
            } => {
                self.attempts = match incr_with_max(self.attempts, *max_attempts) {
                    Some(a) => a,
                    None => return None,
                };
                let delay = (*delay as u64).saturating_mul(self.attempts as u64);

                Some(cmp::min(*max_delay as u64, add_jitter(delay, *jitter)))
            }
            ReconnectionConfig::Exponential {
                min_delay,
                max_delay,
                max_attempts,
                multiplicative_factor,
                jitter,
            } => {
                self.attempts = match incr_with_max(self.attempts, *max_attempts) {
                    Some(a) => a,
                    None => return None,
                };
                let delay = (*multiplicative_factor as u64)
                    .saturating_pow(self.attempts - 1)
                    .saturating_mul(*min_delay as u64);

                Some(cmp::min(*max_delay as u64, add_jitter(delay, *jitter)))
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

fn add_jitter(delay: u64, jitter: u32) -> u64 {
    if jitter == 0 {
        delay
    } else {
        delay.saturating_add(thread_rng().gen_range(0..jitter as u64))
    }
}
