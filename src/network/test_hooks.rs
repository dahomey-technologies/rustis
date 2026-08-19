//! Observability and fault-injection probes the network task reports into.
//!
//! The task owns its whole routing state and exposes none of it, so the
//! failure paths — a redirected message, a socket killed mid-read, a queue at
//! its high-water mark — can only be driven and observed from inside it. These
//! hooks are that seam. The module is gated behind `cfg(test)`, so nothing here
//! is compiled into a shipped build.

use crate::RetryReason;
use std::collections::VecDeque;
use std::sync::Arc;

/// Test-only observability and fault-injection hook for the send batch.
///
/// This is the queue-position primitive of the failure-path test
/// infrastructure: it lets a test deterministically force retry reasons onto a
/// message drained by [`NetworkHandler::send_messages`] and observe the retry
/// reasons every command is actually fed with. It carries no cost in shipped
/// builds because it is gated behind `cfg(test)`, so it is compiled only when
/// the crate itself is built as a test target, like the existing
/// `kill_connection_on_write` primitive.
#[derive(Clone, Default)]
pub(crate) struct SendBatchTestHook {
    /// Retry reasons to force onto the **first** message of each
    /// `send_messages` drain that contains at least one message. One entry is
    /// consumed per such drain; `None` leaves that drain untouched.
    inject_first_message_reasons: Arc<std::sync::Mutex<VecDeque<Option<Vec<RetryReason>>>>>,
    /// Records, in feed order, `(command name, number of retry reasons fed)`
    /// for every command actually fed to the connection.
    fed_retry_reasons: Arc<std::sync::Mutex<Vec<(String, usize)>>>,
    /// When set, the next fed command whose name matches is armed to kill the
    /// connection on its `usize`-th following read (see
    /// [`CommandBuilder::kill_connection_on_read`]). Consumed on first match,
    /// so it fires exactly once. Lets a test inject a send failure onto a
    /// command it does not build itself, such as the sink's internal
    /// `UNSUBSCRIBE`.
    kill_on_read_by_name: Arc<std::sync::Mutex<Option<(String, usize)>>>,
}

#[allow(
    clippy::expect_used,
    reason = "test-support code: a panic is how a test reports failure"
)]
impl SendBatchTestHook {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Queues retry reasons to be forced onto the first message of the next
    /// send drain (or `None` to skip that drain).
    pub(crate) fn push_injection(&self, reasons: Option<Vec<RetryReason>>) {
        self.inject_first_message_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .push_back(reasons);
    }

    /// Returns the recorded `(command name, number of retry reasons fed)`
    /// entries, in feed order.
    pub(crate) fn fed_retry_reasons(&self) -> Vec<(String, usize)> {
        self.fed_retry_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .clone()
    }

    pub(super) fn take_injection(&self) -> Option<Vec<RetryReason>> {
        self.inject_first_message_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .pop_front()
            .flatten()
    }

    pub(super) fn record_fed(&self, command_name: String, num_reasons: usize) {
        self.fed_retry_reasons
            .lock()
            .expect("send batch test hook mutex poisoned")
            .push((command_name, num_reasons));
    }

    /// Arms the connection to be killed on the `num_reads`-th read following the
    /// next fed command named `command_name`.
    pub(crate) fn arm_kill_on_read_for(&self, command_name: &str, num_reads: usize) {
        *self
            .kill_on_read_by_name
            .lock()
            .expect("send batch test hook mutex poisoned") =
            Some((command_name.to_owned(), num_reads));
    }

    /// If the next queued kill matches `command_name`, consumes it and returns
    /// the read count to arm.
    pub(super) fn take_kill_on_read_for(&self, command_name: &str) -> Option<usize> {
        let mut guard = self
            .kill_on_read_by_name
            .lock()
            .expect("send batch test hook mutex poisoned");
        if guard.as_ref().is_some_and(|(name, _)| name == command_name) {
            return guard.take().map(|(_, num_reads)| num_reads);
        }
        None
    }
}

