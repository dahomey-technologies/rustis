use crate::{
    client::{Client, ClientPreparedCommand},
    commands::ConnectionCommands,
    network::PushReceiver,
    resp::{FromValue, Value},
    Error, Result,
};
use futures::{Stream, StreamExt};
use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

/// Stream to get [`MONITOR`](https://redis.io/commands/monitor/) command events
/// when the stream is dropped or closed, a reset command is sent to the Redis server
pub struct MonitorStream {
    closed: bool,
    receiver: PushReceiver,
    client: Client,
}

impl MonitorStream {
    pub(crate) fn new(receiver: PushReceiver, client: Client) -> Self {
        Self {
            closed: false,
            receiver,
            client,
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        self.client.reset().await?;
        self.closed = true;
        Ok(())
    }
}

impl Stream for MonitorStream {
    type Item = MonitoredCommandInfo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.closed {
            Poll::Ready(None)
        } else {
            match self.get_mut().receiver.poll_next_unpin(cx) {
                Poll::Ready(value) => match value {
                    Some(value) => match value {
                        Ok(value) => match value.into() {
                            Ok(str) => Poll::Ready(Some(str)),
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
}

impl Drop for MonitorStream {
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        let _result = self.client.reset().forget();
    }
}

/// Result for the [`monitor`](crate::commands::BlockingCommands::monitor) command.
#[derive(Debug)]
pub struct MonitoredCommandInfo {
    pub unix_timestamp_millis: f64,
    pub database: usize,
    pub server_addr: SocketAddr,
    pub command: String,
    pub command_args: Vec<String>,
}

impl FromValue for MonitoredCommandInfo {
    fn from_value(value: Value) -> Result<Self> {
        let line: String = value.into()?;
        let mut parts = line.split(' ');

        let info = match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(unix_timestamp_millis), Some(database), Some(server_addr), Some(command)) => {
                let database = &database[1..];
                let server_addr = &server_addr[..server_addr.len() - 1];
                match (
                    unix_timestamp_millis.parse::<f64>(),
                    server_addr.parse::<SocketAddr>(),
                    database.parse::<usize>(),
                ) {
                    (Ok(unix_timestamp_millis), Ok(server_addr), Ok(database)) => Some(Self {
                        unix_timestamp_millis,
                        database,
                        server_addr,
                        command: command[1..command.len() - 1].to_owned(),
                        command_args: parts.map(|a| a[1..a.len() - 1].to_owned()).collect(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        };

        info.ok_or_else(|| Error::Client(format!("Cannot parse result from MONITOR event: {line}")))
    }
}
