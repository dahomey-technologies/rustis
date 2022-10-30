use crate::{resp::{Command, Value}, Future};

pub trait SendBatch {
    fn send_batch(&mut self, commands: Vec<Command>) -> Future<Value>;
}