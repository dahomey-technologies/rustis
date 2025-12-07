use crate::{
    Result,
    client::{Client, Config, IntoConfig},
};
#[cfg(feature = "native-tls")]
use native_tls::Certificate;

/// copy-paste of the root certificate located at crt/certs/ca.crt
#[cfg(any(feature = "native-tls", feature = "rustls"))]
const ROOT_CERTIFICATE: &str = r#"
-----BEGIN CERTIFICATE-----
MIIEmjCCAwKgAwIBAgIRAMevjxGPA5ze+1QVT7rV7o8wDQYJKoZIhvcNAQELBQAw
ZTEeMBwGA1UEChMVbWtjZXJ0IGRldmVsb3BtZW50IENBMR0wGwYDVQQLDBRyb290
QExULVJLRDAyNzA1NTEwNjEkMCIGA1UEAwwbbWtjZXJ0IHJvb3RATFQtUktEMDI3
MDU1MTA2MB4XDTI1MDcwNTA5NDc0OVoXDTM1MDcwNTA5NDc0OVowZTEeMBwGA1UE
ChMVbWtjZXJ0IGRldmVsb3BtZW50IENBMR0wGwYDVQQLDBRyb290QExULVJLRDAy
NzA1NTEwNjEkMCIGA1UEAwwbbWtjZXJ0IHJvb3RATFQtUktEMDI3MDU1MTA2MIIB
ojANBgkqhkiG9w0BAQEFAAOCAY8AMIIBigKCAYEA0q7gJrQwX6sSO9dKmqLp09hP
tHNGaTdhYsc4PBP1Z0lroieGW1UmmsVlWOaCH4166y56qpa/tfXMbWUTiSrzeW9J
3grKS18HHDZNzXEsIsEmg66tDc9BKRoVv++XFd6OOxURa068t3AXVbpDCGOCfALV
yzLOAJDXhASQ4u/uXT0WvVzJWbbCliDXEuJDMZPYdP2K7ticU+KrtMNhps4xZHst
DhVW/me43JV8aTgUPEeD402igAKcXjQ42N4q1IZb4CUWNpL0tRY9EJmb1FCL5V8d
mroVnCTfUgoinXEZhJ3xC8LfQFUZW0+7xQXI2YVv/TYTuV1eHpIBqd2QCakVv9P/
HHzKL7pZ1BQKAibb0YHum2m0c3j5wszpjHl+cSbXlGTOdoqIEycIuMzO667RvdVT
H4o3B2nf52ChZCuy0zkHIJapSLSi3a4JYp7wMP7uoljNcbcJOUPrmphcYxbOkfmF
B4YNcpWI1EeCNUElYsEuj7zzpB8bwq4RoE9t2X9nAgMBAAGjRTBDMA4GA1UdDwEB
/wQEAwICBDASBgNVHRMBAf8ECDAGAQH/AgEAMB0GA1UdDgQWBBQXcq+Dji/6Xa3M
bGRThAkzfUOcVDANBgkqhkiG9w0BAQsFAAOCAYEAGZPX2hfDg9YAbGPTK6ZHDFPw
R6ZRdxDQ8zFa1HDrQUvwkd3NhiY2CYYLkusMI0Dh1ut/xmoNRI7v9OL9twixAprl
zPdCDc7oH9oYLOfFmUTWQ+Q8f8G2K97cZyc7WotMHiGDsafdVkZgEY4q6lyjmCM9
WE8XFsc29RPtTLubronyLx5smDghjuDsrXbf9W2w5itYtVTq1uW7m0Shz7+Dhq92
0WkCbI1XxqAe/UuiCQk3jUoQBvE5WfpaVf66Q+sA6MrZsTZ4Y9cvWj482LZ5mDX/
wqDY1BSLvTnYC0QIzj/W3e94anJ4rjaoxKfT7OEEPk8tkl8ZVsBpoeINfzOB28LE
0UmVTANz8G55Sv4FguSBh6LZ1yuxx4vn6zJUmZI+snMaza2vMi9IJHJi7GQh01TQ
WVysQ5r2H8HWTaTivATozaOhu0vgcLl524mQ+3KtQ5CM4d+gbWe4b5XxfxxMfG2K
Zz4JtMr3UAPczB+k+ei1v8o7sESoHoRoLvFVkFPp
-----END CERTIFICATE-----
"#;

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

