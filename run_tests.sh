#!/bin/sh
# Refuse to run against a deployment that is up but not usable.
#
# A Redis container answers PING the moment it starts, so `docker ps` reports
# every node healthy long before the cluster has formed — and a cluster whose
# nodes announce an address that is no longer this host's never forms at all.
# The suite does not fail against that: the cluster clients loop on reconnection
# and the tests hang, saying nothing about why. The CI gates its test step on
# `cluster_state:ok` for exactly this reason; this is that gate, locally.
#
# The check reports and steps aside when docker is absent or no container of this
# deployment is running, so a deployment set up by other means still runs the
# suite. What tells the two apart is whether *any* `redis-*` container is up: a
# deployment with some of its containers gone is broken, not absent, and is
# refused rather than waved through — a missing node makes its tests fail to
# connect while the suite still reports a total.
# The filter is not inspected, so the gate also stops a run that needs no server
# at all. `--hermetic` is the way to run those, and
# `RUSTIS_SKIP_DEPLOYMENT_CHECK=1` the way past the gate for anything else.

# `--hermetic` turns `server-tests` off, which leaves the tests that reach no
# server. They need no deployment check and no `--test-threads=1`: none of them
# shares a Redis, so they run in parallel, in about a second. `--tests` is what
# keeps the doctests out — every one of them opens a connection.
if [ "$1" = "--hermetic" ]; then
    shift
    exec cargo test --tests --no-default-features \
        --features tokio-runtime,tokio-rustls,pool,json,client-cache "$@"
fi

check_deployment() {
    [ -n "$RUSTIS_SKIP_DEPLOYMENT_CHECK" ] && return 0
    command -v docker >/dev/null 2>&1 || return 0

    running=$(docker ps --format '{{.Names}}' 2>/dev/null)
    echo "$running" | grep -q '^redis-' || return 0

    if ! echo "$running" | grep -qx redis-standalone; then
        echo "redis-standalone is not running: most of the suite has no server." >&2
        echo "A container outside this deployment may hold port 6379." >&2
        echo "Recreate it with:  cd redis && ./docker_down.sh && ./docker_up.sh" >&2
        return 1
    fi

    if ! echo "$running" | grep -qx redis-node1; then
        echo "The cluster nodes are not running, so every cluster test connects to" >&2
        echo "nothing while the suite still reports a total." >&2
        echo "Recreate the deployment with:  cd redis && ./docker_down.sh && ./docker_up.sh" >&2
        return 1
    fi

    docker exec redis-node1 redis-cli -p 7000 cluster info 2>/dev/null |
        grep -q "cluster_state:ok" && return 0

    announced=$(docker exec redis-node1 redis-cli -p 7000 cluster nodes 2>/dev/null |
        sed -n 's/^[^ ]* \([0-9.]*\):.*/\1/p' | head -1)
    echo "Node 7000 does not report cluster_state:ok, so the cluster tests would hang." >&2
    echo "  announced:      ${announced:-unknown}" >&2
    echo "  this host:      $(hostname -I 2>/dev/null)" >&2
    echo "The nodes announce the address that redis/.env held when they were created." >&2
    echo "If it is not one of this host's, the cluster bus cannot form: every node" >&2
    echo "sees only itself, and no slot is served." >&2
    echo "Recreate the deployment with:  cd redis && ./docker_down.sh && ./docker_up.sh" >&2
    echo "Starting the containers from a container app runs neither script, which is" >&2
    echo "what leaves redis/.env holding a stale address." >&2
    return 1
}

check_deployment || exit 1

cargo test --features tokio-rustls,pool,json,client-cache "$@" -- --test-threads=1
