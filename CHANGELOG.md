# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Versions up to and including `0.19.3` are documented in the
[GitHub releases](https://github.com/dahomey-technologies/rustis/releases).

## [Unreleased]

## [0.20.0] - 2026-07-26

This release closes a large correctness and performance pass over the RESP
layer, the network task, the cluster client and the client-side cache. It
contains breaking changes; read that section before upgrading.

### BREAKING CHANGES

- **Many public types are now `#[non_exhaustive]`, so that adding a variant or a
  field to them stops being a breaking change.** This is a one-time cost taken
  here, in a release that already breaks, rather than on each future addition.
  What changes for you:
  - Matching one of the affected enums now requires a `_ => …` arm.
  - Constructing one of the affected structs with a struct literal (including
    `Config { host, ..Default::default() }`) is no longer possible from outside
    the crate; use `Default::default()` and assign the fields, or the type's
    constructor.

  The types covered are the ones whose shape is dictated by something other than
  our own design: the error types (`Error`, `ClientError`, `RedisError`,
  `RedisErrorKind`, `RetryReason`), the configuration types (`Config`,
  `SentinelConfig`, `ClusterConfig`, `TlsConfig`, `ServerConfig`,
  `ReconnectionConfig`, `BufferConfig`, `RespLimits`), every command-option enum
  that follows Redis's own vocabulary (`SetCondition`, `BitOperation`,
  `ExpireOption`, `SortOrder`, `GeoUnit`, `FtLanguage`, `XTrimOperator`, … 60 in
  total), and every struct deserialized from a server reply (`ClusterInfo`,
  `ClientInfo`, `FtInfoResult`, `MemoryStats`, `SentinelMasterInfo`,
  `XStreamInfo`, … 71 in total), where Redis adds fields between versions.

  Deliberately **not** covered: `resp::Value` and the enums decoded from a server
  reply (`RoleResult`, `ReplicationState`, `RequestPolicy`, `ClusterState`, …).
  These describe the protocol and Redis's own closed vocabularies; matching them
  exhaustively is a legitimate thing to want, and a compile error on a new
  variant is information rather than a nuisance. The builder-style `*Options`
  structs are also untouched — their fields are already private, so they were
  never literal-constructible and gain nothing.
- `resp::Command::name()` now returns `&[u8]` instead of `Bytes`. It borrows from
  the command instead of bumping a reference count on every call. Callers that
  need an owned value can use `Bytes::copy_from_slice(command.name())`.
- `resp::FastPathCommandBuilder::arg` and `::key` are no longer public. They
  panicked on any non-primitive argument; they are now private, fallible, and
  every fast-path constructor falls back to the generic `cmd(NAME)` builder
  rather than panicking. Build commands through `resp::cmd` instead.
- `cache::Cache::zremrangebyscore` was removed. `ZREMRANGEBYSCORE` is a write
  command and had no place on the cached read surface; call it on the `Client`.
- `Error::Tls` now wraps `Arc<native_tls::Error>` instead of `native_tls::Error`,
  so `Error` stays `Clone` (a TLS failure has to be reported to every in-flight
  command). The `From<native_tls::Error>` conversion is gone.
- `Error::OneshotCanceled` now wraps `tokio::sync::oneshot::error::RecvError`
  instead of `futures::channel::oneshot::Canceled`, following the result channels
  moving to tokio.
- Variants were added to `ClientError` (`CrossSlot`, `InvalidConfig`,
  `MaxCommandAttemptsReached`, `MaxNestingDepthExceeded`, `BulkLengthTooLarge`,
  `CollectionLengthTooLarge`, `InconsistentRoutingState`, `InvalidCacheKey`,
  `UnexpectedSubscriptionConfirmation`), `SetCondition` (`IFNE`, `IFDEQ`) and
  `BitOperation` (`Diff`, `Diff1`, `AndOr`, `One`), and public fields to `Config`
  (`buffers`, `limits`, `max_command_attempts`, `max_messages_per_wave`),
  `SentinelConfig` (`max_discovery_rounds`) and `FtIndexAttribute` (`algorithm`,
  `data_type`, `dim`, `distance_metric`). All of these types are
  `#[non_exhaustive]` as of this release, so equivalent additions will not break
  again.
- `CommandBuilder::kill_connection_on_write` is no longer public. It is a
  failure-injection hook for the crate's own tests and is now gated behind
  `cfg(test)`, so it is absent from shipped builds instead of being part of the
  API.
