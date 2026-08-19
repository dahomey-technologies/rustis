use super::network_handler::max_attempts_reached;
use crate::{
    ClientError, Error, ErrorKind,
    client::{Message, StatsRecorder},
    resp::RespResponse,
};
use std::{collections::VecDeque, sync::Arc};

/// A message waiting to be written to the socket.
pub(crate) struct MessageToSend {
    pub message: Message,
}

impl MessageToSend {
    pub(crate) fn new(message: Message) -> Self {
        Self { message }
    }
}

/// A message that has been written and is waiting for its reply.
#[derive(Debug)]
pub(crate) struct MessageToReceive {
    pub message: Message,
    pub num_commands: usize,
    pub pending_responses: Vec<RespResponse>,
    /// What this message still holds of the queue budget.
    ///
    /// It is carried rather than recomputed on removal, so the amount released
    /// is exactly the amount charged whatever the message has been through since.
    pub queued_bytes: usize,
}

impl MessageToReceive {
    pub(crate) fn new(message: Message, num_commands: usize, queued_bytes: usize) -> Self {
        Self {
            message,
            num_commands,
            // A batch collects exactly `num_commands` responses; size the buffer
            // once instead of letting it grow.
            pending_responses: Vec::with_capacity(num_commands),
            queued_bytes,
        }
    }
}

/// The two queues a connection holds, and the totals that bound them.
///
/// # Why one type for two queues
///
/// The byte budget covers both: a command is charged when it is queued and
/// released when its **reply** arrives, so a connection that answers nothing
/// stays bounded. Splitting the queues would split that single total between two
/// owners, which is what it must never be.
///
/// # Why the totals are not public fields
///
/// They are maintained incrementally — the queues are walked often enough that
/// summing them per message would be quadratic in the depth — so every push and
/// pop has to adjust them. Behind fields, that is seven call sites obeying an
/// unwritten rule, and one of them forgot: a reconnection replay rebuilt the
/// byte total and left the command total, which then counted the replayed
/// messages twice. Here the adjustment belongs to the operation, so a caller
/// cannot move a message without moving its charge.
///
/// The two units are not interchangeable. Bytes are held until the reply, and
/// commands only until the write: [`queued_commands`](Self::queued_commands)
/// answers "what is still waiting to go out", which is why it falls to zero on
/// an idle connection while the byte total may not.
pub(crate) struct MessageQueue {
    to_send: VecDeque<MessageToSend>,
    to_receive: VecDeque<MessageToReceive>,
    /// `Config::backpressure.max_queued_bytes`; `0` disables the budget.
    max_queued_bytes: usize,
    queued_bytes: usize,
    queued_commands: usize,
    /// Replies belonging to a message that has already been resolved, and which
    /// must therefore be dropped instead of matched.
    results_to_discard: usize,
    stats: Arc<StatsRecorder>,
}

impl MessageQueue {
    pub(crate) fn new(max_queued_bytes: usize, stats: Arc<StatsRecorder>) -> Self {
        Self {
            to_send: VecDeque::new(),
            to_receive: VecDeque::new(),
            max_queued_bytes,
            queued_bytes: 0,
            queued_commands: 0,
            results_to_discard: 0,
            stats,
        }
    }

    /// Whether queuing `cost` more bytes would breach the budget.
    ///
    /// With nothing outstanding a message is always admitted, whatever its size:
    /// refusing one larger than the whole budget would make it permanently
    /// unsendable rather than merely delayed. What is held is therefore the
    /// budget plus at most one message.
    pub(crate) fn would_exceed_budget(&self, cost: usize) -> bool {
        self.max_queued_bytes != 0
            && self.queued_bytes != 0
            // A saturated sum is still above any budget, so saturating here gives
            // the same answer without an overflow to reason about.
            && self.queued_bytes.saturating_add(cost) > self.max_queued_bytes
    }

    pub(crate) fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    /// Commands still waiting to be written.
    pub(crate) fn queued_commands(&self) -> usize {
        self.queued_commands
    }

