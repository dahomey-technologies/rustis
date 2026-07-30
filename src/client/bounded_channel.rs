//! Bounded channel carrying pub/sub messages from the network task to a
//! subscriber, dropping the oldest messages rather than growing without limit.
//!
//! # Why this is neither an `mpsc` nor a mutex-guarded queue
//!
//! The network task must never block or await on delivery: it is the single
//! owner of the connection's routing state, so anything that makes it wait on a
//! subscriber stalls every other caller on the same connection.
//!
//! That rules out a bounded `mpsc`, whose only non-blocking option is to reject
//! the *newest* message — which leaves a lagging subscriber stuck on stale data
//! it can never catch up from. Dropping the *oldest* instead requires the
//! producer to reach the head of the queue, which no `mpsc` exposes.
//!
//! It also rules out guarding a `VecDeque` with a mutex, even though the
//! critical section would be a few instructions: the consumer holds that lock
//! while popping, and a consumer descheduled inside it — routine under a
//! CPU-quota-throttled container, which is exactly the deployment this budget
//! exists for — would block the network task for as long as it takes to be
//! rescheduled. A tiny critical section makes that unlikely, not impossible, and
//! "unlikely to stall the whole connection" is not the guarantee this path
//! needs.
//!
//! [`SegQueue`] is lock-free and MPMC, so **both** ends can pop: the producer
//! evicts from the head to stay within budget while the consumer pops from the
//! same head, and neither can ever wait on the other.

use crate::{Result, resp::RespResponse};
use crossbeam_queue::SegQueue;
use futures_util::{Stream, task::AtomicWaker};
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

type Item = Result<RespResponse>;

struct Shared {
    /// Queued messages, each paired with the byte cost it was charged.
    queue: SegQueue<(Item, usize)>,
    /// Bytes currently queued.
    ///
    /// Tracked separately from the queue, so it can lag a push or a pop by one
    /// message: the pair is not updated atomically. The budget is therefore
    /// honoured to within one in-flight message on each side, which is the same
    /// slack the send queue already accepts and is documented as such.
    bytes: AtomicUsize,
    waker: AtomicWaker,
    /// Messages evicted to stay within budget, over the channel's whole life.
    dropped: AtomicUsize,
    /// Live senders. The receiver ends its stream when this reaches zero.
    senders: AtomicUsize,
    receiver_alive: AtomicUsize,
    /// `0` disables the budget, restoring unbounded growth.
    max_bytes: usize,
}

/// Error returned when the subscriber is gone, so nothing can receive.
///
/// Carries the message back so the caller can report what was lost.
#[derive(Debug)]
pub(crate) struct SendError(Item);

impl SendError {
    pub(crate) fn into_inner(self) -> Item {
        self.0
    }
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the subscriber is gone")
    }
}

/// Sending half, held by the network task, one clone per subscribed channel or
/// pattern.
pub(crate) struct BoundedSender {
    shared: Arc<Shared>,
}

/// Receiving half, polled as a `Stream` by the subscriber.
pub(crate) struct BoundedReceiver {
    shared: Arc<Shared>,
}

/// Creates a pub/sub channel holding at most `max_bytes` of undelivered
/// messages; `0` disables the budget.
pub(crate) fn bounded_channel(max_bytes: usize) -> (BoundedSender, BoundedReceiver) {
    let shared = Arc::new(Shared {
        queue: SegQueue::new(),
        bytes: AtomicUsize::new(0),
        waker: AtomicWaker::new(),
        dropped: AtomicUsize::new(0),
        senders: AtomicUsize::new(1),
        receiver_alive: AtomicUsize::new(1),
        max_bytes,
    });
    (
        BoundedSender {
            shared: Arc::clone(&shared),
        },
        BoundedReceiver { shared },
    )
}

impl BoundedSender {
    /// Queues a message, evicting the oldest ones if that breaches the budget.
    ///
    /// Never blocks and never awaits. Fails only when the subscriber is gone,
    /// which is the same condition the previous unbounded channel reported.
    pub(crate) fn send(&self, item: Item) -> std::result::Result<(), SendError> {
        if self.shared.receiver_alive.load(Ordering::Acquire) == 0 {
            return Err(SendError(item));
        }

        let cost = item
            .as_ref()
            .map(|response| response.retained_bytes())
            .unwrap_or(0);

        self.shared.queue.push((item, cost));
        self.shared.bytes.fetch_add(cost, Ordering::AcqRel);

        if self.shared.max_bytes != 0 {
            let mut evicted = 0usize;
            // Keep the newest: a subscriber that resumes should see current data,
            // not a prefix it can never catch up from.
            //
            // `len() > 1` keeps the queue from being emptied by its own newest
            // message when that message is larger than the whole budget: an
            // oversized message is delivered rather than made undeliverable,
            // exactly as the send queue always admits into an empty queue.
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "one increment per queue entry actually popped, so the \
                          count is bounded by the queue length."
            )]
            while self.shared.bytes.load(Ordering::Acquire) > self.shared.max_bytes
                && self.shared.queue.len() > 1
            {
                let Some((_, cost)) = self.shared.queue.pop() else {
                    // The consumer emptied the queue between the two checks.
                    break;
                };
                self.shared.bytes.fetch_sub(cost, Ordering::AcqRel);
                evicted += 1;
            }
            if evicted > 0 {
                self.shared.dropped.fetch_add(evicted, Ordering::Relaxed);
            }
        }

        self.shared.waker.wake();
        Ok(())
    }
}

