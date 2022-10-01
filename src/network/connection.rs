use crate::{
    resp::{Command, CommandEncoder, Value, ValueDecoder},
    tcp_connect, Config, Result, TcpStreamReader, TcpStreamWriter,
};
#[cfg(feature = "tls")]
use crate::{tcp_tls_connect, TcpTlsStreamReader, TcpTlsStreamWriter};
use futures::{SinkExt, StreamExt};
use tokio_util::codec::{FramedRead, FramedWrite};

enum Streams {
    Tcp(
        FramedRead<TcpStreamReader, ValueDecoder>,
        FramedWrite<TcpStreamWriter, CommandEncoder>,
    ),
    #[cfg(feature = "tls")]
    TcpTls(
        FramedRead<TcpTlsStreamReader, ValueDecoder>,
        FramedWrite<TcpTlsStreamWriter, CommandEncoder>,
    ),
}

pub struct Connection {
    config: Config,
    streams: Streams,
}

impl Connection {
    pub async fn initialize(config: Config) -> Result<Self> {
        let streams = Self::connect(&config).await?;

        Ok(Self {
            config,
            streams,
        })
    }

    pub async fn write(&mut self, command: Command) -> Result<()> {
        println!("Sending {command:?}");
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = match &mut self.streams {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            #[cfg(feature = "tls")]
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        } {
            println!("Received result {value:?}");
            Some(value)
        } else {
            None
        }
    }

    pub(crate) async fn reconnect(&mut self) -> bool {
        match Self::connect(&self.config).await {
            Ok(streams) => {
                self.streams = streams;
                true
            }
            Err(e) => {
                println!("Failed to reconnect: {:?}", e);
                false
            }
        }

        // TODO improve reconnection strategy with multiple retries
    }

    async fn connect(config: &Config) -> Result<Streams> {
        #[cfg(feature = "tls")]
        if let Some(tls_config) = &config.tls_config {
            let (reader, writer) = tcp_tls_connect(&config.host, config.port, tls_config).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            let (reader, writer) = tcp_connect(&config.host, config.port).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::Tcp(framed_read, framed_write))
        }

        #[cfg(not(feature = "tls"))] {
            let (reader, writer) = tcp_connect(&config.host, config.port).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::Tcp(framed_read, framed_write))
        }
    }
}
