use crate::resp::{RespResponse, RespView};
use std::fmt;

pub enum PubSubMessage<'a> {
    Subscribe(&'a [u8]),
    PSubscribe(&'a [u8]),
    SSubscribe(&'a [u8]),
    Unsubscribe(&'a [u8]),
    PUnsubscribe(&'a [u8]),
    SUnsubscribe(&'a [u8]),
    Message(&'a [u8], &'a [u8]),
    PMessage(&'a [u8], &'a [u8], &'a [u8]),
    SMessage(&'a [u8], &'a [u8]),
}

impl<'a> fmt::Debug for PubSubMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Subscribe(arg0) => f
                .debug_tuple("Subscribe")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::PSubscribe(arg0) => f
                .debug_tuple("PSubscribe")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::SSubscribe(arg0) => f
                .debug_tuple("SSubscribe")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::Unsubscribe(arg0) => f
                .debug_tuple("Unsubscribe")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::PUnsubscribe(arg0) => f
                .debug_tuple("PUnsubscribe")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::SUnsubscribe(arg0) => f
                .debug_tuple("SUnsubscribe")
                .field(&String::from_utf8_lossy(arg0))
                .finish(),
            Self::Message(arg0, arg1) => f
                .debug_tuple("Message")
                .field(&String::from_utf8_lossy(arg0))
                .field(&String::from_utf8_lossy(arg1))
                .finish(),
            Self::PMessage(arg0, arg1, arg2) => f
                .debug_tuple("PMessage")
                .field(&String::from_utf8_lossy(arg0))
                .field(&String::from_utf8_lossy(arg1))
                .field(&String::from_utf8_lossy(arg2))
                .finish(),
            Self::SMessage(arg0, arg1) => f
                .debug_tuple("SMessage")
                .field(&String::from_utf8_lossy(arg0))
                .field(&String::from_utf8_lossy(arg1))
                .finish(),
        }
    }
}

impl<'a> TryFrom<&'a RespResponse> for PubSubMessage<'a> {
    type Error = ();

    fn try_from(response: &'a RespResponse) -> Result<Self, Self::Error> {
        if let RespView::Push(resp_array) = response.view() {
            if resp_array.len() < 2 {
                return Err(());
            }

            let mut iterator = resp_array.into_iter();

            let Some(RespView::BulkString(kind)) = iterator.next() else {
                return Err(());
            };

            let Some(RespView::BulkString(channel_or_pattern)) = iterator.next() else {
                return Err(());
            };

            match kind {
                b"subscribe" => Ok(PubSubMessage::Subscribe(channel_or_pattern)),
                b"psubscribe" => Ok(PubSubMessage::PSubscribe(channel_or_pattern)),
                b"ssubscribe" => Ok(PubSubMessage::SSubscribe(channel_or_pattern)),
                b"unsubscribe" => Ok(PubSubMessage::Unsubscribe(channel_or_pattern)),
                b"punsubscribe" => Ok(PubSubMessage::PUnsubscribe(channel_or_pattern)),
                b"sunsubscribe" => Ok(PubSubMessage::SUnsubscribe(channel_or_pattern)),
                b"message" => {
                    let Some(RespView::BulkString(channel)) = iterator.next() else {
                        return Err(());
                    };

                    let Some(RespView::BulkString(payload)) = iterator.next() else {
                        return Err(());
                    };

                    Ok(PubSubMessage::Message(channel, payload))
                }
                b"pmessage" => {
                    let Some(RespView::BulkString(channel)) = iterator.next() else {
                        return Err(());
                    };

                    let Some(RespView::BulkString(payload)) = iterator.next() else {
                        return Err(());
                    };

                    Ok(PubSubMessage::PMessage(
                        channel_or_pattern,
                        channel,
                        payload,
                    ))
                }
                b"smessage" => {
                    let Some(RespView::BulkString(payload)) = iterator.next() else {
                        return Err(());
                    };
                    Ok(PubSubMessage::SMessage(channel_or_pattern, payload))
                }
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }
}
