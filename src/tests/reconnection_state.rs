use crate::{client::ReconnectionConfig, network::ReconnectionState};
use std::collections::HashSet;

/// Drives `attempts` delays and returns the last `sample` of them.
fn tail_delays(config: ReconnectionConfig, attempts: usize, sample: usize) -> Vec<u64> {
    let mut state = ReconnectionState::new(config);
    let mut delays = Vec::with_capacity(attempts);
    for _ in 0..attempts {
        delays.push(state.next_delay().expect("the policy retries forever"));
    }
    delays.split_off(attempts - sample)
}

fn distinct(delays: &[u64]) -> usize {
    delays.iter().copied().collect::<HashSet<u64>>().len()
}

#[test]
fn jitter_survives_exponential_saturation() {
    // 100 * 2^9 = 51_200 ms, far past the 1_000 ms ceiling.
    let delays = tail_delays(
        ReconnectionConfig::Exponential {
            max_attempts: 0,
            min_delay: 100,
            max_delay: 1_000,
            multiplicative_factor: 2,
            jitter: 100,
        },
        200,
        100,
    );

    assert!(
        distinct(&delays) > 1,
        "a saturated policy must still spread the herd, got {delays:?}"
    );
    for delay in delays {
        assert!(
            (1_000..1_100).contains(&delay),
            "{delay} is outside the ceiling plus the jitter window"
        );
    }
}

#[test]
fn jitter_survives_linear_saturation() {
    let delays = tail_delays(
        ReconnectionConfig::Linear {
            max_attempts: 0,
            max_delay: 1_000,
            delay: 100,
            jitter: 100,
        },
        200,
        100,
    );

    assert!(
        distinct(&delays) > 1,
        "a saturated policy must still spread the herd, got {delays:?}"
    );
    for delay in delays {
        assert!((1_000..1_100).contains(&delay), "{delay} is out of range");
    }
}

#[test]
fn a_zero_jitter_gives_the_exact_delay() {
    let delays = tail_delays(
        ReconnectionConfig::Constant {
            max_attempts: 0,
            delay: 1_000,
            jitter: 0,
        },
        10,
        10,
    );

    assert_eq!(vec![1_000; 10], delays);
}

#[test]
fn the_ceiling_bounds_the_delay_before_the_jitter() {
    let delays = tail_delays(
        ReconnectionConfig::Exponential {
            max_attempts: 0,
            min_delay: 100,
            max_delay: 1_000,
            multiplicative_factor: 2,
            jitter: 0,
        },
        20,
        5,
    );

    assert_eq!(vec![1_000; 5], delays);
}

/// A policy the three built-in shapes cannot express: it opens a circuit after
/// three failures and reads a signal only its owner has.
#[test]
fn a_custom_policy_decides_the_delay_and_when_to_give_up() {
    use crate::client::{CustomReconnectionPolicy, ReconnectionPolicy};
    use std::sync::{
        Arc,
        atomic::{AtomicU32, AtomicUsize, Ordering},
    };

    struct CircuitBreaker {
        healthy: Arc<AtomicU32>,
        resets: Arc<AtomicUsize>,
    }

    impl ReconnectionPolicy for CircuitBreaker {
        fn next_delay(&self, attempt: u32) -> Option<std::time::Duration> {
            if self.healthy.load(Ordering::Relaxed) == 0 {
                // The circuit is open: stop reconnecting rather than hammer a
                // backend an external signal says is down.
                return None;
            }
            Some(std::time::Duration::from_millis(u64::from(attempt) * 7))
        }

        fn reset(&self) {
            self.resets.fetch_add(1, Ordering::Relaxed);
        }
    }

    let healthy = Arc::new(AtomicU32::new(1));
    let resets = Arc::new(AtomicUsize::new(0));
    let mut state = ReconnectionState::new(ReconnectionConfig::Custom(
        CustomReconnectionPolicy::new(CircuitBreaker {
            healthy: Arc::clone(&healthy),
            resets: Arc::clone(&resets),
        }),
    ));

    // The attempt number is handed in, so a stateless policy stays stateless.
    assert_eq!(Some(7), state.next_delay());
    assert_eq!(Some(14), state.next_delay());

    // A successful reconnection tells the policy, which is what lets an adaptive
    // one close its own circuit.
    state.reset_attempts();
    assert_eq!(1, resets.load(Ordering::Relaxed));
    assert_eq!(Some(7), state.next_delay());

    healthy.store(0, Ordering::Relaxed);
    assert_eq!(None, state.next_delay(), "an open circuit gives up");
}

/// A closure is enough for a policy that only shapes the delay.
#[test]
fn a_closure_is_a_reconnection_policy() {
    use crate::client::CustomReconnectionPolicy;
    use std::time::Duration;

    let mut state =
        ReconnectionState::new(ReconnectionConfig::Custom(CustomReconnectionPolicy::new(
            |attempt: u32| (attempt <= 2).then(|| Duration::from_millis(100)),
        )));

    assert_eq!(Some(100), state.next_delay());
    assert_eq!(Some(100), state.next_delay());
    assert_eq!(None, state.next_delay());
}
