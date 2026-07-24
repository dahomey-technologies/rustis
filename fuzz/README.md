# rustis fuzz targets

`cargo-fuzz` (libFuzzer) targets over the RESP read path. This addresses
**PROC-01** in `RUSTIS_AUDIT.md`: the invariant is that arbitrary, untrusted
server bytes must produce an error — never a panic or a process abort. Running
these retires the "panic/abort on server input" class at once (RESP-02, RESP-07,
RESP-08, RESP-12, VAL-02, HARD-01).

## Prerequisites

```bash
cargo install cargo-fuzz     # needs a nightly toolchain
rustup toolchain install nightly
```

## Targets

| Target | What it drives |
|---|---|
| `frame_parser` | `RespFrameParser::parse` directly, one-shot |
| `decode_chunked` | Streaming `BufferDecoder`, input split at fuzzer-chosen byte boundaries — exercises the partial-frame / `Error::EOF` resume path (and makes RESP-06's re-parse observable) |
| `resp_deserializer` | Bytes → `Value` via `RespBuf::to` (frame parser + `RespDeserializer`) |
| `value_deserializer` | Bytes → `Value`, then `Value` → several concrete Rust types (coercions in `value_deserializer.rs`) |

## Running

```bash
cargo +nightly fuzz run frame_parser
cargo +nightly fuzz run decode_chunked
cargo +nightly fuzz run resp_deserializer
cargo +nightly fuzz run value_deserializer
```

A time-boxed CI run, e.g.:

```bash
cargo +nightly fuzz run frame_parser -- -max_total_time=60
```

## Corpus vs seeds

- `corpus/<target>/` is the **live, coverage-guided corpus** libFuzzer grows as
  it runs. It is machine-generated, can reach tens of MiB, and is **gitignored**.
- `seeds/<target>/` holds a small set of **curated, hand-written starter inputs**
  (representative valid RESP3 frames, including a >5-element array to reach the
  range path from RESP-04). These *are* tracked.

`cargo fuzz run` reads and writes `corpus/<target>/` by default. To prime a fresh
checkout from the tracked seeds, pass the seed directory as an extra corpus:

```bash
cargo +nightly fuzz run frame_parser corpus/frame_parser seeds/frame_parser
```

libFuzzer merges every input from both directories at startup and writes newly
discovered inputs only into the first one (`corpus/<target>/`).

## Wiring into the crate

The targets reach otherwise `pub(crate)` internals through the `fuzzing` feature
of the parent crate, which compiles the `rustis::fuzz_api` façade. That feature
is fuzz-only and carries no public-API stability guarantee.
