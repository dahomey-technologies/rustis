//! End-to-end cost of routing a command through a cluster, against a plain
//! connection to the same node.
//!
//! A cluster client does per command what a standalone one does not: hash the
//! keys to a slot, look the slot up in the topology, pick the node, file a
//! `RequestInfo` with one `SubRequest` per node reached, then match each node's
//! reply back to it. None of that was measured — `cluster_read_alloc` covers only
//! the per-wakeup fan-out allocation — so the routing path had no figure to
//! defend a change against.
//!
//! The `standalone` baseline is the same command on the same server without any
//! of it, so the difference is the routing and the bookkeeping. `mget_*` crosses
//! slots on purpose: that is the only shape that fans a command out over several
//! nodes and then reassembles the replies in the caller's key order.
//!
//! The baseline's address is asked of the cluster rather than assumed, and this
//! matters more than it looks: a cluster client re-targets every node to the
//! address the node announces, so a baseline left on `127.0.0.1` reaches the same
//! container by a different route. Under the `redis/` deployment that route is
//! Docker's userland proxy instead of a DNAT rule, worth ~20 µs on a ~55 µs round
//! trip — enough to make the routed client look *faster* than the plain one,
//! which is how the mistake announces itself.
//!
//! Requires the cluster of the `redis/` deployment (nodes on 7000-7002).
//!
//! Run with:
//!   cargo bench --features bench --bench cluster_routing

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rustis::{
    client::Client,
    commands::{ClusterCommands, StringCommands},
};
use std::hint::black_box;
use tokio::runtime::Runtime;

/// Keys chosen so that a slice of them spans several shards: `key{i}` hashes
/// spread over the whole slot range, which is what makes `mget` fan out.
const KEY_COUNT: usize = 100;

fn host() -> String {
    std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

async fn cluster_client() -> Client {
    let host = host();
    Client::connect(format!(
        "redis+cluster://{host}:7000,{host}:7001,{host}:7002"
    ))
    .await
    .unwrap()
}

/// The address the first node announces to the cluster, which is the one a
/// cluster client ends up talking to whatever address it was given to bootstrap.
async fn announced_address() -> String {
    let bootstrap = Client::connect(format!("{}:7000", host())).await.unwrap();
    let nodes: String = bootstrap.cluster_nodes().await.unwrap();

    nodes
        .lines()
        .find(|line| line.contains("myself"))
        .and_then(|line| line.split_whitespace().nth(1))
        // The field is `ip:port@bus-port`; only the client half is wanted.
        .and_then(|address| address.split('@').next())
        .expect("the node names itself in CLUSTER NODES")
        .to_string()
}

/// A plain connection to the first cluster node, plus a key that node owns.
///
/// The key is found by trying: a plain connection does not follow a `MOVED`, so
/// only a key whose slot this node serves can be read over it. Which slots that
/// is depends on how the deployment divided them, so it is asked rather than
/// assumed.
async fn standalone_client(keys: &[String]) -> (Client, String) {
    let client = Client::connect(announced_address().await).await.unwrap();

    for key in keys {
        if client.set(key, "value").await.is_ok() {
            return (client, key.clone());
        }
    }
    panic!(
        "no key among the {} tried is served by the first node",
        keys.len()
    );
}

async fn seed(client: &Client, keys: &[String]) {
    // One key at a time: a cluster `mset` would be split across shards, which is
    // the very path under test and has no business running during setup.
    for key in keys {
        let _: () = client.set(key, "value").await.unwrap();
    }
}

fn bench(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let keys: Vec<String> = (0..KEY_COUNT).map(|i| format!("key{i}")).collect();

    let (cluster, standalone, local_key) = rt.block_on(async {
        let cluster = cluster_client().await;
        seed(&cluster, &keys).await;
        let (standalone, local_key) = standalone_client(&keys).await;
        (cluster, standalone, local_key)
    });

    let mut group = c.benchmark_group("cluster_routing");

    // One key, one shard: the routing and bookkeeping a command pays even when
    // it reaches a single node, against the same command with none of it.
    group.bench_function("get_cluster", |b| {
        b.to_async(&rt).iter(|| async {
            let value: String = cluster.get(&local_key).await.unwrap();
            black_box(value);
        })
    });
    group.bench_function("get_standalone", |b| {
        b.to_async(&rt).iter(|| async {
            let value: String = standalone.get(&local_key).await.unwrap();
            black_box(value);
        })
    });

    // Several shards for one command: the split, then the reassembly that has to
    // hand the replies back in the order the caller named the keys.
    for &n in &[2usize, 10, 100] {
        group.bench_with_input(BenchmarkId::new("mget_cluster", n), &n, |b, &n| {
            let (cluster, keys) = (&cluster, &keys);
            b.to_async(&rt).iter(|| async move {
                let values: Vec<Option<String>> = cluster.mget(&keys[..n]).await.unwrap();
                black_box(values);
            })
        });
    }

    // Many keys, one shard: a hash tag pins them all to the same slot, so the
    // command is routed as a single sub-request. This is the shape where the
    // per-command key bookkeeping is largest while none of it is read back.
    let tagged: Vec<String> = (0..KEY_COUNT).map(|i| format!("{{tag}}key{i}")).collect();
    rt.block_on(seed(&cluster, &tagged));

    group.bench_function("mget_one_slot/100", |b| {
        let (cluster, tagged) = (&cluster, &tagged);
        b.to_async(&rt).iter(|| async move {
            let values: Vec<Option<String>> = cluster.mget(tagged.as_slice()).await.unwrap();
            black_box(values);
        })
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
