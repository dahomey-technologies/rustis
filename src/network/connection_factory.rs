use crate::{Result, TcpStreamReader, TcpStreamWriter, tcp_connect};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct ConnectionFactory {
    connection_factory_impl: Arc<ConnectionFactoryImpl>,
}

impl ConnectionFactory {
    pub async fn initialize(addr: impl Into<String>) -> Result<ConnectionFactory> {
        ConnectionFactoryImpl::initialize(addr).await.map(|c| ConnectionFactory {
            connection_factory_impl: Arc::new(c)
        })
    }

    pub async fn get_connection(&self) -> Result<(TcpStreamReader, TcpStreamWriter)> {
        self.connection_factory_impl.get_connection().await
    }
}

struct ConnectionFactoryImpl {
    addr: String,
    first_connection: Mutex<Option<(TcpStreamReader, TcpStreamWriter)>>,
}

impl ConnectionFactoryImpl {
    pub async fn initialize(addr: impl Into<String>) -> Result<ConnectionFactoryImpl> {
        let addr: String = addr.into();
        let first_connection = Mutex::new(Some(tcp_connect(&addr).await?));

        Ok(ConnectionFactoryImpl {
            addr,
            first_connection,
        })
    }

    pub async fn get_connection(&self) -> Result<(TcpStreamReader, TcpStreamWriter)> {
        match self.get_first_connection() {
            Some(first_connection) => Ok(first_connection),
            None => tcp_connect(&self.addr).await,
        }
    }

    fn get_first_connection(&self) -> Option<(TcpStreamReader, TcpStreamWriter)> {
        let mut first_connection = self.first_connection.lock().unwrap();
        let first_connection = std::mem::replace(&mut *first_connection, None);
        first_connection
    }
}
