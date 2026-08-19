//! Connecting to a Redis Cluster.
//!
//! The nodes given here are seeds: the client asks one of them for the topology
//! and connects to every shard itself, so listing one reachable node is enough.
//! It then routes each command to the node owning its key's slot and follows
//! `MOVED`/`ASK` redirections on its own.
//!
//! ```sh
//! cargo run --example cluster
//! ```
use rustis::{
    Result,
    client::{Client, ClusterConfig, Config, ReadPreference, ServerConfig},
    commands::StringCommands,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // The short form. Every node of the cluster is discovered from these seeds.
    let client = Client::connect("redis+cluster://127.0.0.1:7000,127.0.0.1:7001").await?;
    client.set("cluster_key", "value").await?;
    let value: String = client.get("cluster_key").await?;
    println!("{value}");

    // The same thing spelled out, for the two knobs a URI reads but a config
    // makes obvious.
    let mut cluster = ClusterConfig::default();
    cluster.nodes = vec![("127.0.0.1".to_owned(), 7000)];
    // Reads go to the replicas of their shard, round-robin. A replica lags: this
    // trades read-your-writes for read throughput.
    cluster.read_preference = ReadPreference::PreferReplica;
    // Reload the topology on a timer, not only when a redirection corrects it.
    // Without this, a resharding touching no slot this client uses is never
    // noticed and a new node never connected to.
    cluster.topology_refresh_interval = Some(Duration::from_secs(30));

    let mut config = Config::default();
    config.server = ServerConfig::Cluster(cluster);
    let client = Client::connect(config).await?;

    // A multi-key command must stay inside one slot. `{...}` is a hash tag: only
    // what it encloses is hashed, so these two keys land on the same shard.
    client.set("{user:1}:name", "alice").await?;
    client.set("{user:1}:email", "alice@example.com").await?;
    let values: Vec<String> = client.mget(["{user:1}:name", "{user:1}:email"]).await?;
    println!("{values:?}");

    Ok(())
}
