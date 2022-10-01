#[cfg(feature = "tls")]
use crate::IntoConfig;
use crate::{Client, Result};
#[cfg(feature = "tls")]
use native_tls::Certificate;
use std::time::Duration;

/// copy-paste of the root certificate located at crt/certs/ca.crt
#[cfg(feature = "tls")]
const ROOT_CERTIFICATE: &str = r#"-----BEGIN CERTIFICATE-----
MIIFSzCCAzOgAwIBAgIULTp8cWRl326SijHSTdHpP0y/SkAwDQYJKoZIhvcNAQEL
BQAwNTETMBEGA1UECgwKUmVkaXMgVGVzdDEeMBwGA1UEAwwVQ2VydGlmaWNhdGUg
QXV0aG9yaXR5MB4XDTIyMTAwMTA4MjMyM1oXDTMyMDkyODA4MjMyM1owNTETMBEG
A1UECgwKUmVkaXMgVGVzdDEeMBwGA1UEAwwVQ2VydGlmaWNhdGUgQXV0aG9yaXR5
MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEApWm0kFW03v2s58VX3FI/
gdIo0l+aEdYXoSaic9xPWl5MV5+T7YQ7V9tck6whi4ceUtDAKa8QZBoiJ9gn/Lbr
e6ebZiJ7blBscEqKzXZk5URlHbxXlbfldHScnKNxluI5ApJ0sYov58R60klJNWeK
Wlz+Hn2ubN1IkXuClMJ59i0UZ+MlALpXzpSiW1gS7pT4gIkuCQfdWWwUNDNuFN57
/l9fU0VYQd/7AI9eJnV9ltTOaVyiL/uO3mueWBmM+AeeGaX0WRtctRcR2sNqDMIx
JW/bUziTRncI+HEGQd7Wf1+yi4fjajUzLj8omVvO7RBrCyq5RV9dpMEAgY4LIM8w
g+VlLDbOT52CY90ADbTfMDgiH5mg2Zt3l4xNiwno/Itkng93Hh/AbPomOGtJSYPr
CIrzhkfD6PXMXYzdXAJLzjCc5sOIBrFhDSIzkGOAxX0DZaxngimCytigw8c1KdIw
Z/j71rDjv6blleGJ6ZXBwtdQEG2clDSuVjBRuIIxe64/wMEe702MMC28Y97SZ3WV
JU4KaQoW5oVaoom9+hngCT6btpmT6adu0oC424bmSxUdB6/Kk+kgsD6SyaXt6VCf
PUpfipNwbS/GFoqevLSGjOsyrEl5nzF0VBdcg9TOodlqruVtNFSnShQh93hKmY1J
Mz62dg/LnnZ4+yO1ARZk+tcCAwEAAaNTMFEwHQYDVR0OBBYEFHr9VpODSEUfgO/W
sVebyT+YTcy9MB8GA1UdIwQYMBaAFHr9VpODSEUfgO/WsVebyT+YTcy9MA8GA1Ud
EwEB/wQFMAMBAf8wDQYJKoZIhvcNAQELBQADggIBABImtJJhwE2b0cNeSI+ng9oa
6PBXX5usNZQBuw3wvaLFephpbUH/HrWFJCbscubZ0wmt0UD6Ly7v32DJl505NFQQ
XMDAdApLRMHcbFcyqIIVkcSlOoRNlf5Dx9A2oqUzwI392OrDeYEF+a+9CcLGo6Fv
m8vii6Z1JS7TgGa9KqFErOgyvVv5xoSH31EVgezIoMgYWya2oiKZayYLNlr/eo3c
8DxEvJ2sdv8MI09lrwAZ61bx4aDYcpnDckVzvttdjGrunr1AyblCo6yhKUax1w5K
qLvLoVwuTFo4VFzMJMeIuRLm79hxhQIjAgIba8Cms0EPxcivVaWG5KvY8/oXLlP6
YRgmWvuA9UGvnrAvUw+eAZj1aFzLRGLXG3VxHUSJyhEV54dZCMWMfK0KOdyptbR3
7phJCeCGYS8/kJnCMAXU4NfiGWmRcTxkJTqgHC3txgzJQ4Izt8oeekJwlOJEN6R8
OCT4DeNGKy0bcAwaUT2n+b+OmQpaT/F7u2Hx/n0356QjVSoNTgmg6Bjsp5hNlX6i
I22ZhrayIRlXmMUmivMWBriz44yu6bo74EV6zHNvF1LYR7u2ajtdzlk1fHSE4OfM
+J8SNDwRYRFTxZPTK2Yf/PQtyl+xaWAHcT7NumXQcqOVxq9jfaurIOCWz4i4BIeK
bsPEVuonk6XLwUlSNI2W
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

#[cfg(feature = "tls")]
pub(crate) fn get_default_tls_port() -> u16 {
    match std::env::var("REDIS_TLS_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => 6380,
    }
}

pub(crate) fn get_default_addr() -> String {
    format!("{}:{}", get_default_host(), get_default_port())
}

pub(crate) async fn get_test_client() -> Result<Client> {
    Client::connect(get_default_addr()).await
}

#[cfg(feature = "tls")]
pub(crate) async fn get_tls_test_client() -> Result<Client> {
    let uri = format!(
        "rediss://:pwd@{}:{}",
        get_default_host(),
        get_default_tls_port()
    );

    let mut config = uri.into_config()?;

    if let Some(tls_config) = &mut config.tls_config {
        let root_cert = Certificate::from_pem(ROOT_CERTIFICATE.as_bytes())?;
        tls_config.root_certificates(vec![root_cert]);
        // non trusted cert for tests
        tls_config.danger_accept_invalid_certs(true);
    }

    Client::connect(config).await
}

#[allow(dead_code)]
#[cfg(feature = "tokio-runtime")]
pub(crate) async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[allow(dead_code)]
#[cfg(feature = "async-std-runtime")]
pub(crate) async fn sleep(duration: Duration) {
    async_std::task::sleep(duration).await;
}
