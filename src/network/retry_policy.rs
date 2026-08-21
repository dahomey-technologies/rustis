use crate::{Error, ErrorKind, Result, client::Message, resp::RespResponse};

/// The per-message retry rule.
///
/// A message is replayed for two reasons that look nothing alike on the wire: a
/// cluster redirect the server answered with, and a connection that died under
/// it. Both end at the same three questions — does this reply mean "try again",
/// what does the next attempt have to be told, and has this message used up its
/// budget — so the answers live here rather than at each choke point.
///
/// Getting the budget wrong is what a pathological redirect loop needs to replay
/// a command forever, and getting the reasons wrong sends the retry back to the
/// node that just refused it.
pub(crate) struct RetryPolicy {
    /// Per-message attempt cap from `Config::max_command_attempts`. `0` means
    /// unlimited, which is the default and the historical behavior.
    max_command_attempts: usize,
}

impl RetryPolicy {
    pub(crate) fn new(max_command_attempts: usize) -> Self {
        Self {
            max_command_attempts,
        }
    }

    /// Whether a message attempted `attempts` times may be attempted again.
    pub(crate) fn has_budget(&self, attempts: usize) -> bool {
        self.max_command_attempts == 0 || attempts < self.max_command_attempts
    }

    /// Counts one attempt on `message` and reports whether it may still be
    /// replayed.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`attempts` counts retries of one message, bounded by the cap \
                  when there is one and by the reconnection count otherwise."
    )]
    pub(crate) fn charge_attempt(&self, message: &mut Message) -> bool {
        message.attempts += 1;
        self.has_budget(message.attempts)
    }

    /// Whether `result` sends `message` back for another attempt.
    ///
    /// A message that already carries reasons is mid-replay: the batch it
    /// belongs to asked for a retry, so it goes back whatever this particular
    /// reply says.
    pub(crate) fn asks_for_retry(&self, result: &Result<RespResponse>, message: &Message) -> bool {
        match result {
            Err(e) => matches!(e.kind(), ErrorKind::Retry(_)),
            Ok(_) => message.retry_reasons.is_some(),
        }
    }

    /// Adds the reasons `result` carries to the ones `message` already holds.
    ///
    /// The reasons accumulate rather than replace: a message redirected twice
    /// must be fed both, or the second attempt goes back to the node that
    /// redirected it first.
    pub(crate) fn absorb_reasons(&self, message: &mut Message, result: Result<RespResponse>) {
        let Err(ErrorKind::Retry(reasons)) = result.map_err(Error::into_kind) else {
            return;
        };

        match &mut message.retry_reasons {
            Some(retry_reasons) => retry_reasons.extend(reasons.into_inner()),
            None => message.retry_reasons = Some(Vec::from_iter(reasons.into_inner())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::*;
    use crate::{RetryReasons, client::Message, resp::cmd};
    use smallvec::SmallVec;

    fn message() -> Message {
        Message::single_forget(cmd("PING").into(), true)
    }

    fn redirect(slot: u16) -> Result<RespResponse> {
        let mut reasons = SmallVec::new();
        reasons.push(crate::RetryReason::Moved {
            hash_slot: slot,
            address: (String::from("127.0.0.1"), 6379),
        });
        Err(Error::from(ErrorKind::Retry(RetryReasons::new(reasons))))
    }

    /// The reasons travel inside an opaque wrapper, so what the retry path reads
    /// back has to be what the reply put in, in the same order.
    #[test]
    fn the_wrapper_hands_back_the_reasons_it_was_given() {
        let policy = RetryPolicy::new(0);
        let mut message = message();

        policy.absorb_reasons(&mut message, redirect(1));
        policy.absorb_reasons(&mut message, redirect(2));

        let slots: Vec<u16> = message
            .retry_reasons
            .unwrap()
            .iter()
            .map(|reason| match reason {
                crate::RetryReason::Moved { hash_slot, .. } => *hash_slot,
                other => panic!("unexpected reason: {other:?}"),
            })
            .collect();
        assert_eq!(vec![1, 2], slots);
    }

    #[test]
    fn a_cap_of_zero_never_runs_out() {
        let policy = RetryPolicy::new(0);
        assert!(policy.has_budget(1));
        assert!(policy.has_budget(1_000_000));
    }

    #[test]
    fn the_budget_ends_at_the_cap_not_past_it() {
        let policy = RetryPolicy::new(3);
        assert!(policy.has_budget(2));
        assert!(!policy.has_budget(3));
        assert!(!policy.has_budget(4));
    }

    #[test]
    fn charging_an_attempt_spends_the_budget_one_at_a_time() {
        let policy = RetryPolicy::new(2);
        let mut message = message();

        assert!(policy.charge_attempt(&mut message));
        assert_eq!(1, message.attempts);
        assert!(!policy.charge_attempt(&mut message));
        assert_eq!(2, message.attempts);
    }

    #[test]
    fn a_redirect_asks_for_a_retry_and_an_ordinary_error_does_not() {
        let policy = RetryPolicy::new(0);
        let message = message();

        assert!(policy.asks_for_retry(&redirect(1), &message));
        assert!(!policy.asks_for_retry(&Err(Error::from(ErrorKind::DisconnectedByPeer)), &message));
    }

    #[test]
    fn a_message_already_carrying_reasons_goes_back_whatever_the_reply_says() {
        let policy = RetryPolicy::new(0);
        let mut message = message();
        assert!(!policy.asks_for_retry(&Ok(RespResponse::Null), &message));

        policy.absorb_reasons(&mut message, redirect(1));
        assert!(policy.asks_for_retry(&Ok(RespResponse::Null), &message));
    }

    #[test]
    fn reasons_pile_up_across_redirects() {
        let policy = RetryPolicy::new(0);
        let mut message = message();

        policy.absorb_reasons(&mut message, redirect(1));
        policy.absorb_reasons(&mut message, redirect(2));

        assert_eq!(2, message.retry_reasons.as_ref().unwrap().len());
    }

    #[test]
    fn a_reply_that_asks_for_nothing_leaves_the_reasons_alone() {
        let policy = RetryPolicy::new(0);
        let mut message = message();

        policy.absorb_reasons(&mut message, redirect(1));
        policy.absorb_reasons(&mut message, Ok(RespResponse::Null));
        policy.absorb_reasons(
            &mut message,
            Err(Error::from(ErrorKind::DisconnectedByPeer)),
        );

        assert_eq!(1, message.retry_reasons.as_ref().unwrap().len());
    }
}
