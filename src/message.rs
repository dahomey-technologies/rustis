use crate::{Command, PubSubSender, ValueSender};

#[derive(Debug)]
pub(crate) struct Message {
    pub command: Command,
    pub database: usize,
    pub value_sender: Option<ValueSender>,
    pub pub_sub_sender: Option<PubSubSender>,
}

impl Message {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            database: 0,
            value_sender: None,
            pub_sub_sender: None,
        }
    }

    #[allow(dead_code)]
    pub fn database(mut self, database: usize) -> Self {
        self.database = database;
        self
    }

    #[allow(dead_code)]
    pub fn value_sender(mut self, value_sender: ValueSender) -> Self {
        self.value_sender = Some(value_sender);
        self
    }

    #[allow(dead_code)]
    pub fn pub_sub_sender(mut self, pub_sub_sender: PubSubSender) -> Self {
        self.pub_sub_sender = Some(pub_sub_sender);
        self
    }
}
