# Contributing to rustis

The rules below are the ones CI enforces or the code assumes. Nothing here is a
matter of taste, and where a rule exists because of a specific failure, the
failure is named.

## Running the tests

The suite has two halves, and which one you need depends on what you touched.

**Without a server.** `./run_tests.sh --hermetic` runs everything that reaches
neither a Redis nor the network: RESP parsing and deserialization, command
encoding, configuration parsing, the message queue, cluster topology arithmetic,
and the client stack over an in-memory pipe. About 470 tests in a second, with no
Docker and no deployment. Use it as the inner loop.

**With a server.** The rest needs a running deployment:

1. from `redis/`, run `docker_up.sh` (or `docker_up.cmd`)
2. run `./run_tests.sh`

`run_tests.sh` refuses to start against a deployment that is up but unusable — a
cluster whose nodes announce a stale address never forms, and the tests then hang
instead of failing. `RUSTIS_SKIP_DEPLOYMENT_CHECK=1` steps past the gate.

The server half **requires `--test-threads=1`**, which `run_tests.sh` passes:
those tests share one Redis instance and flush it, so running them in parallel
produces failures that belong to no test in particular.

## Where a new test goes

The two halves are separated by the `server-tests` feature, which is on by
default. The gate lives on the module list in `src/tests/mod.rs`, never on an
individual test:

* a module gated on `server-tests` needs a live Redis;
* a `<module>_server` module holds the server-bound tests of the module it is
  named after, whose own tests stay hermetic;
* everything ungated must pass with no server and no network.

Put a test that needs a Redis in the gated module. A test placed on the wrong
side is not silent: it fails `./run_tests.sh --hermetic`.

A hermetic test that still needs a server to talk to has one:
`src/tests/fake_server.rs` answers RESP3 over an in-memory pipe, and
`src/tests/fault_injection_proxy.rs` scripts a broken one.

## What CI checks

* `cargo fmt --all -- --check`, and `cargo check` in debug and release.
* `cargo clippy --all-targets -- -D warnings`, on the library feature set and
  again on the one that adds `bench` and `web-examples` — otherwise no job builds
  the benchmark targets or the gated examples, and they rot.
* The MSRV job compiles with exactly the declared toolchain.
* A feature matrix compiles each combination on its own, and a second matrix
  asserts that the rejected combinations still fail with the message they promise.
* `cargo semver-checks` reports the public-API breaks a pull request introduces.
  It never fails the job: the report is there so each break becomes a deliberate
  `CHANGELOG.md` entry rather than a discovery made after publishing.
* The four `cargo-fuzz` targets run weekly. They reach the parser through the
  `fuzzing` feature, which exposes the same kind of internal entry points as
  `bench` does through `resp::bench_support` and carries the same absence of a
  stability guarantee: `cd fuzz && cargo +nightly fuzz run <target>`.

**Never `--all-features`.** The two TLS runtimes are mutually exclusive and
enabling both is a compile error; the CI feature sets are what to reproduce
locally.

## Rules the code assumes

**Panics.** `unwrap`, `expect`, `panic`, `unreachable`, `todo`, `unimplemented`
and `arithmetic_side_effects` are denied crate-wide, and `indexing_slicing` is
denied in `resp/` and `network/`. A surviving site carries
`#[expect(…, reason = "…")]` naming the invariant that makes it unreachable —
`expect` rather than `allow`, so a justification whose lint stops firing becomes
a warning and gets deleted instead of rotting. Test code is exempt: a test that
panics is a test that failed.

**`#![forbid(unsafe_code)]`**, argued in `src/lib.rs` rather than assumed.

**MSRV.** Declared as `rust-version` in `Cargo.toml`. Raising it is a breaking
change and is announced in `CHANGELOG.md`.

**`CHANGELOG.md`.** Every user-visible change gets an entry, in the section it
belongs to. A breaking change also gets a line in the `BREAKING CHANGES`
checklist at the top of `[Unreleased]`, stating what a caller has to do.

## Benchmarks

From a running deployment, `cargo bench --features bench`. The feature is
required: every benchmark target declares `required-features = ["bench"]`, so a
plain `cargo bench` skips all of them and reports success having measured
nothing.

A measurement that compares two variants must alternate them inside each round
and compare per-round ratios. Two sequential series measure the machine's mood as
much as the code.

## Releasing

`Cargo.toml` holds the version; the tag confirms it. Publishing runs from a
GitHub release, and the workflow refuses a tag that disagrees with the manifest,
checks the version bump against `cargo semver-checks`, and builds the docs.rs
feature set and the native-tls backend before publishing — `cargo publish` builds
with default features only, so neither is covered otherwise.
