use crate::{
    resp::{Command, CommandEncoder, Value, ValueDecoder},
    tcp_connect, Config, Result, TcpStreamReader, TcpStreamWriter,
};
use futures::{SinkExt, StreamExt};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct Connection {
    addr: String,
    framed_read: FramedRead<TcpStreamReader, ValueDecoder>,
    framed_write: FramedWrite<TcpStreamWriter, CommandEncoder>,
}

impl Connection {
    pub async fn initialize(config: Config) -> Result<Self> {
        let addr = config.to_addr();
        let (reader, writer) = tcp_connect(&addr).await?;
        let framed_read = FramedRead::new(reader, ValueDecoder);
        let framed_write = FramedWrite::new(writer, CommandEncoder);

        Ok(Self {
            addr,
            framed_read,
            framed_write,
        })
    }

    pub async fn write(&mut self, command: Command) -> Result<()> {
        println!("Sending {command:?}");
        self.framed_write.send(command).await
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = self.framed_read.next().await {
            println!("Received result {value:?}");
            Some(value)
        } else {
            None
        }
    }

    pub(crate) async fn reconnect(&mut self) -> bool {
        match tcp_connect(&self.addr).await {
            Ok((reader, writer)) => {
                self.framed_read = FramedRead::new(reader, ValueDecoder);
                self.framed_write = FramedWrite::new(writer, CommandEncoder);
                true
            }
            Err(e) => {
                println!("Failed to reconnect: {:?}", e);
                false
            }
        }

        // TODO improve reconnection strategy with multiple retries
    }
}
