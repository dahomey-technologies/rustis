use crate::{resp::BulkString, ConnectionMultiplexer, PubSubReceiver, Result};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct PubSubStream {
    channel: String,
    receiver: PubSubReceiver,
    connection: ConnectionMultiplexer,
}

impl PubSubStream {
    pub(crate) fn new(
        channel: String,
        receiver: PubSubReceiver,
        connection: ConnectionMultiplexer,
    ) -> Self {
        Self {
            channel,
            receiver,
            connection,
        }
    }
}

impl Stream for PubSubStream {
    type Item = Result<BulkString>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.get_mut().receiver.poll_next_unpin(cx)
    }
}

impl Drop for PubSubStream {
    fn drop(&mut self) {
        let mut channel = String::new();
        std::mem::swap(&mut channel, &mut self.channel);
        let _ = self.connection.unsubscribe(channel.into());
    }
}
