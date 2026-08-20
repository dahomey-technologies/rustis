use crate::{
    network::{PubSubSender, PushSender},
    resp::{RespResponse, SubscriptionType},
};
use bytes::Bytes;
use std::collections::{HashMap, VecDeque};

/// A subscription the caller asked for, waiting for the server to confirm it.
pub(crate) struct PendingSubscription {
    pub channel_or_pattern: Bytes,
    pub subscription_type: SubscriptionType,
    pub sender: PubSubSender,
}

/// What became of a pub/sub message handed to [`Router::deliver`].
#[derive(Debug)]
pub(crate) enum Delivery {
    /// Handed to the subscriber's channel. The size is what a paused subscriber
    /// now retains, read only by the test-only traffic probe.
    Delivered {
        #[cfg_attr(
            not(test),
            allow(dead_code, reason = "read only by the `cfg(test)` traffic probe")
        )]
        retained_bytes: usize,
    },
    /// The subscriber is gone. The subscription has been dropped from the table
    /// and queued for an UNSUBSCRIBE; see [`Router::take_orphaned`].
    SubscriberGone,
    /// Nobody is subscribed to this channel or pattern, so the message is
    /// dropped. The server sent something this connection never asked for.
    NoSubscriber,
}

/// What a subscription confirmation from the server means.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SubscriptionConfirmed {
    /// Registered. `more_to_come` says whether the caller's reply is still owed,
    /// a batch of subscriptions being answered once.
    Registered { more_to_come: bool },
    /// The channel was already in the table, which the caller must be told about
    /// rather than served twice.
    AlreadySubscribed,
    /// No pending subscription matches. The pending entry, if any, is left
    /// intact: a mismatched confirmation must not consume a subscriber.
    Unexpected,
}

/// What an unsubscription confirmation from the server means.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UnsubscriptionConfirmed {
    /// One of a batch, and more of that batch are still owed.
    More,
    /// The last of its batch, so the caller's reply is due now.
    Complete,
    /// Nobody asked for this. It belongs to the caller as a plain reply rather
    /// than being swallowed here.
    Unsolicited,
}

/// Where a push from the server goes: the pub/sub subscription table and the two
/// push sinks.
///
/// # Why the two sinks are separate fields
///
/// They carry different flows — MONITOR output and client-side-caching
/// invalidations — routed by distinct connection states. Sharing one field was a
/// latent trap rather than a working multiplexer: registering one consumer
/// silently overwrote the other's slot.
///
/// # What it does not decide
///
/// Nothing here writes to the connection or answers a caller. An operation
/// returns what it found, and the caller turns that into a reply, a log line or
/// an UNSUBSCRIBE. That is what keeps the table testable without a socket: every
/// method below is a pure state transition over the table plus a channel send.
pub(crate) struct Router {
    /// Confirmed subscriptions, by channel or pattern.
    subscriptions: HashMap<Bytes, (SubscriptionType, PubSubSender)>,
    /// One entry per caller-issued SUBSCRIBE, holding what that command still
    /// waits to have confirmed.
    ///
    /// A batch, not a queue of single channels: in a cluster the command is split
    /// per node and the nodes answer in whatever order they answer, so a
    /// confirmation is matched by name rather than by rank. The caller's one
    /// reply is owed when its batch empties.
    pending_subscriptions: VecDeque<HashMap<Bytes, PendingSubscription>>,
    /// One entry per caller-issued UNSUBSCRIBE, holding what that command still
    /// waits to have confirmed.
    pending_unsubscriptions: VecDeque<HashMap<Bytes, SubscriptionType>>,
    /// Subscriptions whose subscriber is gone, collected while a push is routed
    /// and unsubscribed from at the end of the read wave.
    ///
    /// Delivery is matched on the synchronous read path, but sending the
    /// UNSUBSCRIBE needs the async send path, so the two are separated.
    orphaned_subscriptions: Vec<(Bytes, SubscriptionType)>,
    /// Sink for client-side-caching invalidation pushes.
    invalidation_sender: Option<PushSender>,
    /// Sink for MONITOR output.
    monitor_sender: Option<PushSender>,
}

