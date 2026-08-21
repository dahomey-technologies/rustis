# Security policy

## Reporting a vulnerability

Report privately, through GitHub's **Report a vulnerability** button on the
[Security tab](https://github.com/dahomey-technologies/rustis/security/advisories/new).
It opens a private advisory that only the maintainers can read.

Please do not open a public issue for a vulnerability. A public report is a
disclosure, and it happens before there is a version to upgrade to.

Useful in a report: the rustis version, the Redis version and topology
(standalone, cluster, sentinel), whether TLS is in use, and the shortest input or
sequence that reproduces it.

## Supported versions

The crate is pre-1.0. Fixes go to the latest published version; there are no
maintained release branches.

## What the crate treats as hostile input

Everything arriving over the connection. A reply is parsed before anything else
knows whether the server is the one you meant, so the RESP layer assumes an
attacker-controlled byte stream:

* `#![forbid(unsafe_code)]`, so a malformed or hostile reply cannot become a
  memory-safety bug.
* The explicit-panic lint family is denied crate-wide, and `indexing_slicing` in
  `resp/` and `network/` — the two zones where a panic ends the connection rather
  than one command.
* Frame size, nesting depth and collection length are bounded and configurable
  (`Config::limits`), so a crafted reply cannot drive an unbounded allocation or
  a stack overflow. The defaults are 128 levels of nesting, 512 MiB per bulk
  string and 128 M elements per collection.
* Four `cargo-fuzz` targets exercise the frame parser, both deserializers and the
  chunked decode path, weekly in CI.

A report showing any of these failing — a panic, an unbounded allocation, a
hang — on input a server can send is in scope, and so is anything that lets a
connection read or write on behalf of another.

Out of scope: what a Redis server you have chosen to trust does with the data you
send it, and configuration that disables the protections above.
