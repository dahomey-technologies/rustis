use crate::{
    cmd,
    resp::{Array, ResultValueExt, Value},
    BitmapCommands, Command, CommandSend, Database, Error, Future, GenericCommands, GeoCommands,
    HashCommands, ListCommands, Result, ServerCommands, SetCommands, SortedSetCommands,
    StringCommands, ValueReceiver, ValueSender,
};
use futures::channel::oneshot;
use std::{collections::VecDeque, sync::Mutex};

pub struct Transaction {
    database: Database,
    command_queue: Mutex<VecDeque<Command>>,
    value_sender_queue: Mutex<VecDeque<ValueSender>>,
}

impl Transaction {
    pub fn new(database: Database) -> Self {
        Self {
            database,
            command_queue: Mutex::new(VecDeque::new()),
            value_sender_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn send<'a>(
        &'a self,
        command: Command,
    ) -> impl futures::Future<Output = Result<Value>> + 'a {
        let (value_sender, value_receiver): (ValueSender, ValueReceiver) = oneshot::channel();

        self.command_queue.lock().unwrap().push_back(command);
        self.value_sender_queue
            .lock()
            .unwrap()
            .push_back(value_sender);

        async fn await_for_result(value_receiver: ValueReceiver) -> Result<Value> {
            let value = value_receiver.await?;
            value.into_result()
        }

        await_for_result(value_receiver)
    }

    pub async fn execute(&self) -> Result<()> {
        let mut commands = self
            .command_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect::<VecDeque<_>>();
        let mut value_senders = self
            .value_sender_queue
            .lock()
            .unwrap()
            .drain(..)
            .collect::<VecDeque<_>>();

        self.database.send(cmd("MULTI")).await?;

        while let Some(command) = commands.pop_front() {
            self.database.send(command).await.into_result()?;
        }

        let result = self.database.send(cmd("EXEC")).await?;

        match result {
            Value::Array(Array::Vec(results)) => {
                for value in results.into_iter() {
                    match value_senders.pop_front() {
                        Some(value_sender) => {
                            let _ = value_sender.send(value.into());
                        }
                        None => {
                            return Err(Error::Internal("Unexpected transaction reply".to_owned()))
                        }
                    }
                }
            }
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => return Err(Error::Internal("Unexpected transaction reply".to_owned())),
        }

        Ok(())
    }
}

impl CommandSend for Transaction {
    fn send(&self, command: Command) -> Future<'_, Value> {
        Box::pin(self.send(command))
    }
}

impl BitmapCommands for Transaction {}
impl GenericCommands for Transaction {}
impl GeoCommands for Transaction {}
impl HashCommands for Transaction {}
impl ListCommands for Transaction {}
impl SetCommands for Transaction {}
impl SortedSetCommands for Transaction {}
impl ServerCommands for Transaction {}
impl StringCommands for Transaction {}
