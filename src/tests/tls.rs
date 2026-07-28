#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::{Result, commands::StringCommands, tests::get_tls_test_client};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use serial_test::serial;

#[cfg(feature = "rustls")]
#[tokio::test]
#[serial]
async fn tls() -> Result<()> {
    let client = get_tls_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg(feature = "native-tls")]
#[tokio::test]
#[serial]
async fn tls() -> Result<()> {
    let client = get_tls_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg(feature = "native-tls")]
#[test]
fn native_tls_default_min_protocol_version() {
    use crate::client::TlsConfig;

    // The default native-tls minimum protocol version must be TLS 1.2:
    // TLS 1.0/1.1 are deprecated by RFC 8996.
    let config = TlsConfig::default();
    let debug = format!("{config:?}");
    assert!(
        debug.contains("min_protocol_version: Some(Tlsv12)"),
        "unexpected default min protocol version: {debug}"
    );
}
