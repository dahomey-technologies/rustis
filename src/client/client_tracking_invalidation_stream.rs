use crate::network::PushReceiver;
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub(crate) struct ClientTrackingInvalidationStream {
    receiver: PushReceiver,
}

impl ClientTrackingInvalidationStream {
    pub(crate) fn new(receiver: PushReceiver) -> Self {
        Self { receiver }
    }
}

impl Stream for ClientTrackingInvalidationStream {
    type Item = Vec<String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.get_mut().receiver.poll_next_unpin(cx) {
            Poll::Ready(value) => match value {
                Some(value) => match value {
                    Ok(value) => match value.into::<(String, Vec<String>)>() {
                        Ok((_invalidate, keys)) => Poll::Ready(Some(keys)),
                        Err(_) => Poll::Ready(None),
                    },
                    Err(_) => Poll::Ready(None),
                },
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
