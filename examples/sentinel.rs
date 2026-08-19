//! Connecting through Redis Sentinel.
//!
//! The client never names the master. It asks the Sentinels which instance
//! currently holds the service, connects there, and repeats the discovery after
//! a failover — which is the whole point: the master's address changes and the
//! application's configuration does not.
//!
//! ```sh
//! cargo run --example sentinel
//! ```
use rustis::{
    Result,
    client::{Client, Config, SentinelConfig, ServerConfig},
    commands::StringCommands,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // The short form: the Sentinel instances, then the service name.
    let client =
        Client::connect("redis+sentinel://127.0.0.1:26379,127.0.0.1:26380/mymaster").await?;
    client.set("sentinel_key", "value").await?;
    let value: String = client.get("sentinel_key").await?;
    println!("{value}");

    // Spelled out. The two knobs worth knowing bound the discovery: one round
    // tries every instance in turn, and a stale Sentinel announcing a
    // non-master instance would otherwise spin forever.
    let mut sentinel = SentinelConfig::default();
    sentinel.instances = vec![
        ("127.0.0.1".to_owned(), 26379),
        ("127.0.0.1".to_owned(), 26380),
    ];
    sentinel.service_name = "mymaster".to_owned();
    sentinel.wait_between_failures = Duration::from_millis(250);
    sentinel.max_discovery_rounds = 10;
    // The Sentinels are separate servers with ACLs of their own: these are not
    // the master's credentials.
    sentinel.username = None;
    sentinel.password = None;

    let mut config = Config::default();
    config.server = ServerConfig::Sentinel(sentinel);
    // This one is the master's.
    config.password = Some("master_password".to_owned());

    match Client::connect(config).await {
        Ok(client) => {
            let value: String = client.get("sentinel_key").await?;
            println!("{value}");
        }
        Err(e) => println!("no sentinel answered: {e}"),
    }

    Ok(())
}
