use crate::resp::{RespResponse, RespView};
use std::fmt;

pub(crate) enum PubSubPush<'a> {
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

impl<'a> fmt::Debug for PubSubPush<'a> {
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

impl<'a> TryFrom<&'a RespResponse> for PubSubPush<'a> {
    type Error = ();

    fn try_from(response: &'a RespResponse) -> Result<Self, Self::Error> {
        if let Ok(RespView::Push(resp_array)) = response.view() {
            if resp_array.len() < 2 {
                return Err(());
            }

            let mut iterator = resp_array.into_iter();

            let Some(Ok(RespView::BulkString(kind))) = iterator.next() else {
                return Err(());
            };

            let Some(Ok(RespView::BulkString(channel_or_pattern))) = iterator.next() else {
                return Err(());
            };

            match kind {
                b"subscribe" => Ok(PubSubPush::Subscribe(channel_or_pattern)),
                b"psubscribe" => Ok(PubSubPush::PSubscribe(channel_or_pattern)),
                b"ssubscribe" => Ok(PubSubPush::SSubscribe(channel_or_pattern)),
                b"unsubscribe" => Ok(PubSubPush::Unsubscribe(channel_or_pattern)),
                b"punsubscribe" => Ok(PubSubPush::PUnsubscribe(channel_or_pattern)),
                b"sunsubscribe" => Ok(PubSubPush::SUnsubscribe(channel_or_pattern)),
                b"message" => {
                    let Some(Ok(RespView::BulkString(payload))) = iterator.next() else {
                        return Err(());
                    };

                    Ok(PubSubPush::Message(channel_or_pattern, payload))
                }
                b"pmessage" => {
                    let Some(Ok(RespView::BulkString(channel))) = iterator.next() else {
                        return Err(());
                    };

                    let Some(Ok(RespView::BulkString(payload))) = iterator.next() else {
                        return Err(());
                    };

                    Ok(PubSubPush::PMessage(channel_or_pattern, channel, payload))
                }
                b"smessage" => {
                    let Some(Ok(RespView::BulkString(payload))) = iterator.next() else {
                        return Err(());
                    };
                    Ok(PubSubPush::SMessage(channel_or_pattern, payload))
                }
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }
}
