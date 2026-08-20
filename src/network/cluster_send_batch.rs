//! What a batch of commands owes only once, however many of them ask for it.
//!
//! A retried command carries its retry reasons, and the handler re-feeds the
//! whole batch with them: fifty commands redirected by the same resharding
//! arrive each carrying the same `MOVED`. The two answers to that — reload the
//! topology, wait out a transient error — are answers for the batch, not for the
//! command. Paying them per command would reload the map fifty times and sleep
//! fifty delays, spending the messages' attempts on waiting.
//!
//! Each is therefore claimed rather than checked: the flag moves in the same
//! step that grants the turn, so a caller cannot take the turn without marking
//! it taken. `flush` ends the batch and hands both turns back.

/// The once-per-batch turns, both available.
#[derive(Debug, Default)]
pub(super) struct SendBatch {
    topology_refresh: Turn,
    transient_delay: Turn,
}

impl SendBatch {
    /// Whether this batch may still reload the topology. Answers `true` once.
    pub(super) fn claim_topology_refresh(&mut self) -> bool {
        self.topology_refresh.claim()
    }

    /// Whether this batch may still wait out a transient cluster error. Answers
    /// `true` once.
    pub(super) fn claim_transient_delay(&mut self) -> bool {
        self.transient_delay.claim()
    }

    /// Ends the batch: the next one owes both again.
    pub(super) fn end(&mut self) {
        *self = Self::default();
    }
}

/// Something owed once, granted to the first caller that asks.
#[derive(Debug, Default)]
struct Turn {
    taken: bool,
}

impl Turn {
    fn claim(&mut self) -> bool {
        !std::mem::replace(&mut self.taken, true)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::SendBatch;

    /// Every command of a retried batch carries the same reasons, so each asks.
    /// Only the first is served: one reload covers the whole batch, and one
    /// delay is what the cluster asked for.
    #[test]
    fn a_turn_is_granted_once_per_batch() {
        let mut batch = SendBatch::default();

        assert!(batch.claim_topology_refresh());
        assert!(!batch.claim_topology_refresh());
        assert!(!batch.claim_topology_refresh());

        assert!(batch.claim_transient_delay());
        assert!(!batch.claim_transient_delay());
    }

    /// The two turns are owed separately: a batch that reloaded the topology
    /// must still be able to wait out a transient error, and the other way
    /// round.
    #[test]
    fn the_two_turns_do_not_consume_each_other() {
        let mut batch = SendBatch::default();

        assert!(batch.claim_topology_refresh());
        assert!(
            batch.claim_transient_delay(),
            "a reload must not spend the delay"
        );

        let mut batch = SendBatch::default();
        assert!(batch.claim_transient_delay());
        assert!(
            batch.claim_topology_refresh(),
            "a delay must not spend the reload"
        );
    }

    /// A new batch owes both again — otherwise a topology that moved after the
    /// first batch would never be reloaded.
    #[test]
    fn ending_the_batch_hands_both_turns_back() {
        let mut batch = SendBatch::default();
        assert!(batch.claim_topology_refresh());
        assert!(batch.claim_transient_delay());

        batch.end();

        assert!(batch.claim_topology_refresh());
        assert!(batch.claim_transient_delay());
    }
}
