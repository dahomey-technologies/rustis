use crate::{resp::Value, Client, ConnectionCommandResult, PubSubCommands, PubSubReceiver, Result};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream to get messages from the channel [`subscribed`](https://redis.io/docs/manual/pubsub/) to
pub struct PubSubStream {
    channel: String,
    receiver: PubSubReceiver,
    connection: Client,
}

impl PubSubStream {
    pub(crate) fn new(channel: String, receiver: PubSubReceiver, connection: Client) -> Self {
        Self {
            channel,
            receiver,
            connection,
        }
    }
}

impl Stream for PubSubStream {
    type Item = Result<Value>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.get_mut().receiver.poll_next_unpin(cx)
    }
}

impl Drop for PubSubStream {
    fn drop(&mut self) {
        let mut channel = String::new();
        std::mem::swap(&mut channel, &mut self.channel);
        let _result = self.connection.unsubscribe(channel).send_and_forget();
    }
}
