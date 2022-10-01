use crate::{
    resp::{Command, CommandEncoder, Value, ValueDecoder},
    tcp_connect, tcp_tls_connect, Config, Result, TcpStreamReader, TcpStreamWriter,
    TcpTlsStreamReader, TcpTlsStreamWriter,
};
use futures::{SinkExt, StreamExt};
use tokio_native_tls::native_tls::TlsConnector;
use tokio_util::codec::{FramedRead, FramedWrite};

enum Streams {
    Tcp(
        FramedRead<TcpStreamReader, ValueDecoder>,
        FramedWrite<TcpStreamWriter, CommandEncoder>,
    ),
    TcpTls(
        FramedRead<TcpTlsStreamReader, ValueDecoder>,
        FramedWrite<TcpTlsStreamWriter, CommandEncoder>,
    ),
}

pub struct Connection {
    host: String,
    port: u16,
    tls_connector: Option<TlsConnector>,
    streams: Streams,
}

impl Connection {
    pub async fn initialize(config: Config) -> Result<Self> {
        let host = config.host.clone();
        let port = config.port;
        let tls_connector = config
            .tls_config
            .map(|c| c.into_tls_connector())
            .transpose()?;

        let streams = Self::connect(&host, port, &tls_connector).await?;

        Ok(Self {
            host,
            port,
            tls_connector,
            streams,
        })
    }

    pub async fn write(&mut self, command: Command) -> Result<()> {
        println!("Sending {command:?}");
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        }
    }

    pub async fn read(&mut self) -> Option<Result<Value>> {
        if let Some(value) = match &mut self.streams {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        } {
            println!("Received result {value:?}");
            Some(value)
        } else {
            None
        }
    }

    pub(crate) async fn reconnect(&mut self) -> bool {
        match Self::connect(&self.host, self.port, &&self.tls_connector.clone()).await {
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

    async fn connect(
        host: &str,
        port: u16,
        tls_connector: &Option<TlsConnector>,
    ) -> Result<Streams> {
        if let Some(tls_connector) = tls_connector {
            let (reader, writer) = tcp_tls_connect(&host, port, tls_connector.clone()).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            let (reader, writer) = tcp_connect(&host, port).await?;
            let framed_read = FramedRead::new(reader, ValueDecoder);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::Tcp(framed_read, framed_write))
        }
    }
}