impl Router {
    pub(crate) fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            pending_subscriptions: VecDeque::new(),
            pending_unsubscriptions: VecDeque::new(),
            orphaned_subscriptions: Vec::new(),
            invalidation_sender: None,
            monitor_sender: None,
        }
    }

    /// Whether this channel or pattern is already subscribed.
    pub(crate) fn is_subscribed(&self, channel_or_pattern: &Bytes) -> bool {
        self.subscriptions.contains_key(channel_or_pattern)
    }

    /// Records the subscriptions one caller-issued SUBSCRIBE waits to have
    /// confirmed.
    pub(crate) fn expect_subscriptions(
        &mut self,
        pending: impl IntoIterator<Item = PendingSubscription>,
    ) {
        let batch: HashMap<Bytes, PendingSubscription> = pending
            .into_iter()
            .map(|pending| (pending.channel_or_pattern.clone(), pending))
            .collect();
        if !batch.is_empty() {
            self.pending_subscriptions.push_back(batch);
        }
    }

    /// The channels or patterns of one kind this connection currently holds.
    ///
    /// A channel-less UNSUBSCRIBE names nothing, and cancels every subscription
    /// of its kind the connection holds. What it will be answered is therefore
    /// not readable off the command — it is this, which is why the caller is
    /// released only once every one of them comes back.
    pub(crate) fn subscriptions_of(
        &self,
        subscription_type: SubscriptionType,
    ) -> HashMap<Bytes, SubscriptionType> {
        self.subscriptions
            .iter()
            .filter(|(_, (kind, _))| *kind == subscription_type)
            .map(|(name, (kind, _))| (name.clone(), *kind))
            .collect()
    }

    /// Records the channels one caller-issued UNSUBSCRIBE waits to have
    /// confirmed.
    pub(crate) fn expect_unsubscriptions(&mut self, channels: HashMap<Bytes, SubscriptionType>) {
        self.pending_unsubscriptions.push_back(channels);
    }

    pub(crate) fn set_monitor_sink(&mut self, sink: PushSender) {
        self.monitor_sender = Some(sink);
    }

    pub(crate) fn monitor_sink(&self) -> Option<&PushSender> {
        self.monitor_sender.as_ref()
    }

    pub(crate) fn has_monitor_sink(&self) -> bool {
        self.monitor_sender.is_some()
    }

    pub(crate) fn set_invalidation_sink(&mut self, sink: PushSender) {
        self.invalidation_sender = Some(sink);
    }

    pub(crate) fn invalidation_sink_mut(&mut self) -> Option<&mut PushSender> {
        self.invalidation_sender.as_mut()
    }

    /// Hands a pub/sub message to whoever subscribed to `channel_or_pattern`.
    ///
    /// A delivery fails only when the receiving half has been dropped, which is
    /// permanent: retrying on the next message would never succeed. So the entry
    /// is removed here and queued for an UNSUBSCRIBE, which also makes this the
    /// first and only failure for that channel — one warning and one
    /// UNSUBSCRIBE, not one per message. Leaving it would keep the server
    /// publishing to a channel nobody can receive on for as long as the
    /// connection lives.
    pub(crate) fn deliver(
        &mut self,
        channel_or_pattern: &[u8],
        value: crate::Result<RespResponse>,
    ) -> Delivery {
        let retained_bytes = value
            .as_ref()
            .map_or(0, crate::resp::RespResponse::retained_bytes);

        // The key is looked up alongside its value because sending consumes
        // `value`, and with it the borrowed channel name: cleaning the
        // subscription up needs a name that outlives the send.
        let Some((key, (subscription_type, pub_sub_sender))) =
            self.subscriptions.get_key_value(channel_or_pattern)
        else {
            return Delivery::NoSubscriber;
        };

        let key = key.clone();
        let subscription_type = *subscription_type;
        match pub_sub_sender.send(value) {
            Ok(()) => Delivery::Delivered { retained_bytes },
            Err(_) => {
                self.subscriptions.remove(&key);
                self.orphaned_subscriptions.push((key, subscription_type));
                Delivery::SubscriberGone
            }
        }
    }

    /// Applies a subscription confirmation from the server.
    pub(crate) fn confirm_subscription(
        &mut self,
        channel_or_pattern: &[u8],
    ) -> SubscriptionConfirmed {
        // Only the oldest batch is looked at: a later SUBSCRIBE cannot be
        // confirmed before an earlier one has been, and taking from it would
        // release the wrong caller.
        let Some(batch) = self.pending_subscriptions.front_mut() else {
            return SubscriptionConfirmed::Unexpected;
        };
        // Removed rather than peeked at, but only on a match: a confirmation
        // naming something else must not consume — and silently drop — a
        // subscriber that is still waiting.
        let Some(pending) = batch.remove(channel_or_pattern) else {
            return SubscriptionConfirmed::Unexpected;
        };

        let more_to_come = !batch.is_empty();
        if !more_to_come {
            self.pending_subscriptions.pop_front();
        }
        if self
            .subscriptions
            .insert(
                pending.channel_or_pattern,
                (pending.subscription_type, pending.sender),
            )
            .is_some()
        {
            return SubscriptionConfirmed::AlreadySubscribed;
        }
        SubscriptionConfirmed::Registered { more_to_come }
    }

    /// Applies an unsubscription confirmation from the server.
    ///
    /// The subscription leaves the table whatever the answer: the server has
    /// stopped publishing to it either way.
    pub(crate) fn confirm_unsubscription(
        &mut self,
        channel_or_pattern: &[u8],
    ) -> UnsubscriptionConfirmed {
        self.subscriptions.remove(channel_or_pattern);

        let Some(remaining) = self.pending_unsubscriptions.front_mut() else {
            return UnsubscriptionConfirmed::Unsolicited;
        };

        if remaining.len() > 1 {
            remaining.remove(channel_or_pattern);
            UnsubscriptionConfirmed::More
        } else {
            self.pending_unsubscriptions.pop_front();
            UnsubscriptionConfirmed::Complete
        }
    }

    pub(crate) fn has_orphaned(&self) -> bool {
        !self.orphaned_subscriptions.is_empty()
    }

    /// Takes the subscriptions whose subscriber turned out to be gone during
    /// this read wave, so the caller can unsubscribe from them.
    pub(crate) fn take_orphaned(&mut self) -> Vec<(Bytes, SubscriptionType)> {
        std::mem::take(&mut self.orphaned_subscriptions)
    }

    /// Forgets every subscription, for a `RESET` that made the server forget
    /// them too.
    pub(crate) fn clear_subscriptions(&mut self) {
        self.subscriptions.clear();
    }

    /// Forgets the orphaned subscriptions, for a fresh connection that is
    /// subscribed to nothing: their UNSUBSCRIBE has already been achieved.
    pub(crate) fn clear_orphaned(&mut self) {
        self.orphaned_subscriptions.clear();
    }

    /// Prepares the table for a reconnection and reports what has to be
    /// re-issued on the new socket, in the order to issue it.
    ///
    /// Pending unsubscriptions are dropped first, emitting nothing: a fresh
    /// connection is subscribed to nothing, so a pending unsubscription has
    /// already achieved its goal. Dropping their channels up front is also what
    /// stops the resubscribe list from restoring a subscription the caller was
    /// in the middle of cancelling.
    ///
    /// A pending subscription is promoted to a confirmed one: it is being
    /// re-issued now, so the confirmation that comes back must find it in the
    /// table.
    pub(crate) fn take_resubscriptions(&mut self) -> Vec<(Bytes, SubscriptionType)> {
        for map in self.pending_unsubscriptions.drain(..) {
            for channel_or_pattern in map.into_keys() {
                self.subscriptions.remove(&channel_or_pattern);
            }
        }

        let mut to_reissue: Vec<(Bytes, SubscriptionType)> = self
            .subscriptions
            .iter()
            .map(|(channel_or_pattern, (subscription_type, _))| {
                (channel_or_pattern.clone(), *subscription_type)
            })
            .collect();

        for batch in std::mem::take(&mut self.pending_subscriptions) {
            for pending in batch.into_values() {
                to_reissue.push((
                    pending.channel_or_pattern.clone(),
                    pending.subscription_type,
                ));
                self.subscriptions.insert(
                    pending.channel_or_pattern,
                    (pending.subscription_type, pending.sender),
                );
            }
        }

        to_reissue
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
    use crate::client::bounded_channel;

    const BUDGET: usize = 1024 * 1024;

    fn channel(name: &str) -> Bytes {
        Bytes::from(name.to_owned())
    }

    /// A subscriber whose receiving half is kept alive by the returned guard.
    fn subscriber(name: &str) -> (PendingSubscription, crate::network::PubSubReceiver) {
        let (sender, receiver) = bounded_channel(BUDGET);
        (
            PendingSubscription {
                channel_or_pattern: channel(name),
                subscription_type: SubscriptionType::Channel,
                sender,
            },
            receiver,
        )
    }

    #[test]
    fn a_confirmation_registers_the_subscriber_that_asked_for_it() {
        let mut router = Router::new();
        let (pending, _receiver) = subscriber("news");
        router.expect_subscriptions([pending]);
        assert!(!router.is_subscribed(&channel("news")));

        assert_eq!(
            SubscriptionConfirmed::Registered {
                more_to_come: false
            },
            router.confirm_subscription(b"news")
        );
        assert!(router.is_subscribed(&channel("news")));
    }

    /// A confirmation naming another channel must leave the pending subscriber
    /// alone: consuming it would drop a caller that is still waiting.
    #[test]
    fn a_mismatched_confirmation_does_not_consume_the_pending_subscriber() {
        let mut router = Router::new();
        let (pending, _receiver) = subscriber("news");
        router.expect_subscriptions([pending]);

        assert_eq!(
            SubscriptionConfirmed::Unexpected,
            router.confirm_subscription(b"sports")
        );
        // Still pending, so the real confirmation still works.
        assert_eq!(
            SubscriptionConfirmed::Registered {
                more_to_come: false
            },
            router.confirm_subscription(b"news")
        );
    }

    #[test]
    fn a_batch_owes_its_caller_one_reply_at_the_last_confirmation() {
        let mut router = Router::new();
        let (first, _r1) = subscriber("a");
        let (second, _r2) = subscriber("b");
        router.expect_subscriptions([first, second]);

        assert_eq!(
            SubscriptionConfirmed::Registered { more_to_come: true },
            router.confirm_subscription(b"a")
        );
        assert_eq!(
            SubscriptionConfirmed::Registered {
                more_to_come: false
            },
            router.confirm_subscription(b"b")
        );
    }

    /// In a cluster a batch is split per node, and the nodes answer in whatever
    /// order they answer. A confirmation is matched by name, not by rank, so the
    /// caller's reply is owed once the batch is complete and not before.
    #[test]
    fn a_batch_is_confirmed_whatever_order_its_channels_come_back_in() {
        let mut router = Router::new();
        let (first, _r1) = subscriber("a");
        let (second, _r2) = subscriber("b");
        router.expect_subscriptions([first, second]);

        assert_eq!(
            SubscriptionConfirmed::Registered { more_to_come: true },
            router.confirm_subscription(b"b")
        );
        assert_eq!(
            SubscriptionConfirmed::Registered {
                more_to_come: false
            },
            router.confirm_subscription(b"a")
        );
        assert!(router.is_subscribed(&channel("a")));
        assert!(router.is_subscribed(&channel("b")));
    }

    #[test]
    fn an_unsolicited_unsubscription_belongs_to_the_caller() {
        let mut router = Router::new();
        assert_eq!(
            UnsubscriptionConfirmed::Unsolicited,
            router.confirm_unsubscription(b"news")
        );
    }

    #[test]
    fn a_batched_unsubscription_answers_once_at_the_end() {
        let mut router = Router::new();
        let mut batch = HashMap::new();
        batch.insert(channel("a"), SubscriptionType::Channel);
        batch.insert(channel("b"), SubscriptionType::Channel);
        router.expect_unsubscriptions(batch);

        assert_eq!(
            UnsubscriptionConfirmed::More,
            router.confirm_unsubscription(b"a")
        );
        assert_eq!(
            UnsubscriptionConfirmed::Complete,
            router.confirm_unsubscription(b"b")
        );
        // The batch is done, so a further one is nobody's.
        assert_eq!(
            UnsubscriptionConfirmed::Unsolicited,
            router.confirm_unsubscription(b"c")
        );
    }

    #[test]
    fn a_message_reaches_its_subscriber() {
        let mut router = Router::new();
        let (pending, _receiver) = subscriber("news");
        router.expect_subscriptions([pending]);
        router.confirm_subscription(b"news");

        assert!(matches!(
            router.deliver(b"news", Ok(RespResponse::ok())),
            Delivery::Delivered { .. }
        ));
        assert!(!router.has_orphaned());
    }

    #[test]
    fn a_message_on_a_channel_nobody_holds_is_dropped() {
        let mut router = Router::new();
        assert!(matches!(
            router.deliver(b"news", Ok(RespResponse::ok())),
            Delivery::NoSubscriber
        ));
        assert!(!router.has_orphaned());
    }

    /// A dropped receiver is permanent, so the subscription goes at the first
    /// failure: one warning and one UNSUBSCRIBE, not one per message.
    #[test]
    fn a_dead_subscriber_is_unsubscribed_once_and_only_once() {
        let mut router = Router::new();
        let (pending, receiver) = subscriber("news");
        router.expect_subscriptions([pending]);
        router.confirm_subscription(b"news");
        drop(receiver);

        assert!(matches!(
            router.deliver(b"news", Ok(RespResponse::ok())),
            Delivery::SubscriberGone
        ));
        assert!(!router.is_subscribed(&channel("news")));
        assert_eq!(1, router.take_orphaned().len());

        // The table no longer holds it, so a second message finds nobody rather
        // than orphaning it again.
        assert!(matches!(
            router.deliver(b"news", Ok(RespResponse::ok())),
            Delivery::NoSubscriber
        ));
        assert!(!router.has_orphaned());
    }

    #[test]
    fn a_reconnection_reissues_confirmed_and_pending_subscriptions() {
        let mut router = Router::new();
        let (confirmed, _r1) = subscriber("confirmed");
        router.expect_subscriptions([confirmed]);
        router.confirm_subscription(b"confirmed");
        let (still_pending, _r2) = subscriber("pending");
        router.expect_subscriptions([still_pending]);

        let to_reissue = router.take_resubscriptions();

        assert_eq!(2, to_reissue.len());
        // A re-issued pending subscription becomes a confirmed one: the
        // confirmation coming back must find it in the table.
        assert!(router.is_subscribed(&channel("pending")));
        assert!(router.is_subscribed(&channel("confirmed")));
    }

    /// A caller cancelling a subscription across a reconnection must not have it
    /// silently restored.
    #[test]
    fn a_reconnection_does_not_restore_a_subscription_being_cancelled() {
        let mut router = Router::new();
        let (pending, _receiver) = subscriber("news");
        router.expect_subscriptions([pending]);
        router.confirm_subscription(b"news");

        let mut batch = HashMap::new();
        batch.insert(channel("news"), SubscriptionType::Channel);
        router.expect_unsubscriptions(batch);

        assert!(router.take_resubscriptions().is_empty());
        assert!(!router.is_subscribed(&channel("news")));
    }

    #[test]
    fn the_two_push_sinks_do_not_share_a_slot() {
        let mut router = Router::new();
        let (monitor, _monitor_receiver) = bounded_channel(BUDGET);
        let (invalidation, _invalidation_receiver) = bounded_channel(BUDGET);

        router.set_monitor_sink(monitor);
        router.set_invalidation_sink(invalidation);

        // Registering one must not overwrite the other's slot, which is what a
        // single shared field did.
        assert!(router.has_monitor_sink());
        assert!(router.invalidation_sink_mut().is_some());
    }
}
