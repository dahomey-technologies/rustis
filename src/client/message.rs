use crate::{
    Error, PubSubSender, PushSender, RetryReason,
    network::{ResultSender, ResultsSender},
    resp::{Command, SubscriptionType},
};
use bytes::Bytes;
use log::warn;
use smallvec::SmallVec;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(debug_assertions)]
static MESSAGE_SEQUENCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
pub enum CommandsIteratorRef<'a> {
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
pub enum CommandsIteratorMut<'a> {
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
        commands: SmallVec<[Command; 10]>,
        results_sender: ResultsSender,
    },
    PubSub {
        command: Command,
        result_sender: ResultSender,
        subscription_type: SubscriptionType,
        subscriptions: SmallVec<[(Bytes, PubSubSender); 10]>,
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
    pub retry_reasons: Option<SmallVec<[RetryReason; 10]>>,
    pub retry_on_error: bool,
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) message_seq: usize,
}

impl Message {
    #[inline(always)]
    pub fn single(command: Command, result_sender: ResultSender, retry_on_error: bool) -> Self {
        Message {
            kind: MessageKind::Single {
                command,
                result_sender: Some(result_sender),
            },
            retry_reasons: None,
            retry_on_error,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn single_forget(command: Command, retry_on_error: bool) -> Self {
        Message {
            kind: MessageKind::Single {
                command,
                result_sender: None,
            },
            retry_reasons: None,
            retry_on_error,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn batch(
        commands: SmallVec<[Command; 10]>,
        results_sender: ResultsSender,
        retry_on_error: bool,
    ) -> Self {
        Message {
            kind: MessageKind::Batch {
                commands,
                results_sender,
            },
            retry_reasons: None,
            retry_on_error,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn pub_sub(
        command: Command,
        result_sender: ResultSender,
        subscription_type: SubscriptionType,
        subscriptions: SmallVec<[(Bytes, PubSubSender); 10]>,
    ) -> Self {
        Message {
            kind: MessageKind::PubSub {
                command,
                result_sender,
                subscription_type,
                subscriptions,
            },
            retry_reasons: None,
            retry_on_error: true,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn monitor(command: Command, result_sender: ResultSender, push_sender: PushSender) -> Self {
        Message {
            kind: MessageKind::Monitor {
                command,
                result_sender,
                push_sender: Some(push_sender),
            },
            retry_reasons: None,
            retry_on_error: true,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn client_tracking_invalidation(push_sender: PushSender) -> Self {
        Message {
            kind: MessageKind::Invalidation {
                push_sender: Some(push_sender),
            },
            retry_reasons: None,
            retry_on_error: false,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    pub fn send_error(self, tag: &str, error: Error) {
        match self.kind {
            MessageKind::Single {
                result_sender: Some(result_sender),
                ..
            } => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                        "[{tag}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            MessageKind::Batch { results_sender, .. } => {
                if let Err(e) = results_sender.send(Err(error)) {
                    warn!(
                        "[{tag}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            MessageKind::PubSub { result_sender, .. } => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                        "[{tag}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            MessageKind::Monitor { result_sender, .. } => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                        "[{tag}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                    );
                }
            }
            _ => (), // nothing to answer
        }
    }

    pub fn num_commands(&self) -> usize {
        match &self.kind {
            MessageKind::Single { .. } => 1,
            MessageKind::Batch { commands, .. } => commands.len(),
            MessageKind::PubSub { .. } => 1,
            MessageKind::Monitor { .. } => 1,
            MessageKind::Invalidation { .. } => 0,
        }
    }

    pub fn commands(&self) -> CommandsIteratorRef<'_> {
        match &self.kind {
            MessageKind::Single { command, .. } => CommandsIteratorRef::Single(Some(command)),
            MessageKind::Batch { commands, .. } => CommandsIteratorRef::Batch(commands.into_iter()),
            MessageKind::PubSub { command, .. } => CommandsIteratorRef::Single(Some(command)),
            MessageKind::Monitor { command, .. } => CommandsIteratorRef::Single(Some(command)),
            MessageKind::Invalidation { push_sender: _ } => CommandsIteratorRef::Single(None),
        }
    }

    pub fn commands_mut(&mut self) -> CommandsIteratorMut<'_> {
        match &mut self.kind {
            MessageKind::Single { command, .. } => CommandsIteratorMut::Single(Some(command)),
            MessageKind::Batch { commands, .. } => CommandsIteratorMut::Batch(commands.into_iter()),
            MessageKind::PubSub { command, .. } => CommandsIteratorMut::Single(Some(command)),
            MessageKind::Monitor { command, .. } => CommandsIteratorMut::Single(Some(command)),
            MessageKind::Invalidation { push_sender: _ } => CommandsIteratorMut::Single(None),
        }
    }
}
