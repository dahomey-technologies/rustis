# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Versions up to and including `0.19.3` are documented in the
[GitHub releases](https://github.com/dahomey-technologies/rustis/releases).

## [Unreleased]

### Changed

- **A connection URI now rejects a query parameter it does not understand.** An
  unknown key (`?commandtimeout=5000`, `?reconnection=constant`) and a value that
  does not parse (`?command_timeout=5s`) were both dropped silently, leaving the
  default in place — a mistyped `command_timeout` meant no timeout at all. Both
  now fail with `ClientError::InvalidUri`, whose message names the parameter.
  Code relying on a URI with an unknown parameter being accepted must drop it.
  The documented `reconnection` parameter never existed and has been removed from
  the list; `max_command_attempts`, which is parsed, has been added to it.

- **`keep_alive` now defaults to 30 seconds** instead of `None`. Paired with the
  default `command_timeout` of 0 (no timeout), the previous default left a
  half-open connection — one silently dropped by a NAT, a firewall or a load
  balancer — detected by nothing: no timeout, no keepalive, no socket error, so
  every awaiting caller parked forever and `on_reconnect` never fired. Set
  `keep_alive` to `None`, or `keep_alive=0` in a URL, to restore the old
  behaviour.

- **Dependencies updated to their latest releases**, including two major bumps:
  `rand` 0.9 → 0.10 and `atoi` 2.0 → 3.1. Both are internal; the public API is
  unchanged. `serial_test` stays on 3.x — its 4.0 requires Rust 1.93, above the
  crate's 1.88 MSRV, which is itself unchanged.

### Fixed

- **The generic-command API documentation no longer teaches a Cluster misroute.**
  The `MSET`/`MGET` examples in the crate documentation and in `Client::send` added
  their keys with `arg`, which does not mark an argument as a key: the command
  carried no slot and was sent to a random node. They now use `key` and same-slot
  hash tags, and the rule is stated in the `cmd`, `arg` and `key` documentation.

- **`keep_alive` and `no_delay` are now applied to TLS connections.** Both were set
  only on the plain TCP path: a TLS connection ran with Nagle's algorithm enabled
  (up to 40 ms added to a small command) and without TCP keepalive, so a half-open
  socket was detected by nothing. Both paths now share a single socket-setup step
  applied to the `TcpStream` before the TLS handshake.

- **Vector-set commands are usable in pipelines and transactions again.**
  `VectorSetCommands` was implemented for `&Pipeline` and `&Transaction` instead of
  `&mut`, so `.queue()` and `.forget()` did not resolve on any of them and the whole
  family was unreachable in batch mode.

- **Building without any runtime feature now fails with a message that says so.**
  `default-features = false` without `tokio-runtime` left every function of the
  runtime layer without a body, and the user got nine `cannot find … in this scope`
  errors pointing at internal items. A `compile_error!` now names the missing
  feature and how to enable it.

- **The `actix_long_polling_pubsub` and `axum_long_polling_pubsub` examples compile
  again.** They passed `lpop`'s count as a `usize` where the signature takes a `u32`.

## [0.21.0] - 2026-07-30

### BREAKING CHANGES

- **`FtHybridVectorQuery::Knn` has a third field, `shard_k_ratio`.** The variant's
  fields are public and `#[non_exhaustive]` on the enum does not cover them, so an
  existing `Knn { k, ef_runtime }` literal needs `shard_k_ratio: None` added. That
  is the whole break; behaviour is unchanged when it is `None`.

### Added

- **`BackpressureConfig` on `Config` bounds the client's memory.** `max_queued_bytes`
  (16 MiB) caps the send queue, `max_pubsub_bytes` (8 MiB) each subscription,
  `max_push_bytes` (8 MiB) each push sink. Over budget, a stream drops its **oldest**
  messages; `0` restores the previous unbounded behaviour.

- **`dropped_messages()` on `PubSubStream`, `MonitorStream` and
  `ClientTrackingInvalidationStream`**, reporting what a budget shed.

- **`ClientError::SendQueueFull`**, returned when a command is offered to a send queue
  that is over budget. Commands already accepted are never shed.

### Changed

- **`max_command_attempts` defaults to `5` instead of unlimited**, so a command that
  keeps failing ends with `ClientError::MaxCommandAttemptsReached`. `0` still means
  unlimited.

- **`ReconnectionConfig::max_attempts` documents that a non-zero value is a one-way
  door**: reaching the limit ends the network task for good, so it is not recommended
  for long-running services. Behaviour and the `0` default are unchanged.

### Fixed

- **A cache entry whose invalidation was lost is no longer served stale.** The cache
  (feature `client-cache`) now flushes when invalidations are dropped, and a fetch in
  flight across a flush no longer re-inserts its stale value — which also fixes the
  pre-existing flush on reconnection.

- **`FT.HYBRID`'s `KNN SHARD_K_RATIO` is now reachable**, through
  `FtHybridVectorQuery::Knn::shard_k_ratio`. It is a cluster-only knob — it scales
  the candidate count each shard returns — and Redis 8.8 accepts it, where 8.6
  rejected it as an unknown argument. It counts towards the `KNN` clause count:
  `KNN 6 K 2 EF_RUNTIME 30 SHARD_K_RATIO 0.5`. See the breaking-changes section.

- **A struct no longer fails to decode when the server changes its field list.**
  Commands that still answer a flat array under RESP3 (`XINFO`, `BF.INFO`,
  `FT.INFO`, `XAUTOCLAIM`…) are decoded by guessing whether the array holds
  field/value pairs or positional values, and that guess used to key on the struct's
  field count — so one field added by a newer `redis-server` broke it. An array is
  now read as pairs when its length is even and its first element names a field,
  positionally otherwise, with both deserializers sharing the one rule. Added fields
  and appended elements are ignored; a field the server stops sending gives a serde
  error naming it. One consequence: an even-length positional array whose first
  element happens to equal a field name is now read as pairs.

- **Structs decode from cluster-aggregated and cache-rebuilt replies**, which are
  synthesized rather than parsed off the wire and previously failed with
  `CannotParseStruct`.

## [0.20.0] - 2026-07-28

This release closes a large correctness and performance pass over the RESP
layer, the network task, the cluster client and the client-side cache. It
contains breaking changes; read that section before upgrading.

### BREAKING CHANGES

- **`ClientTrackingOptions::redirect` is removed.** Redirection sends client-side
  caching invalidations to *another* connection. It exists for RESP2, which cannot
  deliver an invalidation on a connection that is also answering commands, so the
  target subscribes to `__redis__:invalidate` and reads them as pub/sub messages.

  rustis always negotiates **RESP3**, where invalidations arrive as push frames on
  the very connection that enabled tracking — which is what
  `Client::create_client_tracking_invalidation_stream` and `Cache` consume. Setting
  a redirection therefore sent them somewhere else and **silently starved both**:
  they stayed alive, reported no error, and never fired again. It also could not
  work on a cluster client at all, a client id being a per-node counter.

  If you were relying on it, you were not receiving invalidations. The reasoning is
  documented on `ClientTrackingOptions`.

- **The `async-std-runtime` and `async-std-native-tls` features are removed.**
  async-std is no longer maintained; its authors point users to `smol`. Tokio is
  now the only supported runtime, and the `async-std` and `async-native-tls`
  dependencies are gone from the tree. Enabling either feature is now a Cargo
  error, so the break is loud rather than silent. Nothing changes for the default
  `tokio-runtime` build.

  Support for another runtime is not ruled out: the runtime-facing surface is a
  handful of primitives in one module (connect, spawn, sleep, timeout, join
  handle). If you need a non-tokio runtime, open an issue.

- **`json_set` takes a `JsonSetOptions` instead of a bare `SetCondition`.**
  `JSON.SET` gained the `FPHA` argument in 8.8, so the last parameter had to
  become extensible. Calls passing `None` are unaffected; a call passing a
  condition becomes
  `json_set(k, p, v, JsonSetOptions::default().condition(SetCondition::NX))`,
  or keeps working as it is through `From<SetCondition> for JsonSetOptions`.

- **`XPendingMessageResult::elapsed_millis` is an `i64`, not a `u64`.** The
  server can answer `-1`, so the old type could not represent every reply. See
  the fixes section.

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
  `ExpireOption`, `SortOrder`, `GeoUnit`, `FtLanguage`, `XTrimOperator`, … 66 in
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

  Six of the 66 round-trip: `FtFieldType`, `FtIndexDataType`, `FtPhoneticMatcher`,
  `KeyType`, `TsAggregationType` and `TsDuplicatePolicy` are both written into a
  command and read back out of a reply. They are covered, because what a caller
  does with them is build an option — and Redis does extend them: this release
  alone adds `FtFieldType::Geoshape`, `TsAggregationType::CountNan` and
  `CountAll`.
- **`TsAggregationType`'s discriminants moved.** `CountNan` and `CountAll` were
  inserted before `First`, so every variant from `First` onwards shifted by two.
  Only code casting a variant to an integer (`aggregation as isize`) is affected;
  the wire form is the variant's name and is unchanged.
- **`ClientReplyMode` now implements `Copy`.** A non-`move` closure that used to
  take it by value now captures it by reference.
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
- `FtFlatVectorFieldAttributes::num_attributes` and
  `FtHnswVectorFieldAttributes::num_attributes` were removed, along with the
  unused `resp::SmallVecWithCounter`. The two `num_attributes` were
  hand-maintained mirrors of the fields their struct serializes, which a new
  field would have silently invalidated; the count now comes from the
  serialization itself.
- `CommandBuilder::kill_connection_on_write` is no longer public. It is a
  failure-injection hook for the crate's own tests and is now gated behind
  `cfg(test)`, so it is absent from shipped builds instead of being part of the
  API.
- **Five items that were `pub` without being usable are now private.** Each was
  reachable in name only: no caller outside the crate could construct the
  argument it needed or the type itself.
  - `resp::RespDeserializer`, together with `resp::EnumAccess` and
    `resp::VariantAccess`. `RespDeserializer::new` takes a `RespView`, which is
    crate-private, so the type had no reachable constructor; the other two are
    serde plumbing it hands to a visitor, and their names collided with serde's
    own `EnumAccess` / `VariantAccess` traits under `use rustis::resp::*`.
    Deserialize a reply through `Response` / `PreparedCommand` as before.
  - `commands::deserialize_bzop_min_max_result`, a `#[serde(deserialize_with)]`
    helper that a glob re-export made public.
  - `cache::Cache::from_builder`. Its `builder` parameter is a
    `moka::future::CacheBuilder` over the cache's internal representation, a
    private type alias, so the method could not be called from outside. Use
    `Cache::new`. Configuring the underlying moka cache is not currently
    expressible in the public API; if you need it, open an issue.
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
- **Behavior change** — a malformed numeric reply (`:a\r\n`, `,abc\r\n`) now fails
  the command that received it instead of tearing down the connection. RESP
  framing only needs the terminating `\r\n`, so the stream stays aligned and the
  other in-flight commands on the connection are unaffected. A malformed boolean
  (`#a\r\n`) still fails the connection: its frame length depends on the payload
  being `t` or `f`, so anything else leaves the frame boundary unknown.
- **Behavior change** — a numeric reply deserialized into a `String` now gives
  back the bytes the server sent rather than a re-rendering of the decoded value.
  A reply of `,12.50` used to come back as `"12.50"` → `f64` → `"12.5"`, and
  `,1e21` as twenty-two digits; both are now verbatim. Deserializing into a
  numeric type is unaffected.
- **Behavior change** — an integer reply that does not fit the requested type is
  now an error instead of a silent truncation. `Integer(300)` deserialized as
  `u8` used to yield `44`; it now fails. `i64::MIN` is accepted (it was
  previously rejected). This applies to both the RESP deserializer and the
  `Value` deserializer, which stay consistent with each other.
- Repairing the commands listed under *Fixed* changed five signatures:
  - `ClusterCommands::cluster_info` takes no argument (was `slot`, `count`).
  - `ClusterCommands::cluster_getkeysinslot` returns `R: Response` (was `()`).
  - `TimeSeriesCommands::ts_decrby` returns `u64` (was `()`).
  - `ClusterBumpEpochResult::Bumped` and `Still` now carry the config epoch the
    node ends up with (`Bumped(u64)` / `Still(u64)`), and the enum became
    `#[non_exhaustive]`. Match arms binding no field need updating; the new
    `ClusterBumpEpochResult::epoch()` reads the value whichever the outcome.
  - Eight methods lost a generic type parameter — `ClusterCommands::cluster_addslots`,
    `cluster_addslotsrange`, `cluster_count_failure_reports`, `cluster_delslots`,
    `cluster_delslotsrange`, `cluster_forget`, `SearchCommands::ft_profile_search`
    and `SentinelCommands::sentinel_failover`. Only a call site passing the
    parameter explicitly (`cluster_forget::<T>(…)`) needs changing.

- **Eight option builders changed shape because they could not express their
  command.** All eight are described in the fixes section; the API deltas are:
  - `HScanOptions::no_values` was removed, replaced by
    `HashCommands::hscan_no_values`, which returns `(u64, R)` and emits `NOVALUES`
    itself.
  - `SortOptions::store` was removed; use `sort_and_store`, which appends `STORE`
    itself.
  - `BfInfoResult::expansion_rate` is an `Option<usize>`, not a `usize`.
  - `TsInfoResult::key_self_name` is an `Option<String>`, not a `String`.
  - `FtSearchResultRow::score` is an `FtScore { value, explanation }`, not an
    `f64`.
  - `TsRangeOptions::bucket_timestamp` and `TsMRangeOptions::bucket_timestamp`
    take the new `TsBucketTimestamp` (`Low` / `High` / `Mid`), not a `u64`.
  - `TsRangeOptions::filter_by_ts` and `TsMRangeOptions::filter_by_ts` take
    `impl IntoIterator<Item = u64>`, not a single `&str`.
  - `FtSearchOptions::inkey` and `FtSearchOptions::infields` lost an
    unconstrained generic parameter, which had made them uncallable without a
    turbofish naming an unused type — so no working caller can break.
  - `RestoreOptions::frequency` takes a `u8`, not an `f64`. It emitted
    `FREQUENCY 10.0` where `RESTORE` takes `FREQ 10`, so every call failed; `u8`
    is the LFU counter's actual range.

- **`FtSearchResultRow::values` and `extra_attributes` hold an
  `FtAttributeValue`, not a `String`.** `FT.AGGREGATE`'s `TOLIST` and
  `RANDOM_SAMPLE` reducers return an array of strings per group, which a
  `Vec<(String, String)>` could not hold — both failed with
  `CannotParseString`.

  `FtAttributeValue` is `Text(String) | Array(Vec<String>)`. Read it with
  `as_str()` or `as_array()`, which return `Option`. `PartialEq` against `str`,
  `&str`, `String` and `[String]` is implemented in both directions, so
  `assert_eq!("40", value)` still compiles; code binding the value as a `String`
  needs `.as_str()` or a `match`.

- **`JsonRef` is removed; `Json<T>` serializes as well as it deserializes.** The
  JSON wrapper is now one name in both directions:
  `client.set(key, Json(&value))` and
  `let Json(value): Json<T> = client.get(key).await?`. `Json(&value)` borrows
  exactly as `JsonRef(&value)` did — `&T` is itself `Serialize` — so the
  migration is a rename, with `Json(value)` available when the value can be
  moved.

- **A value `serde_json` cannot encode now fails the command instead of being
  stored as an empty one.** The wrapper's `Serialize` swallowed the error and
  sent a unit argument in the value's place, so `client.set(key, Json(&value))`
  returned `Ok(())` having written an absent value under `key`. It now returns
  `Error::Client(ClientError::SerdeSerialize(_))` and sends nothing. Code that
  treated a JSON argument as infallible has an error to handle.

  Reading a nil reply as `Json<T>` also reports a different message — it now
  names `Option<Json<T>>`, which is what a possibly-missing key needs, instead
  of serde's `invalid type: Option`. Only the text changes.

- **`JsonArrIndexOptions::start` and `stop` are `isize`.** They were `u32` and
  `i32`, so a negative `start` — which `JSON.ARRINDEX` reads as an offset from
  the end of the array, like every other index in this family — could not be
  expressed. Literals still infer; a caller passing a `u32` or `i32` variable
  needs a cast.

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

- **`client::ClientTrackingInvalidationStream` is now exported.**
  `Client::create_client_tracking_invalidation_stream` returns it, but a
  `pub(crate)` re-export kept the name out of reach: the value could be used
  inline and never stored in a field, named in a signature or boxed.

- **Redis 8.8 support**, established the same way as the 8.6 pass below: the
  8.8 server's `COMMAND DOCS` diffed against a throwaway 8.6 server, so the
  delta is what the servers themselves disagree on rather than what a release
  note mentions.

  A new data type, with its own `ArrayCommands` trait: an **array** is
  sparse and index-addressed, so unlike a list it has no push or pop — you
  write at an index you choose (`arset`, `armset`) or at a cursor the array
  carries (`arinsert`, `arring`, moved with `arseek`). Gaps cost nothing, which
  is why `arlen` (highest index plus one) and `arcount` (slots that hold a
  value) never coincide. All eighteen commands are covered: `arcount`, `ardel`,
  `ardelrange`, `arget`, `argetrange`, `argrep`, `arinfo`, `arinsert`,
  `arlastitems`, `arlen`, `armget`, `armset`, `arnext`, `arop`, `arring`,
  `arscan`, `arseek`, `arset`, with `ArGrep`, `ArGrepPredicate`, `ArOperation`,
  `ArInfoOptions`, `ArLastItemsOptions` and `ArrayInfo`.

  Two more commands:
  - `increx` (+ `IncrExOptions`), a bounded increment that sets the expiration
    in the same atomic step. It returns both the new value and the increment
    that was actually applied, which is `0` when a bound stopped it. With
    `ubound_int` as the cap and `enx` to start the window only once, a window
    counter rate limiter no longer needs a Lua script.
  - `xnack` (+ `XNackMode`, `XNackOptions`), which releases pending messages
    back to the group's PEL without acknowledging them. The entries lose their
    owner and their idle time, so another consumer can claim them at once
    instead of waiting out `min-idle-time`.

  Arguments added to existing commands:
  - `ZAggregate::Count` on `zinter`, `zinterstore`, `zunion` and `zunionstore`
  - `FtFieldType::Geoshape` (+ `FtGeoShapeCoordSystem`) on `ft_create`
  - `JsonSetOptions::fpha` (+ `JsonFpType`) on `json_set`, which declares the
    storage type of a floating-point homogeneous array

- **Complete command coverage up to Redis 8.6**, established by diffing the
  server's own `COMMAND DOCS` against the crate rather than by reading release
  notes, and verified against a Redis 8.6.5 server.

  Redis 8.6 itself:
  - `HOTKEYS`: `hotkeys_start`, `hotkeys_stop`, `hotkeys_get`, `hotkeys_reset`,
    `hotkeys_help`, with `HotKeysMetric`, `HotKeysStartOptions` and
    `HotKeysInfo`. `HOTKEYS GET` replies with one entry per node; the fields
    tied to a metric are absent — not empty — when that metric was not tracked,
    so they are `Option`.
  - Stream idempotent production: `xcfgset` (+ `XCfgSetOptions`),
    `XAddOptions::idmp` and `XAddOptions::idmp_auto`, and the six IDMP fields on
    `XStreamInfo`.
  - `TsAggregationType::CountNan` and `CountAll`.

  Commands that pre-dated 8.6 and had been missed:
  - `xsetid` (+ `XSetIdOptions` for `ENTRIESADDED` / `MAXDELETEDID`)
  - `cluster_myshardid`
  - `module_loadex` (+ `ModuleLoadexOptions`) and `module_unload`
  - `json_merge`
  - `bf_card`, `topk_count`
  - `vismember`, `vrange`

  Arguments that existed on implemented commands but were not reachable:
  - `XClaimOptions::last_id` (`LASTID`)
  - `JsonGetOptions::format` with `JsonGetFormat::{String, Expand1, Expand}`
  - `FtFieldSchema::index_missing` / `index_empty`, and
    `FtCreateOptions::index_all` (+ `FtIndexAll`), which takes an explicit
    `ENABLE` / `DISABLE` value rather than being a flag

- **Rustis now emits [`tracing`](https://docs.rs/tracing) events and spans**
  instead of plain `log` records.

  **If you use `log`, nothing changes and you need to do nothing.** The `log`
  feature of `tracing` is enabled, so every event also emits a `log` record and
  existing `env_logger`-style setups keep receiving the same output.

  What you gain by installing a `tracing` subscriber instead: every event from
  the network task is wrapped in a `connection` span carrying a `tag` field, so
  output from several clients stays attributable; reconnections open a nested
  `reconnect` span grouping the in-flight purge, the retries and the
  subscription replay; and in cluster mode, events about a specific node carry a
  `node` field. Connection identity is no longer duplicated into each message —
  it is a structured field a collector can index.

- **A declared minimum supported Rust version: 1.88.** `Cargo.toml` now carries
  `rust-version`, and a CI job compiles both runtimes with exactly that
  toolchain, so the number is verified rather than asserted. Let chains hold the
  floor there; edition 2024 on its own would allow 1.85. Raising the MSRV will be
  treated as a breaking change and announced here.

- Redis 8.4 command support: `FT.HYBRID` (including the advanced
  post-processing options), `CLUSTER SLOT-STATS`, `CLUSTER MIGRATION`, `DIGEST`
  and `DELEX`. Plus the options that were missing from various Redis 8.x
  commands, and `XADD`/`XTRIM`'s entries-deletion policy.
- `StreamCommands::xdelex` and `StreamCommands::xackdel` (Redis 8.2), which
  delete — and for `XACKDEL` acknowledge — stream entries under an explicit
  `StreamEntryDeletionPolicy`, and report per-id whether each entry was removed,
  was missing, or was kept because the policy forbade it.
- `resp::CommandBuilder::arg_counted`, which writes a labeled clause followed by
  the number of arguments it contains — the shape `SORTBY 2 field ASC` and
  `PARAMS 4 n1 v1 n2 v2` require. The count comes from a dry run of the same
  serialization, so it cannot drift from what is written.
- `resp::CommandBuilder::arg_with_count_and_step` and
  `resp::CommandBuilder::key_with_count_and_step`, which prefix a collection with
  the number of `step`-sized groups it holds rather than its raw argument count —
  the shapes `HSETEX key FIELDS numfields field value …` and `MSETEX numkeys key
  value …` require. The `key_` form additionally marks every `step`-th element as
  a routing key for the cluster client. Both derive their count the same way
  `arg_counted` does.
- The examples now declare `tokio-runtime` in their `required-features`, so a
  feature set that does not build them no longer fails the whole target set.
- `Config` now exposes the constants that were hardcoded, each defaulting to its
  previous value: `Config::buffers` (`BufferConfig` — read/write buffer initial
  and shrink-back capacities), `Config::limits` (`RespLimits` — maximum nesting
  depth, bulk length and collection length), `Config::max_messages_per_wave`,
  `Config::max_command_attempts` and `SentinelConfig::max_discovery_rounds`.
  `Config::validate()` runs at connection time and rejects a value that would
  disable a behavior. `RespLimits::DEFAULT` and `BufferConfig::DEFAULT` give the
  same defaults in a `const` context.
- The parser accepts RESP3 attribute (`|`) and big number (`(`) frames.
- The client-side cache now compacts an entry before storing it: the response's
  bytes are copied into freshly-sized buffers instead of pinning the larger
  recycled network block they were read from. A numeric reply is decoded on the
  spot rather than copied, so a cache entry read a thousand times is decoded
  once.
- `cargo-fuzz` targets over the RESP read path, and a `fuzz_api` module exposing
  the parser entry points they drive.
- `resp::bench_support`, the benchmark counterpart to `fuzz_api`: thin entry
  points into the decode-and-deserialize path, isolated from the network, so an
  external `benches/*.rs` crate can measure the parser on hand-built RESP
  buffers. Gated behind the `bench` feature and compiled out of shipped builds.

### Changed

- Three `if let` guards in the `Value` deserializer were rewritten as plain
  matches. They were the only construct in the crate requiring Rust 1.95, so
  removing them lowered the compiler floor by seven releases. Behavior is
  unchanged.
- Documentation only, no API change:
  - `BlockingCommands` and each of its methods now state prominently that a
    blocking command monopolizes its connection, that `command_timeout` bounds
    only the caller's wait and does not free the connection server-side, and
    that these commands belong on a dedicated client. The constraint was
    documented before, but only in the client module's *Limitations* paragraph.
  - The README gained a *Safety* section explaining `#![forbid(unsafe_code)]` as
    a deliberate position — what it costs on a length-delimited protocol, and
    what actually guards the hostile-input surface instead (the panic lint
    policy, the configurable RESP limits, the fuzz targets).
  - The `check_resp2_array` heuristic in the `Value` deserializer is now
    documented, including where it is looser than the equivalent rule in the
    RESP deserializer.
  - The crate-level documentation claimed command coverage "until Redis 8.0";
    it is 8.4, as the README already said.

- **RESP collection decoding now uses a flat parse tape.** A collection reply is
  parsed once into a sequence of fixed-width nodes (one per element, all nesting
  levels) held in a recycled buffer, and reading an element is an O(1) node
  lookup instead of re-parsing the collection from the start. This removes the
  double-parse that the previous 5-range frame cache fell back to beyond its
  fifth element, and makes descending into nested replies O(1) per subtree.
  Walking a collection allocates nothing per element, which makes a large reply
  about 10 % faster to decode end to end.
- **A reply's value is produced when it is read, not when it is received.** The
  parser frames — it finds where a reply ends and indexes a collection's elements
  — and the value itself is decoded by whichever task asks for it. A connection's
  network task is shared by all of its callers, so the arithmetic of an integer or
  a double reply no longer runs there. End-to-end throughput is unchanged; what
  changes is where the work happens, not how much of it there is.
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
- **The network task was optimized for throughput**, cutting per-command
  overhead: the message and result channels moved to tokio's `mpsc`/`oneshot`,
  `Message` shrank from 2536 to 232 bytes, replies are dispatched straight to the
  waiting caller as they decode rather than after the batch completes, the send
  wave is capped so reading is never starved by writing, and the TCP stream is
  split without a `BiLock`. Measured against `0.19.3` on a single connection to a
  local server: **+65 % ops/s at 64 concurrent tasks, +111 % at 256 and +100 % at
  1024**, and −15 % wall-time on the latency-bound multiplexer benchmark. Below 32
  tasks the workload is round-trip-bound and unchanged.
- Routing work stays on the caller thread and off the network task: key hash
  slots are computed lazily by the caller, `ArgLayout` shrank to 12 bytes, and
  the cluster client indexes `MGET` reordering and shard-key lookups by hash set
  instead of scanning.
- The read buffer is reserved from the announced bulk length, so a large reply no
  longer grows the buffer by repeated doubling, and oversized read/write buffers
  shrink back to their target instead of staying at their peak for the
  connection's lifetime.

### Fixed

- **Connection-state commands now reach every node of a cluster instead of one
  random node.** A cluster client is one connection per node, and this state lives
  on the connection. Only `CLIENT SETNAME` and `CLIENT SETINFO` declared a routing
  policy; `AUTH`, `CLIENT TRACKING`, `CLIENT NO-EVICT`, `CLIENT NO-TOUCH` and
  `RESET` carried none, so each landed on an arbitrary shard and left the others
  in their previous state.

  The worst consequence was **client-side caching on a cluster client: it served
  stale values indefinitely.** Tracking armed on one node means the keys held by
  every other shard are cached and never invalidated, with no error and no log.

  A node that joins the topology later — through a `MOVED`-triggered refresh, or
  as a replica connected on demand — is now brought up to the same state before
  anything is sent on it, so a resharding no longer opts a shard out silently.

- **`CLIENT REPLY` now works on a cluster client instead of hanging it.** A cluster
  connection matches each reply against the sub-request it filed for the node it was
  sent to, so a silenced node used to leave that sub-request unresolvable and every
  caller queued behind it waited forever.

  `ON` and `OFF` are sent to every node — the mode has to be the same everywhere —
  and no in-flight bookkeeping is filed while the nodes are silent. This makes the
  use cases the Redis documentation names available to cluster clients too:
  fire-and-forget bursts, mass loading, constantly streamed cache writes.

  `SKIP` silences only the next command, so it is emitted on exactly the nodes that
  command is routed to — one for a key-routed command, every touched shard for a
  multi-shard one, all of them for a broadcast one. Previously it reached a single
  arbitrary node, which shifted the responses of everything that followed.

  `SELECT` and `READONLY` are unchanged and now document why: a cluster has one
  database, and rustis does not route slot reads to replicas.

- **A reconnection now restores the state the caller had attached to the
  connection, not only what the config describes.** Redis keeps a good deal of
  state on the connection itself, and a new socket starts without any of it. The
  handshake only ever restored `HELLO`, the config credentials, the config
  connection name and the config database, so a `SELECT`, an `AUTH`, a
  `CLIENT SETNAME`, a `CLIENT SETINFO`, a `CLIENT NO-EVICT` / `NO-TOUCH`, a
  `CLIENT TRACKING` issued directly or a `SCRIPT DEBUG` issued at
  runtime was silently lost the first time the connection blipped. Two of those
  are silent and damaging: the connection came back reading **database 0** and
  authenticated as **whoever the config names**, while answering normally.

  Each of these is now replayed as the command the caller actually issued, so
  option-carrying commands such as `CLIENT TRACKING` come back exactly as they
  were sent. A replay the server rejects is logged and dropped rather than
  failing the connection, so one bad `AUTH` cannot turn into a permanently dead
  client. `READONLY` is deliberately not among them: a cluster reconnection
  redials every node, so replaying it would grant the whole cluster a capability
  the send path never grants.

- **`CLIENT REPLY OFF` no longer desynchronizes every response after a
  reconnection.** The client mirrors the reply mode to know how many responses
  each command produces, and that mirror did not follow the socket: a
  reconnection taken while the connection was silent left the client expecting
  nothing while the server answered everything, shifting every subsequent
  response by one — permanently.

- **`CLIENT REPLY SKIP` silences one command again, instead of the connection.**
  It was treated as a sticky `OFF` until an explicit `ON`, so `SKIP` followed by
  more than one command made the client expect fewer replies than the server
  sends. `SKIP` before exactly one command — the only case the suite covered —
  behaved correctly and hid this.

- **`RESET` now clears the client's picture of the connection too.** It was
  recognised for a single one of its effects (leaving `MONITOR`). The reply mode
  stayed wrong afterwards, and a later reconnection restored subscriptions the
  caller had explicitly discarded.

- **A pooled client whose network task has ended is dropped from the pool.**
  `PooledClientManager::has_broken` returned `false` unconditionally, so a client
  that could no longer answer anything stayed in circulation and was handed to
  every subsequent borrower. Note that the pool still does **not** reset
  connection state between borrows — it hands out a multiplexed client, not a
  fresh connection; this is now documented on `PooledClientManager`, and callers
  needing a clean slate should issue `RESET` themselves.

- **Eighty-six option builders that nothing called got a test, and seventeen of
  them were broken.** Counting `*Options` / `*Schema` / `*Attribute` builder
  methods rather than commands shows that a tested command says nothing about its
  options: `ft_search` had a test all along and eleven of its options had never
  been sent once. Each of the 86 now has one, written from the syntax the server
  prints for itself and red before green. Thirteen of the seventeen defects were
  options that could not work in any call at all.

  Wrong token or wrong argument shape:
  - `ClientKillOptions::user` emitted `USERNAME` instead of `USER`, so any use
    answered `ERR syntax error`.
  - `ClientKillOptions::addr` and `laddr` sent `ADDR ip port` as two arguments
    where the server wants a single `ip:port`. Same error.
  - `ZAddOptions::change` emitted `CHANGE` instead of `CH`.
  - `FtSearchOptions::inkey` and `infields` suppressed their own field name
    through `#[serde(rename = "")]`, so the count and values went out with no
    `INKEYS` / `INFIELDS` token.
  - `FtAggregateOptions::load_all` emitted `LOAD 1 *`; `LOAD *` carries no count,
    and the counted form loads a field named `*`, which is to say nothing.
  - `FtAggregateOptions::add_scores` was serialized after `LOAD`, and the server
    accepts `ADDSCORES` only before it.

  Options no signature could express:
  - `TsGroupByOptions` had no `Default`, so `ts_mrange` and `ts_mrevrange` forced
    a `GROUPBY` the server makes optional — and grouping renames every returned
    series. It now defaults to emitting nothing.
  - `FtSearchSummarizeOptions` had neither `Default` nor a constructor, so
    `FtSearchOptions::summarize` had no argument that could be built.

  Replies the declared type could not hold:
  - `client_list` returned one phantom `ClientInfo` with `id: 0` on every call:
    the reply is newline-terminated and the trailing empty line was parsed as a
    client.
  - `ts_info(key, false)` always failed with `missing field keySelfName`; the
    existing test only ever passed `true`.
  - `bf_info_all` always failed on a `NONSCALING` filter — the filters the two
    `nonscaling()` options exist to create.
  - `FT.SEARCH ... EXPLAINSCORE` always failed with `CannotParseDouble`, so the
    option's own output was unreachable.

  See `RUSTIS_AUDIT.md` §3.2 for the full table and what the pass says about
  reviewing builders by reading them.

- **All 362 option builders in the commands layer are now covered by a test**,
  including the ones on schemas, reducers and query clauses (`FtReducer`,
  `FtSortBy`, `FtHnswVectorFieldAttributes`, `FtHybridSearch`, `FtHybridVsim`,
  `ArGrep`, `GeoSearchFrom`). Two were broken:
  - `FtReducer::first_value_by` and `first_value_by_order` never emitted the
    `BY` keyword: they sent `FIRST_VALUE 2 @name @age` and
    `FIRST_VALUE 3 @name @age DESC`, both rejected with
    `Unknown argument @age at position 1 for FIRST_VALUE`. Now
    `FIRST_VALUE 3 @name BY @age` and `FIRST_VALUE 4 @name BY @age DESC`.
  - `RestoreOptions::frequency` emitted `FREQUENCY 10.0` where `RESTORE` takes
    `FREQ 10`, so every call answered `syntax error`. See the breaking-changes
    section.

  `FT.HYBRID`'s per-clause `YIELD_SCORE_AS` is confirmed working on Redis 8.8,
  on both the `SEARCH` and the `VSIM` clause.
- **`ZAggregate` was never introduced by its `AGGREGATE` token.** `zinter`,
  `zinterstore`, `zunion` and `zunionstore` appended the bare value, so
  `zinter(keys, None, ZAggregate::Sum)` sent `ZINTER 2 k1 k2 SUM` and failed
  with `syntax error`. The whole enum was therefore unusable, and no test
  passed one — the same blind spot as the two token bugs below. Found while
  adding `ZAggregate::Count`.

- **`XPendingMessageResult::elapsed_millis` could not hold the value the server
  sends.** It was a `u64`, but `XPENDING` answers `-1` for an entry with no
  delivery to measure from — the state `xnack` puts entries in — which failed
  to deserialize. It is now an `i64`. See the breaking-changes section.

- **Two option builders emitted a token the server rejects, so the options were
  unusable.** Both came from a `rename_all` rule silently producing the wrong
  spelling, and neither had a call site anywhere in the crate — which is why no
  test caught them:
  - `ZRangeOptions::reverse()` emitted `REVERSE` instead of `REV`, so any
    `zrange` or `zrangestore` using it failed with `syntax error`.
  - `VSimOptions::with_attributes()` emitted `WITHATTRIBUTES` instead of
    `WITHATTRIBS`, so any `vsim` requesting attributes failed the same way.

  Both now have a live integration test and a wire-form test pinning the exact
  token.

- **A cluster client deadlocked after any subscription.** A subscription is
  acknowledged by a push frame, which the cluster connection hands straight to
  the network task instead of filing it as the answer to the request it sent.
  That request stayed at the head of the pending queue forever, and since
  replies are reported in order, the first reply coming from any other node
  waited behind it — the connection stopped answering entirely. The
  acknowledgement now retires the request; an error reply such as `MOVED` is
  still filed as a result, so redirections keep working.
- **`spublish` was sent to an arbitrary node.** Its shard channel was passed as
  a plain argument rather than as a key, so the cluster client could not route
  it and relied on the server's `MOVED` to find the shard — one useless round
  trip per call, on the path that then hit the deadlock above. `ssubscribe` and
  `sunsubscribe` already routed by slot.
- Eleven broken links in the published documentation, which rendered as dead
  text on docs.rs: `resp::Args` (a trait that does not exist — arguments are any
  `serde::Serialize`), `Command::arg` (it is `CommandBuilder::arg`),
  `Client::send_batch` (removed from the public API, still linked from three
  places — use `Pipeline`), `FtAggregateOptions::reduce` (it is on `FtGroupBy`),
  `FtSugGetOptions::withpayload` (plural), `TsMGetOptions::selected_labels` (the
  method is `selected_label`), `Command::compute_slots` (private), a bare
  `Serialize`, and a doubled parenthesis swallowing a URL. `cargo doc` now runs
  in CI with warnings denied, so these cannot come back unnoticed.

- **`zadd_incr` never incremented anything.** It omitted the `INCR` keyword, so
  it sent a plain `ZADD` and answered the number of elements added — `Some(0.0)`
  for an existing member — instead of the member's new score.
- **`vlinks_with_score` never asked for the scores.** It emitted the same
  `VLINKS` command as `vlinks`, so it answered bare neighbour names and any
  attempt to deserialize the scores failed.
- **`cluster_info` could not succeed.** It sent two arguments `CLUSTER INFO` does
  not accept, and `ClusterInfo` was derived as if the reply were a map when the
  server answers a text blob of `field:value` lines. The type now parses that
  text, tolerating both the counters a server omits and the fields a newer one
  adds.
- **`cluster_getkeysinslot` and `ts_decrby` threw their reply away.** Both were
  typed `()` where the server does answer something — the names of the keys in
  the slot, and the timestamp of the upserted sample.
- **`cluster_bumpepoch` could not succeed.** `CLUSTER BUMPEPOCH` answers a single
  line holding both the outcome and the resulting epoch, as in `STILL 84`, while
  `ClusterBumpEpochResult` was derived as a plain lowercase enum tag. Every call
  failed with `unknown variant`. The type now parses that line and carries the
  epoch.
- **Eight command methods were effectively uncallable.** They carried a generic
  type parameter that appeared nowhere in their signature, so it could not be
  inferred and reaching them required a turbofish naming a type that was never
  used: `cluster_addslots`, `cluster_addslotsrange`,
  `cluster_count_failure_reports`, `cluster_delslots`, `cluster_delslotsrange`,
  `cluster_forget`, `ft_profile_search` and `sentinel_failover`.
- `ClusterInfo`, `ClusterState`, `ClusterBumpEpochResult`, `ClusterLinkInfo` and
  `ClusterLinkDirection` now derive `Debug`, and the two enums `PartialEq`, like
  the other types decoded from a cluster reply.
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
- **Search: several clauses declared the wrong argument count and were rejected
  by the server.** `FT.AGGREGATE`'s `LOAD` and `FT.SEARCH`'s `RETURN` announced
  the number of attributes where Redis counts arguments, so renaming an attribute
  (`FtAttribute::new("a").r#as("b")`) produced `LOAD 1 a AS b` and failed with
  `Unknown argument AS`. `PARAMS` announced the number of pairs instead of twice
  that, so any query with more than one parameter failed. `FT.SPELLCHECK`'s
  `TERMS` was prefixed with a count the syntax does not take. These counts are
  now derived from what is actually written rather than from the collection's
  length, so they cannot disagree with it.
- `resp::Value`'s `Boolean` compares by value rather than by discriminant.
- Closing the last `Client` clone closes the connection race-free.
- A collection element that fails to parse mid-iteration now surfaces an error
  instead of silently truncating the iteration.
- `Debug` on a response renders the decoded reply rather than the internal tape.
- The deserializer's two string entry points agree. A reply readable as a `String`
  is now equally readable where serde asks for a borrowed string — a struct field
  name or an enum variant name — instead of the target type deciding whether the
  command succeeds.

[Unreleased]: https://github.com/dahomey-technologies/rustis/compare/0.21.0...HEAD
[0.21.0]: https://github.com/dahomey-technologies/rustis/compare/0.20.0...0.21.0
[0.20.0]: https://github.com/dahomey-technologies/rustis/compare/0.19.3...0.20.0
