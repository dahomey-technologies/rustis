//! What TLS costs, on the handshake and on every command after it.
//!
//! TLS is one of the crate's headline features and had no benchmark at all, so
//! nothing in the repo said whether the encrypted path costs a driver-side
//! micro-optimisation's worth of time or drowns it. Both halves are measured
//! because they answer different questions: `connect_*` is paid once per
//! connection and is where the socket options are set, `get_*` is paid per
//! command and is where the record layer shows up.
//!
//! The two servers are the deployment's plain 6379 and TLS 6380, reached the same
//! way, so the difference is the encryption and not the route.
//!
//! Requires the `redis/` deployment and its certificate.
//!
//! Run with:
//!   cargo bench --features bench,tokio-rustls --bench tls_round_trip

use criterion::{Criterion, criterion_group, criterion_main};
use rustis::{
    client::{Client, Config, IntoConfig},
    commands::StringCommands,
};
use std::{hint::black_box, io::BufReader, sync::Arc};
use tokio::runtime::Runtime;

/// The deployment's own CA. The server certificate it signed is not trusted by
/// anything else, so the bench has to carry it to get a handshake at all.
const CA_CERTIFICATE: &str = include_str!("../redis/certs/ca.crt");

const KEY: &str = "bench_tls_key";

/// `localhost`, not `127.0.0.1`: the deployment's certificate names the host,
/// and a TLS client checks the name it dialled against the one presented.
fn host() -> String {
    std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string())
}

fn plain_config() -> Config {
    format!("redis://{}:6379", host()).into_config().unwrap()
}

fn tls_config() -> Config {
    let mut config = format!("rediss://:pwd@{}:6380", host())
        .into_config()
        .unwrap();

    let tls_config = config
        .tls_config
        .as_mut()
        .expect("a `rediss://` URL carries a TLS configuration");

    let mut root_store = rustls::RootCertStore::empty();
    let mut reader = BufReader::new(CA_CERTIFICATE.as_bytes());
    for item in rustls_pemfile::read_all(&mut reader) {
        if let rustls_pemfile::Item::X509Certificate(cert_der) = item.unwrap() {
            root_store.add(cert_der).unwrap();
        }
    }

    tls_config.rustls_config = Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth(),
    );

    config
}

fn bench(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let (plain, tls) = rt.block_on(async {
        let plain = Client::connect(plain_config()).await.unwrap();
        let tls = Client::connect(tls_config()).await.unwrap();
        let _: () = plain.set(KEY, "value").await.unwrap();
        let _: () = tls.set(KEY, "value").await.unwrap();
        (plain, tls)
    });

    // Per command, on a connection already up: the record layer and nothing else.
    let mut group = c.benchmark_group("tls_round_trip");
    group.bench_function("get_plain", |b| {
        b.to_async(&rt).iter(|| async {
            let value: String = plain.get(KEY).await.unwrap();
            black_box(value);
        })
    });
    group.bench_function("get_tls", |b| {
        b.to_async(&rt).iter(|| async {
            let value: String = tls.get(KEY).await.unwrap();
            black_box(value);
        })
    });
    group.finish();

    // Per connection: the TCP handshake, then for TLS the certificate
    // verification and key exchange on top of it. A pool sized too small pays
    // this repeatedly, which is what makes the ratio worth knowing.
    let mut group = c.benchmark_group("tls_connect");
    group.bench_function("plain", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(Client::connect(plain_config()).await.unwrap());
        })
    });
    group.bench_function("tls", |b| {
        let config = tls_config();
        b.to_async(&rt).iter(|| async {
            black_box(Client::connect(config.clone()).await.unwrap());
        })
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
