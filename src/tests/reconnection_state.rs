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
