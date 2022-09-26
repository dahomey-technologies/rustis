use crate::{resp::Command, PubSubSender, ValueSender};

#[derive(Debug)]
pub(crate) struct Message {
    pub command: Command,
    pub value_sender: Option<ValueSender>,
    pub pub_sub_senders: Option<Vec<(Vec<u8>, PubSubSender)>>,
}

impl Message {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            value_sender: None,
            pub_sub_senders: None,
        }
    }

    #[allow(dead_code)]
    pub fn value_sender(mut self, value_sender: ValueSender) -> Self {
        self.value_sender = Some(value_sender);
        self
    }

    #[allow(dead_code)]
    pub fn pub_sub_senders(mut self, pub_sub_senders: Vec<(Vec<u8>, PubSubSender)>) -> Self {
        self.pub_sub_senders = Some(pub_sub_senders);
        self
    }
}
