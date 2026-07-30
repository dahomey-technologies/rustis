use crate::{
    Error, PubSubSender, PushSender, RetryReason,
    network::{ResultSender, ResultsSender},
    resp::{Command, SubscriptionType},
};
use bytes::Bytes;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::warn;

#[cfg(test)]
static MESSAGE_SEQUENCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Per-message allowance charged against the send-queue budget on top of the
/// command bytes, covering everything a queued message holds besides its
/// buffers.
///
/// Calibrated by measurement: 100 000 queued commands carrying a 1 KiB value
/// grew resident memory by 229 MiB, i.e. ~2.3 KiB per command, of which ~1.3 KiB
/// does not scale with the value. Rounded down to 1 KiB, so the budget is
/// conservative about the part it can measure exactly (the buffers) rather than
/// about the estimate.
pub(crate) const QUEUED_MESSAGE_OVERHEAD: usize = 1024;

#[derive(Debug)]
pub(crate) enum CommandsIteratorRef<'a> {
    Single(Option<&'a Command>),
    Batch(std::slice::Iter<'a, Command>),
}

impl<'a> Iterator for CommandsIteratorRef<'a> {
    type Item = &'a Command;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(command) => command.take(),
            Self::Batch(iter) => iter.next(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum CommandsIteratorMut<'a> {
    Single(Option<&'a mut Command>),
    Batch(std::slice::IterMut<'a, Command>),
}

impl<'a> Iterator for CommandsIteratorMut<'a> {
    type Item = &'a mut Command;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(command) => command.take(),
            Self::Batch(iter) => iter.next(),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum MessageKind {
    Single {
        command: Command,
        result_sender: Option<ResultSender>,
    },
    Batch {
        commands: Vec<Command>,
        results_sender: ResultsSender,
    },
    PubSub {
        command: Command,
        result_sender: ResultSender,
        subscription_type: SubscriptionType,
        subscriptions: Vec<(Bytes, PubSubSender)>,
    },
    Monitor {
        command: Command,
        result_sender: ResultSender,
        push_sender: Option<PushSender>,
    },
    Invalidation {
        push_sender: Option<PushSender>,
    },
}

#[derive(Debug)]
pub(crate) struct Message {
    pub kind: MessageKind,
    pub retry_reasons: Option<Vec<RetryReason>>,
    pub retry_on_error: bool,
    /// Number of times this message has been (re)attempted. Incremented at each
    /// retry choke point and compared against `Config::max_command_attempts` to
    /// bound retries at the message level (see `NetworkHandler`).
    pub attempts: usize,
    #[cfg(test)]
    #[allow(unused)]
    pub(crate) message_seq: usize,
}

impl Message {
    #[inline(always)]
    pub(crate) fn single(
        command: Command,
        result_sender: ResultSender,
        retry_on_error: bool,
    ) -> Self {
        Message {
            kind: MessageKind::Single {
                command,
                result_sender: Some(result_sender),
            },
            retry_reasons: None,
            attempts: 0,
            retry_on_error,
            #[cfg(test)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub(crate) fn single_forget(command: Command, retry_on_error: bool) -> Self {
        Message {
            kind: MessageKind::Single {
                command,
                result_sender: None,
            },
            retry_reasons: None,
            attempts: 0,
            retry_on_error,
            #[cfg(test)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub(crate) fn batch(
        commands: Vec<Command>,
        results_sender: ResultsSender,
        retry_on_error: bool,
    ) -> Self {
        Message {
            kind: MessageKind::Batch {
                commands,
                results_sender,
            },
            retry_reasons: None,
            attempts: 0,
            retry_on_error,
            #[cfg(test)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub(crate) fn pub_sub(
        command: Command,
        result_sender: ResultSender,
        subscription_type: SubscriptionType,
        subscriptions: Vec<(Bytes, PubSubSender)>,
    ) -> Self {
        Message {
            kind: MessageKind::PubSub {
                command,
                result_sender,
                subscription_type,
                subscriptions,
            },
            retry_reasons: None,
            attempts: 0,
            retry_on_error: true,
            #[cfg(test)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub(crate) fn monitor(
        command: Command,
        result_sender: ResultSender,
        push_sender: PushSender,
    ) -> Self {
        Message {
            kind: MessageKind::Monitor {
                command,
                result_sender,
                push_sender: Some(push_sender),
            },
            retry_reasons: None,
            attempts: 0,
            retry_on_error: true,
            #[cfg(test)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub(crate) fn client_tracking_invalidation(push_sender: PushSender) -> Self {
        Message {
            kind: MessageKind::Invalidation {
                push_sender: Some(push_sender),
            },
            retry_reasons: None,
            attempts: 0,
            retry_on_error: false,
            #[cfg(test)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    /// The connection identity comes from the surrounding span, so this takes no
    /// tag: every caller is the network task, which is already inside it.
    pub(crate) fn send_error(self, error: Error) {
        match self.kind {
            MessageKind::Single {
                result_sender: Some(result_sender),
                ..
            } => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                        "Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            MessageKind::Batch { results_sender, .. } => {
                if let Err(e) = results_sender.send(Err(error)) {
                    warn!(
                        "Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            MessageKind::PubSub { result_sender, .. } => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                        "Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            MessageKind::Monitor { result_sender, .. } => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                        "Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            _ => (), // nothing to answer
        }
    }

    pub(crate) fn num_commands(&self) -> usize {
        match &self.kind {
            MessageKind::Single { .. } => 1,
            MessageKind::Batch { commands, .. } => commands.len(),
            MessageKind::PubSub { .. } => 1,
            MessageKind::Monitor { .. } => 1,
            MessageKind::Invalidation { .. } => 0,
        }
    }

    /// What this message costs against `BackpressureConfig::max_queued_bytes`
    /// while it waits in the send queue.
    ///
    /// The command bytes are exact and free to read: a `Command` owns one
    /// contiguous buffer holding its name and every argument, so this is a
    /// `len()` per command rather than a walk over the arguments.
    ///
    /// [`QUEUED_MESSAGE_OVERHEAD`] is added on top because the buffers are not
    /// the whole cost: a message also carries its result channel, its retry
    /// bookkeeping and the queue node itself. Without it, a flood of tiny
    /// commands would pass under any byte budget while consuming far more than
    /// the budget allows.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the total sums the lengths of buffers that are allocated and held by \
                  this message, plus a small constant."
    )]
    pub(crate) fn queued_bytes(&self) -> usize {
        self.commands()
            .map(|command| command.bytes().len())
            .sum::<usize>()
            + QUEUED_MESSAGE_OVERHEAD
    }

    pub(crate) fn commands(&self) -> CommandsIteratorRef<'_> {
        match &self.kind {
            MessageKind::Single { command, .. } => CommandsIteratorRef::Single(Some(command)),
            MessageKind::Batch { commands, .. } => CommandsIteratorRef::Batch(commands.iter()),
            MessageKind::PubSub { command, .. } => CommandsIteratorRef::Single(Some(command)),
            MessageKind::Monitor { command, .. } => CommandsIteratorRef::Single(Some(command)),
            MessageKind::Invalidation { push_sender: _ } => CommandsIteratorRef::Single(None),
        }
    }

    pub(crate) fn commands_mut(&mut self) -> CommandsIteratorMut<'_> {
        match &mut self.kind {
            MessageKind::Single { command, .. } => CommandsIteratorMut::Single(Some(command)),
            MessageKind::Batch { commands, .. } => CommandsIteratorMut::Batch(commands.iter_mut()),
            MessageKind::PubSub { command, .. } => CommandsIteratorMut::Single(Some(command)),
            MessageKind::Monitor { command, .. } => CommandsIteratorMut::Single(Some(command)),
            MessageKind::Invalidation { push_sender: _ } => CommandsIteratorMut::Single(None),
        }
    }
}
