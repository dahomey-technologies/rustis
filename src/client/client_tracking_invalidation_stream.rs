use crate::network::PushReceiver;
use futures_util::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub struct ClientTrackingInvalidationStream {
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
            Poll::Ready(response) => match response {
                Some(response) => match response {
                    Ok(response) => match response.to::<((), Vec<String>)>() {
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
