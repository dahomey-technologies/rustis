use crate::{Client, Result};

pub(crate) fn get_default_host() -> String {
    match std::env::var("REDIS_HOST") {
        Ok(host) => host,
        Err(_) => "127.0.0.1".to_string(),
    }
}

pub(crate) fn get_default_port() -> u16 {
    match std::env::var("REDIS_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => 6379,
    }
}

pub(crate) fn get_default_addr() -> String {
    format!("{}:{}", get_default_host(), get_default_port())
}

pub(crate) async fn get_test_client() -> Result<Client> {
    Client::connect(get_default_addr()).await    
}