- `Command`, `CommandBuilder`, `PreparedCommand`, `CommandArgsMut`,
  `SortOptions`, `MigrateOptions`, `JsonGetOptions` and `AclDryRunOptions` no
  longer implement `UnwindSafe` and `RefUnwindSafe`. `Command` now carries the
  deferred serialization error described below, and `Error` is not
  `RefUnwindSafe` (it holds an `Arc<std::io::Error>`, which can wrap a boxed
  `dyn Error`). This only affects code passing these types through
  `std::panic::catch_unwind`.
- **Behavior change** — an empty array now decodes to `Value::Array([])`, an
  empty map to `Value::Map({})` and an empty push to `Value::Push([])`, instead
  of all three collapsing to `Value::Null`. RESP's empty-versus-nil distinction
  is preserved: a nil reply (`_` / `*-1`) still decodes to `Value::Null`. Typed
  deserialization (`Vec<T>` → `[]`) is unaffected.
- **Behavior change** — an integer reply that does not fit the requested type is
  now an error instead of a silent truncation. `Integer(300)` deserialized as
  `u8` used to yield `44`; it now fails. `i64::MIN` is accepted (it was
  previously rejected). This applies to both the RESP deserializer and the
  `Value` deserializer, which stay consistent with each other.

### Security

- Passwords are no longer written in clear text by `Display for Config`. Both the
  main and the Sentinel credentials are masked as `:***@`, so a configuration
  logged at startup no longer leaks the password.
- The `native-tls` backends now request TLS 1.2 as their minimum version.
  `native-tls`'s own default allowed TLS 1.0.
- The RESP parser now bounds every quantity a server controls, so a hostile or
  malfunctioning peer cannot drive the client into a crash or an unbounded
  allocation: nesting depth is capped (a deeply nested reply used to overflow the
  stack), bulk and collection lengths are capped, and negative bulk, error and
  collection lengths are rejected rather than being used as sizes. `-1` remains
  the nil form. All limits are configurable through `Config::limits`.
- Decoding and logging server input no longer contain panic paths: the whole
  crate now denies the explicit-panic clippy family (`unwrap_used`,
  `expect_used`, `panic`, `unreachable`, `todo`, `unimplemented`), with
  `indexing_slicing` additionally denied in the `resp` and `network` modules,
  enforced in CI. A panic on the network task would take down every in-flight
  command along with the reconnection loop.

### Added

- Redis 8.4 command support: `FT.HYBRID` (including the advanced
  post-processing options), `CLUSTER SLOT-STATS`, `CLUSTER MIGRATION`, `DIGEST`
  and `DELEX`. Plus the options that were missing from various Redis 8.x
  commands, and `XADD`/`XTRIM`'s entries-deletion policy.
- `StreamCommands::xdelex` and `StreamCommands::xackdel` (Redis 8.2), which
  delete — and for `XACKDEL` acknowledge — stream entries under an explicit
  `StreamEntryDeletionPolicy`, and report per-id whether each entry was removed,
  was missing, or was kept because the policy forbade it.
- The examples now declare `tokio-runtime` in their `required-features`. They are
  written with `#[tokio::main]`, so `cargo test --no-default-features --features
  async-std-runtime,…` used to fail to build on them rather than run the suite;
  the whole suite now runs under async-std.
- `Config` now exposes the constants that were hardcoded, each defaulting to its
  previous value: `Config::buffers` (`BufferConfig` — read/write buffer initial
  and shrink-back capacities), `Config::limits` (`RespLimits` — maximum nesting
  depth, bulk length and collection length), `Config::max_messages_per_wave`,
  `Config::max_command_attempts` and `SentinelConfig::max_discovery_rounds`.
  `Config::validate()` runs at connection time and rejects a value that would
  disable a behavior.
- The parser accepts RESP3 attribute (`|`) and big number (`(`) frames.
- `RespResponse::compact()` copies a response's referenced bytes into
  freshly-sized buffers, releasing the larger recycled network block a retained
  response would otherwise pin. The client-side cache now compacts entries before
  storing them.
- `cargo-fuzz` targets over the RESP read path, and a `fuzz_api` module exposing
  the parser entry points they drive.

### Changed

- **RESP collection decoding now uses a flat parse tape.** A collection reply is
  parsed once into a sequence of fixed-width nodes (one per element, all nesting
  levels) held in a recycled buffer, and reading an element is an O(1) node
  lookup instead of re-parsing the collection from the start. This removes the
  double-parse that the previous 5-range frame cache fell back to beyond its
  fifth element, and makes descending into nested replies O(1) per subtree.