impl Clone for BoundedSender {
    fn clone(&self) -> Self {
        self.shared.senders.fetch_add(1, Ordering::Relaxed);
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl Drop for BoundedSender {
    fn drop(&mut self) {
        if self.shared.senders.fetch_sub(1, Ordering::AcqRel) == 1 {
            // Last sender gone: wake the consumer so its stream ends instead of
            // parking forever.
            self.shared.waker.wake();
        }
    }
}

impl std::fmt::Debug for BoundedSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundedSender")
    }
}

impl BoundedReceiver {
    /// Messages dropped so far to stay within the budget.
    pub(crate) fn dropped_messages(&self) -> usize {
        self.shared.dropped.load(Ordering::Relaxed)
    }
}

impl Stream for BoundedReceiver {
    type Item = Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let shared = &self.shared;

        // Register before the final emptiness check, so a message queued
        // between the check and the registration still wakes this task.
        shared.waker.register(cx.waker());

        if let Some((item, cost)) = shared.queue.pop() {
            shared.bytes.fetch_sub(cost, Ordering::AcqRel);
            return Poll::Ready(Some(item));
        }

        if shared.senders.load(Ordering::Acquire) == 0 {
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}

impl Drop for BoundedReceiver {
    fn drop(&mut self) {
        self.shared.receiver_alive.store(0, Ordering::Release);
        // Release what is still queued now rather than when the last sender
        // goes: the network task keeps a sender per subscription and may hold it
        // long after the consumer is gone.
        while self.shared.queue.pop().is_some() {}
        self.shared.bytes.store(0, Ordering::Release);
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic is how a test reports failure"
)]
mod tests {
    use super::*;
    use crate::resp::{RespFrameParser, RespResponse, RespTapeMut};
    use bytes::Bytes;
    use futures_util::{FutureExt, StreamExt};

    /// Builds a simple-string response of exactly `payload` wire bytes, tagged
    /// with `id` so a test can tell which messages survived.
    fn response(id: usize, payload: usize) -> Item {
        let body = format!("+{id:0>width$}\r\n", width = payload - 3);
        let bytes = Bytes::from(body);
        let mut tape = RespTapeMut::default();
        let mut parser = RespFrameParser::new(&bytes, &mut tape);
        let (frame, _) = parser.parse().unwrap();
        Ok(RespResponse::new(bytes.into(), frame))
    }

    fn id_of(item: &Item) -> usize {
        let text: String = item.as_ref().unwrap().to().unwrap();
        text.trim_start_matches('0').parse().unwrap_or(0)
    }

    /// The budget must evict the **oldest** messages, so a subscriber that
    /// resumes reading sees current data instead of a stale prefix it can never
    /// catch up from.
    #[test]
    fn the_oldest_messages_are_the_ones_dropped() {
        const PAYLOAD: usize = 100;
        // Room for three messages, so the last three of ten must survive.
        let (sender, mut receiver) = bounded_channel(3 * PAYLOAD);

        for id in 0..10 {
            sender.send(response(id, PAYLOAD)).unwrap();
        }

        let mut received = Vec::new();
        while let Some(Some(item)) = receiver.next().now_or_never() {
            received.push(id_of(&item));
        }

        assert_eq!(
            vec![7, 8, 9],
            received,
            "the newest messages must be the ones kept"
        );
        assert_eq!(
            7,
            receiver.dropped_messages(),
            "every evicted message must be counted"
        );
    }

    /// A consumer that keeps up must lose nothing, and must see a zero counter.
    #[test]
    fn a_consumer_that_keeps_up_loses_nothing() {
        const PAYLOAD: usize = 100;
        let (sender, mut receiver) = bounded_channel(3 * PAYLOAD);

        for id in 0..10 {
            sender.send(response(id, PAYLOAD)).unwrap();
            let item = receiver.next().now_or_never().unwrap().unwrap();
            assert_eq!(id, id_of(&item));
        }

        assert_eq!(0, receiver.dropped_messages());
    }

    /// A budget of `0` disables the bound, which is the documented escape hatch.
    #[test]
    fn a_zero_budget_keeps_everything() {
        const PAYLOAD: usize = 100;
        let (sender, mut receiver) = bounded_channel(0);

        for id in 0..50 {
            sender.send(response(id, PAYLOAD)).unwrap();
        }

        let mut count = 0;
        while receiver.next().now_or_never().flatten().is_some() {
            count += 1;
        }
        assert_eq!(50, count);
        assert_eq!(0, receiver.dropped_messages());
    }

    /// The stream must end once no sender is left, rather than park forever.
    #[test]
    fn the_stream_ends_when_the_last_sender_goes() {
        let (sender, mut receiver) = bounded_channel(0);
        sender.send(response(1, 100)).unwrap();
        drop(sender);

        assert!(receiver.next().now_or_never().flatten().is_some());
        assert!(
            receiver.next().now_or_never().unwrap().is_none(),
            "the stream must end, not stay pending"
        );
    }

    /// Sending to a departed subscriber must fail and hand the message back,
    /// which is how the network task reports what it could not deliver.
    #[test]
    fn sending_to_a_departed_subscriber_returns_the_message() {
        let (sender, receiver) = bounded_channel(0);
        drop(receiver);

        let error = sender.send(response(1, 100)).unwrap_err();
        assert_eq!(1, id_of(&error.into_inner()));
    }
}
