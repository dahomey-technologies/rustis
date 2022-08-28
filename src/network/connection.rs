use crate::{ConnectionFactory, Message, MsgReceiver, MsgSender, Result};
use futures::{channel::mpsc, Future};

pub(crate) trait Connection {
    fn get_msg_sender(&self) -> &MsgSender;

    fn send(&self, message: Message) -> Result<()> {
        self.get_msg_sender().unbounded_send(message)?;
        Ok(())
    }
}

pub(crate) async fn connect<F, Fut>(
    connection_factory: &ConnectionFactory,
    mut network_loop: F,
) -> Result<MsgSender>
where
    F: Send + FnMut(ConnectionFactory, MsgReceiver) -> Fut + 'static,
    Fut: Send + Future<Output = Result<()>>,
{
    use crate::spawn;

    let (msg_sender, msg_receiver): (MsgSender, MsgReceiver) = mpsc::unbounded();

    let connection_factory = connection_factory.clone();

    spawn(async move {
        if let Err(e) = network_loop(connection_factory, msg_receiver).await {
            eprintln!("{}", e);
        }
    });

    Ok(msg_sender)
}
