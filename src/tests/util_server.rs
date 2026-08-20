//! The test helpers that reach a live Redis, and the ones only their callers
//! use.

use crate::{
    Result,
    client::{Client, Config, ExclusiveClient, IntoConfig},
    commands::{ClusterCommands, ClusterResetType},
    tests::{RECORDED, RECORDING, log_try_init},
};
#[cfg(feature = "native-tls")]
use native_tls::Certificate;
use std::sync::atomic::Ordering;

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

#[cfg(any(feature = "native-tls", feature = "rustls"))]
pub(crate) fn get_default_host() -> String {
    match std::env::var("REDIS_HOST") {
        Ok(host) => host,
        Err(_) => "localhost".to_string(),
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

pub(crate) fn get_default_tls_port() -> u16 {
    match std::env::var("REDIS_TLS_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => 6380,
    }
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

pub(crate) async fn get_exclusive_test_client_with_config(
    config: impl IntoConfig,
) -> Result<ExclusiveClient> {
    log_try_init();
    ExclusiveClient::connect(config).await
}

pub(crate) async fn get_exclusive_test_client() -> Result<ExclusiveClient> {
    get_exclusive_test_client_with_config(get_default_config()?).await
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

pub(crate) fn get_sentinel_master_test_uri() -> String {
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

/// The spare cluster nodes on 7006/7007, which belong to no cluster and are
/// read by nothing else in the suite. They exist so the topology mutators can
/// be *sent*: against the shared cluster above, moving a slot breaks every
/// other cluster test, which is what made these commands look untestable.
///
/// `node` is 1 or 2; two nodes because MEET, FORGET, REPLICATE and FAILOVER
/// each need a second one to name.
pub(crate) async fn get_spare_cluster_node_client(node: u8) -> Result<Client> {
    log_try_init();
    Client::connect(format!("{}:{}", get_default_host(), 7005 + u16::from(node))).await
}

/// Puts a spare node back to a node belonging to no cluster and owning no slot,
/// so a test never inherits what the one before it built.
pub(crate) async fn reset_spare_cluster_node(client: &Client) -> Result<()> {
    client.cluster_reset(ClusterResetType::Hard).await
}

/// The spare Sentinel on 26382, monitoring `spareservice` (master 6383,
/// replica 6384) with a quorum of one — a deployment the failover commands may
/// destroy without taking the shared sentinel tests down with it.
pub(crate) async fn get_spare_sentinel_test_client() -> Result<Client> {
    log_try_init();
    Client::connect(format!("redis://{}:26382", get_default_host())).await
}

pub(crate) const SPARE_SENTINEL_SERVICE: &str = "spareservice";

pub(crate) async fn get_cluster_test_client_with_command_timeout() -> Result<Client> {
    log_try_init();
    let host = get_default_host();
    Client::connect(format!(
        "redis+cluster://{host}:7000,{host}:7001,{host}:7002?command_timeout=2000"
    ))
    .await
}

/// Resident set size of the current process, in bytes, or `None` where it
/// cannot be read.
///
/// Reads field 2 of `/proc/self/statm` (resident pages) and scales it by the
/// page size. This measures *retention*, which is what a memory-growth claim is
/// about, unlike an allocation counter. Its noise floor is a few MiB, so it is
/// only ever compared against thresholds orders of magnitude above that.
/// Returning `None` off Linux lets a caller skip the check instead of failing on
/// a platform where the figure is unavailable.
pub(crate) fn resident_bytes() -> Option<usize> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: usize = statm.split_whitespace().nth(1)?.parse().ok()?;
    // 4 KiB is the page size on every platform this file exists on that the
    // suite runs against; reading it would need libc, which the crate does not
    // depend on.
    Some(resident_pages * 4096)
}

/// Collects the crate's log events for the lifetime of the guard.
///
/// The buffer is global, so a test that captures must be `#[serial]`.
pub(crate) struct LogCapture;

impl LogCapture {
    pub(crate) fn start() -> Self {
        log_try_init();
        RECORDED.lock().unwrap().clear();
        RECORDING.store(true, Ordering::SeqCst);
        Self
    }

    /// Every event recorded so far, oldest first.
    pub(crate) fn events(&self) -> Vec<(log::Level, String)> {
        RECORDED.lock().unwrap().clone()
    }
}

impl Drop for LogCapture {
    fn drop(&mut self) {
        RECORDING.store(false, Ordering::SeqCst);
    }
}
