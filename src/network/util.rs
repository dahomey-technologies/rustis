use crate::resp::{BytesSeed, RespBuf, RespDeserializer};
use serde::{de::Visitor, Deserializer};
use std::fmt;

pub enum RefPubSubMessage<'a> {
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

impl<'a> std::fmt::Debug for RefPubSubMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Subscribe(arg0) => f
                .debug_tuple("Subscribe")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .finish(),
            Self::PSubscribe(arg0) => f
                .debug_tuple("PSubscribe")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .finish(),
            Self::SSubscribe(arg0) => f
                .debug_tuple("SSubscribe")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .finish(),
            Self::Unsubscribe(arg0) => f
                .debug_tuple("Unsubscribe")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .finish(),
            Self::PUnsubscribe(arg0) => f
                .debug_tuple("PUnsubscribe")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .finish(),
            Self::SUnsubscribe(arg0) => f
                .debug_tuple("SUnsubscribe")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .finish(),
            Self::Message(arg0, arg1) => f
                .debug_tuple("Message")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .field(&std::str::from_utf8(arg1).map_err(|_| fmt::Error)?)
                .finish(),
            Self::PMessage(arg0, arg1, arg2) => f
                .debug_tuple("PMessage")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .field(&std::str::from_utf8(arg1).map_err(|_| fmt::Error)?)
                .field(&std::str::from_utf8(arg2).map_err(|_| fmt::Error)?)
                .finish(),
            Self::SMessage(arg0, arg1) => f
                .debug_tuple("SMessage")
                .field(&std::str::from_utf8(arg0).map_err(|_| fmt::Error)?)
                .field(&std::str::from_utf8(arg1).map_err(|_| fmt::Error)?)
                .finish(),
        }
    }
}

impl<'a> RefPubSubMessage<'a> {
    pub fn from_resp(resp_buffer: &'a RespBuf) -> Option<RefPubSubMessage<'a>> {
        struct RefPubSubMessageVisitor;

        impl<'de> Visitor<'de> for RefPubSubMessageVisitor {
            type Value = Option<RefPubSubMessage<'de>>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("bool")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let Ok(Some(kind)) = seq.next_element::<&str>() else {
                    return Ok(None);
                };

                let Ok(Some(channel_or_pattern)) = seq.next_element_seed(BytesSeed) else {
                    return Ok(None);
                };

                match kind {
                    "subscribe" => Ok(Some(RefPubSubMessage::Subscribe(channel_or_pattern))),
                    "psubscribe" => Ok(Some(RefPubSubMessage::PSubscribe(channel_or_pattern))),
                    "ssubscribe" => Ok(Some(RefPubSubMessage::SSubscribe(channel_or_pattern))),
                    "unsubscribe" => Ok(Some(RefPubSubMessage::Unsubscribe(channel_or_pattern))),
                    "punsubscribe" => Ok(Some(RefPubSubMessage::PUnsubscribe(channel_or_pattern))),
                    "sunsubscribe" => Ok(Some(RefPubSubMessage::SUnsubscribe(channel_or_pattern))),
                    "message" => {
                        let Ok(Some(payload)) = seq.next_element_seed(BytesSeed) else {
                            return Ok(None);
                        };

                        Ok(Some(RefPubSubMessage::Message(channel_or_pattern, payload)))
                    }
                    "pmessage" => {
                        let Ok(Some(channel)) = seq.next_element_seed(BytesSeed) else {
                            return Ok(None);
                        };

                        let Ok(Some(payload)) = seq.next_element_seed(BytesSeed) else {
                            return Ok(None);
                        };

                        Ok(Some(RefPubSubMessage::PMessage(
                            channel_or_pattern,
                            channel,
                            payload,
                        )))
                    }
                    "smessage" => {
                        let Ok(Some(payload)) = seq.next_element_seed(BytesSeed) else {
                            return Ok(None);
                        };

                        Ok(Some(RefPubSubMessage::SMessage(
                            channel_or_pattern,
                            payload,
                        )))
                    }
                    _ => Ok(None),
                }
            }
        }

        if resp_buffer.is_push_message() {
            let mut deserializer = RespDeserializer::new(resp_buffer);
            deserializer.deserialize_seq(RefPubSubMessageVisitor).unwrap_or_default()
        } else {
            None
        }
    }
}