- **The streaming decoder now resumes across TCP chunks.** A reply split over
  several network reads is parsed incrementally — the partial tape and an
  explicit parse stack are carried forward — instead of re-parsing the whole
  accumulated buffer on each read. Decoding a large collection delivered in
  ~16 KB slices is now roughly on par with decoding it from a single slice
  (previously about 2.5× slower).
- **The parser no longer builds error values on the success path.** Its
  per-element (and per-digit) hot path used `Option::ok_or`, which eagerly
  constructs the (large) `Error` enum on every call and drops it again on
  success. Switching to `ok_or_else` builds an `Error` only when one is actually
  returned, cutting parse-and-deserialize time by roughly 15–30 % across reply
  shapes.
- The network task got a series of throughput reductions in per-command
  overhead: the message and result channels moved to tokio's `mpsc`/`oneshot`,
  `Message` shrank from 2536 to 288 bytes, replies are dispatched straight to the
  waiting caller as they decode rather than after the batch completes, the send
  wave is capped so reading is never starved by writing, and the TCP stream is
  split without a `BiLock`.
- Routing work stays on the caller thread and off the network task: key hash
  slots are computed lazily by the caller, `ArgLayout` shrank to 12 bytes, and
  the cluster client indexes `MGET` reordering and shard-key lookups by hash set
  instead of scanning.
- The read buffer is reserved from the announced bulk length, so a large reply no
  longer grows the buffer by repeated doubling, and oversized read/write buffers
  shrink back to their target instead of staying at their peak for the
  connection's lifetime.

### Fixed

- **Concurrent pipelines could return each other's replies.** `pending_responses`
  was shared across batches instead of being scoped to one, corrupting results
  under concurrent pipelined use.
- **Iterating a collection past its fifth element could yield corrupted values**
  (the fallback re-parser produced ranges against the wrong buffer base). The
  tape indexes every element uniformly, removing that path by construction.
- **A large reply arriving in many TCP segments was re-parsed from the start on
  every segment**, making decode cost quadratic in the number of segments. The
  decoder now keeps resume state, so the cost is linear in the reply size.
- Cluster: a redirection now retries only the sub-requests that were redirected
  rather than the whole split command; an `ASK` redirection to a node absent from
  the topology is followed; a per-shard failure surfaces as an error on that
  request instead of reconnecting the entire cluster; requests that can never be
  fulfilled are purged from the in-flight set instead of hanging; aggregates over
  shards of unequal length are rejected; and a transaction spanning several slots
  is refused before being sent rather than failing server-side.
- Reconnection: pub/sub bookkeeping is rebuilt when in-flight messages are
  replayed, in-flight unsubscriptions are dropped rather than resubscribed,
  non-retryable in-flight messages are purged, protocol decode errors trigger a
  reconnect instead of being swallowed, the reconnect delay is capped, and a
  per-message retry counter fails a message past `Config::max_command_attempts`
  instead of replaying it forever.
- Pub/sub: local subscription tracking is kept until the server confirms the
  unsubscribe, an undecodable event no longer terminates the push stream, and a
  subscription confirmation that does not match what was requested is surfaced as
  an error.
- Client cache: client-side tracking is re-armed and the cache purged on
  reconnection; the `MONITOR` parser is quote-aware; an insert racing an
  invalidation can no longer store a stale entry; and a zero-argument key
  returns an error instead of panicking.
- `command_timeout` now applies to `subscribe` and `monitor`.
- Pipelines: an empty pipeline resolves as an empty batch instead of surfacing an
  opaque channel-canceled error, and a single forgotten command has its response
  dropped as it would in a multi-command batch.
- Configuration URLs: IPv6 addresses and percent-encoded credentials are parsed
  correctly, `MOVED`/`ASK` addresses are split at the last colon (IPv6), and
  Sentinel discovery is bounded and resilient.
- Commands: `MSETEX` and `HPERSIST` send their mandatory count argument;
  `HSETEX` no longer panics on an odd field/value list; the `CLIENT REPLY SKIP`
  typo in command-kind detection is corrected; and a command-builder
  serialization error is deferred to send time instead of panicking during the
  build.
- `resp::Value`'s `Boolean` compares by value rather than by discriminant.
- Closing the last `Client` clone closes the connection race-free.
- A collection element that fails to parse mid-iteration now surfaces an error
  instead of silently truncating the iteration.
- `Debug` on a response renders the decoded reply rather than the internal tape.
