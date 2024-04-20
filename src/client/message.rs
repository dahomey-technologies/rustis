use log::warn;
use smallvec::SmallVec;

use crate::{
    network::{ResultSender, ResultsSender},
    resp::Command,
    Error, PubSubSender, PushSender, RetryReason,
};

#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(debug_assertions)]
static MESSAGE_SEQUENCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum Commands {
    None,
    Single(Command, Option<ResultSender>),
    Batch(Vec<Command>, ResultsSender),
}

impl Commands {
    pub fn len(&self) -> usize {
        match &self {
            Commands::None => 0,
            Commands::Single(_, _) => 1,
            Commands::Batch(commands, _) => commands.len(),
        }
    }

    pub fn send_error(self, tag: &str, error: Error) {
        match self {
            Commands::Single(_, Some(result_sender)) => {
                if let Err(e) = result_sender.send(Err(error)) {
                    warn!(
                    "[{tag}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                );
                }
            }
            Commands::Batch(_, results_sender) => {
                if let Err(e) = results_sender.send(Err(error)) {
                    warn!(
                    "[{tag}] Cannot send value to caller because receiver is not there anymore: {e:?}",
                );
                }
            }
            _ => (),
        }
    }
}

impl IntoIterator for Commands {
    type Item = Command;
    type IntoIter = CommandsIterator;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Commands::None => CommandsIterator::Single(None),
            Commands::Single(command, _) => CommandsIterator::Single(Some(command)),
            Commands::Batch(commands, _) => CommandsIterator::Batch(commands.into_iter()),
        }
    }
}

impl<'a> IntoIterator for &'a Commands {
    type Item = &'a Command;
    type IntoIter = RefCommandsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Commands::None => RefCommandsIterator::Single(None),
            Commands::Single(command, _) => RefCommandsIterator::Single(Some(command)),
            Commands::Batch(commands, _) => RefCommandsIterator::Batch(commands.iter()),
        }
    }
}

impl<'a> IntoIterator for &'a mut Commands {
    type Item = &'a mut Command;
    type IntoIter = CommandsIteratorMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Commands::None => CommandsIteratorMut::Single(None),
            Commands::Single(command, _) => CommandsIteratorMut::Single(Some(command)),
            Commands::Batch(commands, _) => CommandsIteratorMut::Batch(commands.iter_mut()),
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum CommandsIterator {
    Single(Option<Command>),
    Batch(std::vec::IntoIter<Command>),
}

impl Iterator for CommandsIterator {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(command) => command.take(),
            Self::Batch(iter) => iter.next(),
        }
    }
}

pub enum RefCommandsIterator<'a> {
    Single(Option<&'a Command>),
    Batch(std::slice::Iter<'a, Command>),
}

impl<'a> Iterator for RefCommandsIterator<'a> {
    type Item = &'a Command;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(command) => command.take(),
            Self::Batch(iter) => iter.next(),
        }
    }
}

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
pub(crate) struct Message {
    pub commands: Commands,
    pub pub_sub_senders: Option<Vec<(Vec<u8>, PubSubSender)>>,
    pub push_sender: Option<PushSender>,
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
            commands: Commands::Single(command, Some(result_sender)),
            pub_sub_senders: None,
            push_sender: None,
            retry_reasons: None,
            retry_on_error,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn single_forget(command: Command, retry_on_error: bool) -> Self {
        Message {
            commands: Commands::Single(command, None),
            pub_sub_senders: None,
            push_sender: None,
            retry_reasons: None,
            retry_on_error,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn batch(
        commands: Vec<Command>,
        results_sender: ResultsSender,
        retry_on_error: bool,
    ) -> Self {
        Message {
            commands: Commands::Batch(commands, results_sender),
            pub_sub_senders: None,
            push_sender: None,
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
        pub_sub_senders: Vec<(Vec<u8>, PubSubSender)>,
    ) -> Self {
        Message {
            commands: Commands::Single(command, Some(result_sender)),
            pub_sub_senders: Some(pub_sub_senders),
            push_sender: None,
            retry_reasons: None,
            retry_on_error: true,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn monitor(command: Command, result_sender: ResultSender, push_sender: PushSender) -> Self {
        Message {
            commands: Commands::Single(command, Some(result_sender)),
            pub_sub_senders: None,
            push_sender: Some(push_sender),
            retry_reasons: None,
            retry_on_error: true,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    #[inline(always)]
    pub fn client_tracking_invalidation(push_sender: PushSender) -> Self {
        Message {
            commands: Commands::None,
            pub_sub_senders: None,
            push_sender: Some(push_sender),
            retry_reasons: None,
            retry_on_error: false,
            #[cfg(debug_assertions)]
            message_seq: MESSAGE_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }
}
