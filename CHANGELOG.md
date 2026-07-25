# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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

### Fixed

- Iterating a collection past its fifth element could yield corrupted values
  (the fallback re-parser produced ranges against the wrong buffer base). The
  tape indexes every element uniformly, removing that path by construction.
- A large reply arriving in many TCP segments was re-parsed from the start on
  every segment, making decode cost quadratic in the number of segments. The
  decoder now keeps resume state, so the cost is linear in the reply size.

### Added

- `RespResponse::compact()` copies a response's referenced bytes into
  freshly-sized buffers, releasing the larger recycled network block a retained
  response would otherwise pin. The client-side cache now compacts entries before
  storing them.

### BREAKING CHANGES

- `resp::RespFrame`'s collection variants (`Array`, `Map`, `Set`, `Push`) changed
  from `{ len: usize, ranges: [Range<u32>; 5] }` to `{ tape: bytes::Bytes, root:
  u32 }`. Code that matched on the old shape must be updated. Scalar variants are
  unchanged.