#[cfg(any(feature = "native-tls", feature = "rustls"))]
pub(crate) fn get_default_tls_port() -> u16 {
    match std::env::var("REDIS_TLS_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => 6380,
    }
}

pub(crate) fn get_default_addr() -> String {
    format!("{}:{}", get_default_host(), get_default_port())
}

pub(crate) fn get_default_config() -> Result<Config> {
    get_default_addr().into_config()
}

pub(crate) async fn get_test_client_with_config(config: impl IntoConfig) -> Result<Client> {
    log_try_init();
    Client::connect(config).await
}

pub(crate) async fn get_test_client() -> Result<Client> {
    get_test_client_with_config(get_default_config()?).await
}

#[cfg(any(feature = "native-tls", feature = "rustls"))]
pub(crate) async fn get_tls_test_client() -> Result<Client> {
    log_try_init();

    let uri = format!(
        "rediss://:pwd@{}:{}",
        get_default_host(),
        get_default_tls_port()
    );

    let mut config = uri.into_config()?;

    #[cfg(feature = "native-tls")]
    if let Some(tls_config) = &mut config.tls_config {
        let root_cert = Certificate::from_pem(ROOT_CERTIFICATE.as_bytes())?;
        tls_config.root_certificates(vec![root_cert]);
        // non trusted cert for tests
        tls_config.danger_accept_invalid_certs(true);
    }

    #[cfg(feature = "rustls")]
    if let Some(tls_config) = &mut config.tls_config {
        use std::{io::BufReader, sync::Arc};

        let mut root_store = rustls::RootCertStore::empty();

        let mut reader = BufReader::new(ROOT_CERTIFICATE.as_bytes());

        for item in rustls_pemfile::read_all(&mut reader) {
            if let rustls_pemfile::Item::X509Certificate(cert_der) = item.unwrap() {
                root_store.add(cert_der)?;
            }
        }

        // let certs = rustls_pemfile::certs(&mut reader);
        // let certs = certs.into_iter().map(Certificate).collect::<Vec<_>>();
        // root_store.add_parsable_certificates(&certs);

        // let root_store =
        //     rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let rustls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        tls_config.rustls_config = Arc::new(rustls_config);
    }

    Client::connect(config).await
}

pub(crate) async fn get_sentinel_test_client() -> Result<Client> {
    log_try_init();
    let host = get_default_host();
    Client::connect(format!("redis://{host}:26379")).await
}

pub fn get_sentinel_master_test_uri() -> String {
    let host = get_default_host();
    format!("redis+sentinel://{host}:26379,{host}:26380,{host}:26381/myservice")
}

pub(crate) async fn get_sentinel_master_test_client() -> Result<Client> {
    log_try_init();
    Client::connect(get_sentinel_master_test_uri()).await
}

pub(crate) async fn get_cluster_test_client() -> Result<Client> {
    log_try_init();
    let host = get_default_host();
    Client::connect(format!(
        "redis+cluster://{host}:7000,{host}:7001,{host}:7002"
    ))
    .await
}

pub(crate) async fn get_cluster_test_client_with_command_timeout() -> Result<Client> {
    log_try_init();
    let host = get_default_host();
    Client::connect(format!(
        "redis+cluster://{host}:7000,{host}:7001,{host}:7002?command_timeout=2000"
    ))
    .await
}

pub fn log_try_init() {
    let _ = env_logger::builder()
        .format_target(false)
        .format_timestamp(None)
        .filter_level(log::LevelFilter::Debug)
        .target(env_logger::Target::Stdout)
        .is_test(true)
        .parse_default_env()
        .try_init();
}