/// Test-only observability of how deep the network task's internal queues get
/// and of how much traffic the pub/sub and push sinks actually absorb.
///
/// Every counter is an `Arc<AtomicUsize>`, so reading one from a test never
/// contends with the network task and the send path never takes a lock. The
/// queue depths are recorded with `fetch_max`, which makes them **high-water
/// marks**: monotone, so a drain cannot lower them and a test can observe the
/// peak after the queue is empty again. A purge therefore shows up as the mark
/// *stopping*, never as the mark falling.
///
/// `futures_channel::mpsc::UnboundedReceiver` exposes no `len()`, so the
/// delivered/failed counters are the only way to observe how much a pub/sub
/// channel holds: when the consumer never polls its stream, `delivered` *is* the
/// channel depth.
///
/// Like [`SendBatchTestHook`], it is gated behind `cfg(test)` and carries no
/// cost in shipped builds.
#[derive(Clone, Default)]
pub(crate) struct QueueMetricsTestHook {
    messages_to_send_high_water: Arc<std::sync::atomic::AtomicUsize>,
    messages_to_receive_high_water: Arc<std::sync::atomic::AtomicUsize>,
    queued_commands: Arc<std::sync::atomic::AtomicUsize>,
    queued_commands_high_water: Arc<std::sync::atomic::AtomicUsize>,
    pub_sub_delivered: Arc<std::sync::atomic::AtomicUsize>,
    pub_sub_delivery_failed: Arc<std::sync::atomic::AtomicUsize>,
    pub_sub_delivered_bytes: Arc<std::sync::atomic::AtomicUsize>,
    push_delivered: Arc<std::sync::atomic::AtomicUsize>,
    push_delivery_failed: Arc<std::sync::atomic::AtomicUsize>,
    push_delivered_bytes: Arc<std::sync::atomic::AtomicUsize>,
    read_wave_high_water: Arc<std::sync::atomic::AtomicUsize>,
    write_wave_high_water: Arc<std::sync::atomic::AtomicUsize>,
}

impl QueueMetricsTestHook {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Deepest `messages_to_send` ever observed, in messages.
    pub(crate) fn messages_to_send_high_water(&self) -> usize {
        self.messages_to_send_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Deepest `messages_to_receive` ever observed, in messages.
    pub(crate) fn messages_to_receive_high_water(&self) -> usize {
        self.messages_to_receive_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Commands waiting in the send queue at the last sample, in commands.
    ///
    /// A message can carry a whole pipeline, so this is not
    /// [`Self::messages_to_send_high_water`] in another unit. Unlike the
    /// high-water marks it is the live value and falls as the queue drains,
    /// which is what makes "the queue emptied" assertable.
    pub(crate) fn queued_commands(&self) -> usize {
        self.queued_commands
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Most commands the send queue ever held at a sample point.
    pub(crate) fn queued_commands_high_water(&self) -> usize {
        self.queued_commands_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Pub/sub messages handed to a subscriber's channel without error.
    pub(crate) fn pub_sub_delivered(&self) -> usize {
        self.pub_sub_delivered
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Pub/sub messages the subscriber's channel refused, its receiver being gone.
    pub(crate) fn pub_sub_delivery_failed(&self) -> usize {
        self.pub_sub_delivery_failed
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Wire bytes of the pub/sub messages counted by [`Self::pub_sub_delivered`].
    /// This is the payload a paused subscriber's channel retains.
    pub(crate) fn pub_sub_delivered_bytes(&self) -> usize {
        self.pub_sub_delivered_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Push messages — monitor lines and tracking invalidations — handed to their
    /// sink's channel without error.
    ///
    /// This is the denominator a shedding assertion needs: `dropped_messages()`
    /// on the stream says how many were evicted, and only this says how many were
    /// offered in the first place.
    pub(crate) fn push_delivered(&self) -> usize {
        self.push_delivered
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Push messages the sink's channel refused, its receiver being gone.
    pub(crate) fn push_delivery_failed(&self) -> usize {
        self.push_delivery_failed
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Wire bytes of the push messages counted by [`Self::push_delivered`]. This
    /// is the payload a paused sink's channel retains.
    pub(crate) fn push_delivered_bytes(&self) -> usize {
        self.push_delivered_bytes
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Most replies handled without returning to the `select!`.
    pub(crate) fn read_wave_high_water(&self) -> usize {
        self.read_wave_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Most messages taken from the channel without returning to the `select!`.
    pub(crate) fn write_wave_high_water(&self) -> usize {
        self.write_wave_high_water
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(super) fn record_read_wave(&self, handled: usize) {
        self.read_wave_high_water
            .fetch_max(handled, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn record_write_wave(&self, handled: usize) {
        self.write_wave_high_water
            .fetch_max(handled, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn record_queue_depths(
        &self,
        to_send: usize,
        to_receive: usize,
        queued_commands: usize,
    ) {
        self.messages_to_send_high_water
            .fetch_max(to_send, std::sync::atomic::Ordering::Relaxed);
        self.messages_to_receive_high_water
            .fetch_max(to_receive, std::sync::atomic::Ordering::Relaxed);
        self.queued_commands
            .store(queued_commands, std::sync::atomic::Ordering::Relaxed);
        self.queued_commands_high_water
            .fetch_max(queued_commands, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn record_pub_sub_delivered(&self, bytes: usize) {
        self.pub_sub_delivered
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.pub_sub_delivered_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn record_pub_sub_delivery_failed(&self) {
        self.pub_sub_delivery_failed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn record_push_delivered(&self, bytes: usize) {
        self.push_delivered
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.push_delivered_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn record_push_delivery_failed(&self) {
        self.push_delivery_failed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}