    /// Only the test-only queue-depth probe asks for this: the send path drains
    /// the queue rather than measuring it.
    #[cfg(test)]
    pub(crate) fn to_send_len(&self) -> usize {
        self.to_send.len()
    }

    pub(crate) fn to_receive_len(&self) -> usize {
        self.to_receive.len()
    }

    #[cfg(test)]
    pub(crate) fn to_send_is_empty(&self) -> bool {
        self.to_send.is_empty()
    }

    /// Queues a message for writing, charging both totals.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the running total counts bytes of buffers that are actually \
                  allocated and still queued, so it is bounded by the memory \
                  holding them. Saturating instead would silently desynchronise \
                  the backpressure accounting from what is really queued."
    )]
    pub(crate) fn push_to_send(&mut self, message: Message) {
        self.queued_bytes += message.queued_bytes();
        self.queued_commands += message.num_commands();
        self.to_send.push_back(MessageToSend::new(message));
    }

    /// Takes the next message to write, releasing its **command** charge only,
    /// and hands back the byte charge it still holds.
    ///
    /// Writing a message frees no memory, only its reply does, so the bytes stay
    /// charged; the caller passes the amount straight back to
    /// [`await_reply`](Self::await_reply) or [`release`](Self::release). It is
    /// returned rather than left for the caller to compute so that the amount
    /// released is the amount charged, whatever the message has been through in
    /// between.
    pub(crate) fn pop_to_send(&mut self) -> Option<(Message, usize)> {
        let message_to_send = self.to_send.pop_front()?;
        self.queued_commands = self
            .queued_commands
            .saturating_sub(message_to_send.message.num_commands());
        let charge = message_to_send.message.queued_bytes();
        Some((message_to_send.message, charge))
    }

    #[cfg(test)]
    pub(crate) fn front_to_send_mut(&mut self) -> Option<&mut MessageToSend> {
        self.to_send.front_mut()
    }

    /// Records a written message as awaiting `num_commands` replies, `cost`
    /// bytes staying charged until they arrive.
    pub(crate) fn await_reply(&mut self, message: Message, num_commands: usize, cost: usize) {
        self.to_receive
            .push_back(MessageToReceive::new(message, num_commands, cost));
    }

    /// Releases the byte charge of a written message that awaits no reply.
    pub(crate) fn release(&mut self, cost: usize) {
        self.queued_bytes = self.queued_bytes.saturating_sub(cost);
    }

    pub(crate) fn front_to_receive_mut(&mut self) -> Option<&mut MessageToReceive> {
        self.to_receive.front_mut()
    }

    /// Takes the message whose reply has arrived, releasing its byte charge.
    pub(crate) fn pop_to_receive(&mut self) -> Option<MessageToReceive> {
        let message_to_receive = self.to_receive.pop_front()?;
        self.queued_bytes = self
            .queued_bytes
            .saturating_sub(message_to_receive.queued_bytes);
        Some(message_to_receive)
    }

    /// Undoes the tail of a write wave that failed to flush, back to the depth
    /// the wave started at, releasing each message's charge and handing it back.
    pub(crate) fn rollback_awaiting(&mut self, start_len: usize) -> Vec<MessageToReceive> {
        let mut rolled_back = Vec::new();
        while self.to_receive.len() > start_len {
            let Some(message_to_receive) = self.to_receive.pop_back() else {
                break;
            };
            self.queued_bytes = self
                .queued_bytes
                .saturating_sub(message_to_receive.queued_bytes);
            rolled_back.push(message_to_receive);
        }
        rolled_back
    }

    /// Whether the next reply belongs to an already-resolved message and must be
    /// dropped, consuming one of the pending discards if so.
    pub(crate) fn take_discard(&mut self) -> bool {
        if self.results_to_discard > 0 {
            self.results_to_discard = self.results_to_discard.saturating_sub(1);
            true
        } else {
            false
        }
    }

    /// Marks `count` further replies as belonging to a message already resolved.
    pub(crate) fn discard_further(&mut self, count: usize) {
        self.results_to_discard = self.results_to_discard.saturating_add(count);
    }

    /// Empties both queues and **both** totals, handing back every message in
    /// replay order: those already written and awaiting a reply first, then
    /// those still queued, which preserves the original global send order.
    ///
    /// Zeroing is part of taking rather than a step the caller remembers: the
    /// messages are about to be queued again, and a total left standing would
    /// count them twice.
    pub(crate) fn take_all(&mut self) -> Vec<Message> {
        self.queued_bytes = 0;
        self.queued_commands = 0;
        self.results_to_discard = 0;
        std::mem::take(&mut self.to_receive)
            .into_iter()
            .map(|message_to_receive| message_to_receive.message)
            .chain(
                std::mem::take(&mut self.to_send)
                    .into_iter()
                    .map(|message_to_send| message_to_send.message),
            )
            .collect()
    }

    /// Keeps what a reconnection may replay, and fails the rest.
    ///
    /// A message survives only if its caller opted into retries and it has
    /// attempts left, `max_command_attempts` being the cap (`0` = unlimited).
    /// The replay itself counts as one attempt, so it is charged here.
    ///
    /// Both queues are filtered by the same rule, and in place: a message that
    /// was written and one that was not are equally lost when the socket dies.
    /// The order of the survivors is kept — a prefix-only purge would leave a
    /// non-retryable message behind a retryable one, and replay it.
    ///
    /// The pending discards go too. They name replies that died with the
    /// connection, so keeping the count would discard legitimate replies from
    /// the next one.
    ///
    /// Both totals are then rebuilt from the survivors, so this decides *what*
    /// is kept and never *what it costs*. Bytes come from both queues; commands
    /// from the send queue alone, since a retained reply waits for an answer,
    /// not to be written.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the totals are rebuilt from messages whose buffers are \
                  allocated and held, so they are bounded by that memory; \
                  `attempts` is bounded by the reconnection count."
    )]
    pub(crate) fn purge_for_replay(&mut self, max_command_attempts: usize) {
        self.results_to_discard = 0;

        // `send_error` consumes the message it answers, so a survivor is moved
        // into a new queue rather than kept in place.
        fn survives(message: &mut Message, max_command_attempts: usize) -> bool {
            if !message.retry_on_error {
                return false;
            }

            message.attempts += 1;
            !max_attempts_reached(message.attempts, max_command_attempts)
        }

        fn failure(message: &Message) -> Error {
            if message.retry_on_error {
                Error::from(ClientError::MaxCommandAttemptsReached)
            } else {
                Error::from(ErrorKind::DisconnectedByPeer)
            }
        }

        let mut retained_to_receive = VecDeque::with_capacity(self.to_receive.len());
        for mut message_to_receive in std::mem::take(&mut self.to_receive) {
            if survives(&mut message_to_receive.message, max_command_attempts) {
                retained_to_receive.push_back(message_to_receive);
            } else {
                let error = failure(&message_to_receive.message);
                message_to_receive.message.send_error(error);
            }
        }
        self.to_receive = retained_to_receive;

        let mut retained_to_send = VecDeque::with_capacity(self.to_send.len());
        for mut message_to_send in std::mem::take(&mut self.to_send) {
            if survives(&mut message_to_send.message, max_command_attempts) {
                retained_to_send.push_back(message_to_send);
            } else {
                let error = failure(&message_to_send.message);
                message_to_send.message.send_error(error);
            }
        }
        self.to_send = retained_to_send;

        self.queued_bytes = self
            .to_receive
            .iter()
            .map(|message_to_receive| message_to_receive.queued_bytes)
            .sum();
        self.queued_commands = 0;
        for message_to_send in &self.to_send {
            self.queued_bytes += message_to_send.message.queued_bytes();
            self.queued_commands += message_to_send.message.num_commands();
        }
    }

    /// Publishes the totals to the counters a client reads.
    ///
    /// Called once per network-loop iteration rather than at each operation: the
    /// queues only change inside that body, so no reader can tell the difference
    /// and the accounting keeps one owner.
    pub(crate) fn publish(&self) {
        self.stats
            .set_queued(self.queued_commands, self.queued_bytes);
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
    use super::*;
    use crate::resp::cmd;

    /// A single-command message whose reply nobody awaits.
    fn message() -> Message {
        Message::single_forget(cmd("PING").into(), true)
    }

    fn queue() -> MessageQueue {
        MessageQueue::new(0, StatsRecorder::new())
    }

    #[test]
    fn a_written_message_stops_counting_as_queued_to_send() {
        let mut queue = queue();
        queue.push_to_send(message());
        queue.push_to_send(message());
        assert_eq!(2, queue.queued_commands());

        let (msg, cost) = queue.pop_to_send().unwrap();
        queue.await_reply(msg, 1, cost);
        // Written, so it no longer waits to go out — but its bytes are held
        // until the reply, which is what bounds a connection that answers
        // nothing.
        assert_eq!(1, queue.queued_commands());
        assert!(queue.queued_bytes() > 0);
    }

    #[test]
    fn an_idle_queue_holds_no_commands_and_no_bytes() {
        let mut queue = queue();
        queue.push_to_send(message());
        let (msg, cost) = queue.pop_to_send().unwrap();
        queue.await_reply(msg, 1, cost);
        queue.pop_to_receive().unwrap();

        assert_eq!(0, queue.queued_commands());
        assert_eq!(0, queue.queued_bytes());
    }

    /// The defect the extraction was written to make impossible: a reconnection
    /// purges the queues, then replays what it kept. Both totals must be rebuilt
    /// by the replay, not one of them.
    #[test]
    fn a_reconnection_replay_charges_each_message_once() {
        let mut queue = queue();
        queue.push_to_send(message());
        queue.push_to_send(message());
        assert_eq!(2, queue.queued_commands());

        // The purge keeps everything, and rebuilds the totals from what it kept.
        queue.purge_for_replay(0);
        assert_eq!(2, queue.queued_commands());
        let after_purge = queue.queued_bytes();

        // The replay hands every message back and queues it again.
        let to_replay = queue.take_all();
        assert_eq!(2, to_replay.len());
        assert_eq!(0, queue.queued_commands(), "taking must zero both totals");
        assert_eq!(0, queue.queued_bytes(), "taking must zero both totals");
        for msg in to_replay {
            queue.push_to_send(msg);
        }

        assert_eq!(
            2,
            queue.queued_commands(),
            "a replayed message must be charged once, not once per pass"
        );
        assert_eq!(after_purge, queue.queued_bytes());

        // And the queue drains back to nothing, which an inflated total never
        // does: `pop_to_send` saturates instead of going negative, so the excess
        // would sit there for the life of the connection.
        while let Some((msg, charge)) = queue.pop_to_send() {
            queue.await_reply(msg, 1, charge);
        }
        assert_eq!(0, queue.queued_commands());
    }

    #[test]
    fn a_purge_that_drops_a_message_drops_its_charge_with_it() {
        let mut queue = queue();
        queue.push_to_send(Message::single_forget(cmd("PING").into(), false));
        queue.push_to_send(message());

        queue.purge_for_replay(0);

        assert_eq!(1, queue.queued_commands());
        assert_eq!(1, queue.to_send_len());
    }

    #[test]
    fn nothing_outstanding_admits_a_message_larger_than_the_budget() {
        let mut queue = MessageQueue::new(10, StatsRecorder::new());
        // Refusing it would make it permanently unsendable rather than delayed.
        assert!(!queue.would_exceed_budget(1_000));

        queue.push_to_send(message());
        assert!(queue.would_exceed_budget(1_000));
    }

    #[test]
    fn a_zero_budget_admits_everything() {
        let queue = queue();
        assert!(!queue.would_exceed_budget(usize::MAX));
    }

    #[test]
    fn a_failed_flush_gives_back_only_what_the_wave_added() {
        let mut queue = queue();
        queue.push_to_send(message());
        let (already_awaited, earlier_charge) = queue.pop_to_send().unwrap();
        queue.await_reply(already_awaited, 1, earlier_charge);

        let start_len = queue.to_receive_len();
        queue.push_to_send(message());
        let (in_this_wave, wave_charge) = queue.pop_to_send().unwrap();
        queue.await_reply(in_this_wave, 1, wave_charge);
        assert_eq!(earlier_charge + wave_charge, queue.queued_bytes());

        let rolled_back = queue.rollback_awaiting(start_len);
        assert_eq!(1, rolled_back.len());
        assert_eq!(1, queue.to_receive_len(), "the earlier message must stay");
        assert_eq!(
            earlier_charge,
            queue.queued_bytes(),
            "only the wave's charge is released"
        );
    }

    #[test]
    fn a_discard_is_consumed_once_per_reply() {
        let mut queue = queue();
        assert!(!queue.take_discard());

        queue.discard_further(2);
        assert!(queue.take_discard());
        assert!(queue.take_discard());
        assert!(!queue.take_discard());
    }

    /// A message whose caller is waiting, so the error a purge sends is
    /// observable.
    fn awaited_message(
        retry_on_error: bool,
    ) -> (
        Message,
        tokio::sync::oneshot::Receiver<crate::Result<RespResponse>>,
    ) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        (
            Message::single(cmd("PING").into(), sender, retry_on_error),
            receiver,
        )
    }

    #[test]
    fn a_purge_fails_what_opted_out_of_retries_and_keeps_the_rest() {
        let mut queue = queue();
        let (opted_out, mut opted_out_receiver) = awaited_message(false);
        let (retryable, mut retryable_receiver) = awaited_message(true);
        queue.push_to_send(opted_out);
        queue.push_to_send(retryable);

        queue.purge_for_replay(0);

        assert_eq!(1, queue.queued_commands());
        assert!(
            matches!(opted_out_receiver.try_recv(), Ok(Err(_))),
            "a message that opted out of retries is failed, not replayed"
        );
        assert!(
            retryable_receiver.try_recv().is_err(),
            "a retryable message is replayed, not answered"
        );
    }

    #[test]
    fn a_purge_keeps_the_order_of_the_survivors() {
        let mut queue = queue();
        queue.push_to_send(Message::single_forget(cmd("FIRST").into(), true));
        queue.push_to_send(Message::single_forget(cmd("SECOND").into(), false));
        queue.push_to_send(Message::single_forget(cmd("THIRD").into(), true));

        queue.purge_for_replay(0);

        // A prefix-only purge would leave SECOND behind FIRST and replay it.
        let replayed: Vec<_> = queue
            .take_all()
            .iter()
            .filter_map(|message| {
                message
                    .command_name()
                    .map(|name| String::from_utf8_lossy(&name).into_owned())
            })
            .collect();
        assert_eq!(vec!["FIRST", "THIRD"], replayed);
    }

    #[test]
    fn a_purge_charges_one_attempt_and_fails_a_message_out_of_budget() {
        let mut queue = queue();
        let (message, mut receiver) = awaited_message(true);
        queue.push_to_send(message);

        // Cap of 2: the first replay is the first attempt, the second exhausts it.
        queue.purge_for_replay(2);
        assert_eq!(1, queue.queued_commands());
        assert!(receiver.try_recv().is_err());

        queue.purge_for_replay(2);
        assert_eq!(0, queue.queued_commands());
        assert_eq!(0, queue.queued_bytes());
        assert!(receiver.try_recv().is_ok());
    }

    #[test]
    fn a_purge_drops_the_replies_the_dead_connection_owed() {
        let mut queue = queue();
        queue.push_to_send(message());
        let (msg, cost) = queue.pop_to_send().unwrap();
        queue.await_reply(msg, 1, cost);
        queue.discard_further(1);

        queue.purge_for_replay(0);

        assert!(
            !queue.take_discard(),
            "a discard names a reply that died with the connection"
        );
    }
}
