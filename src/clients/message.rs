use crate::{resp::Command, PubSubSender, ValueSender, MonitorSender};

#[derive(Debug)]
pub(crate) struct Message {
    pub command: Command,
    pub value_sender: Option<ValueSender>,
    pub pub_sub_senders: Option<Vec<(Vec<u8>, PubSubSender)>>,
    pub monitor_sender: Option<MonitorSender>,
}

impl Message {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            value_sender: None,
            pub_sub_senders: None,
            monitor_sender: None,
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

    #[allow(dead_code)]
    pub fn monitor_sender(mut self, monitor_sender: MonitorSender) -> Self {
        self.monitor_sender = Some(monitor_sender);
        self
    }
}
