//! Connecting over TLS.
//!
//! The `rediss://` scheme turns TLS on with the platform's default trust anchors,
//! which is all a managed Redis with a publicly-signed certificate needs. A
//! private CA — the usual case for a self-hosted server — needs its root added
//! to the trust store, which is what the second half does.
//!
//! ```sh
//! cargo run --example tls --features tokio-rustls
//! ```
use rustis::{
    Result,
    client::{Client, Config, IntoConfig, TlsConfig},
    commands::StringCommands,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Public CA: the scheme is the whole configuration.
    match Client::connect("rediss://user:password@redis.example.com:6380").await {
        Ok(client) => {
            let value: String = client.get("tls_key").await?;
            println!("{value}");
        }
        Err(e) => println!("could not reach the TLS server: {e}"),
    }

    // Private CA: the root has to be trusted explicitly, otherwise the handshake
    // fails on an unknown issuer. A DER-encoded certificate here; a PEM one goes
    // through `rustls-pemfile` first.
    let mut roots = rustls::RootCertStore::empty();
    if let Ok(der) = std::fs::read("ca.der") {
        let _ = roots.add(rustls::pki_types::CertificateDer::from(der));
    }

    let mut config: Config = "rediss://redis.internal:6380".into_config()?;
    config.tls_config = Some(TlsConfig::new(Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )));

    match Client::connect(config).await {
        Ok(client) => {
            let value: String = client.get("tls_key").await?;
            println!("{value}");
        }
        Err(e) => println!("could not reach the internal TLS server: {e}"),
    }

    Ok(())
}
