use crate::{network::PushReceiver, resp::BulkString};
use futures_util::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tracing::warn;

pub struct ClientTrackingInvalidationStream {
    receiver: PushReceiver,
}

impl ClientTrackingInvalidationStream {
    pub(crate) fn new(receiver: PushReceiver) -> Self {
        Self { receiver }
    }

    /// Number of invalidation messages dropped so far because this stream fell
    /// behind its memory budget.
    ///
    /// **A non-zero value means the reader no longer knows which keys are
    /// stale.** Unlike a pub/sub message, a lost invalidation is not merely
    /// missing data: acting on the remaining ones would leave the dropped keys
    /// cached and served forever. A consumer that observes this counter move
    /// must discard everything it cached, which is what the `Cache` (feature `client-cache`)
    /// does — the same response it already has for invalidations lost across a
    /// reconnection.
    pub fn dropped_messages(&self) -> usize {
        self.receiver.dropped_messages()
    }
}

impl Stream for ClientTrackingInvalidationStream {
    /// Redis keys are binary-safe, hence [`BulkString`] rather than `String`:
    /// a key that is not valid UTF-8 must still reach the consumer.
    type Item = Vec<BulkString>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // An undecodable event must not end the stream: the consumer would stop
        // polling and never learn of another invalidation, leaving every cache
        // built on top of it serving stale data for good. Skip it and keep
        // reading instead.
        loop {
            let Poll::Ready(response) = this.receiver.poll_next_unpin(cx) else {
                return Poll::Pending;
            };

            let Some(response) = response else {
                return Poll::Ready(None);
            };

            match response {
                Ok(response) => match response.to::<((), Vec<BulkString>)>() {
                    Ok((_invalidate, keys)) => return Poll::Ready(Some(keys)),
                    Err(e) => warn!("Cannot decode a client tracking invalidation: {e}"),
                },
                Err(e) => warn!("Error while receiving a client tracking invalidation: {e}"),
            }
        }
    }
}
