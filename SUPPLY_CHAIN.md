# Supply Chain

This file says what is known about the crates this one depends on. [SECURITY.md](SECURITY.md) says
how the checks are run, and `audit/<task>/` holds their latest results.

A program that uses this library resolves its own versions, so what is said here holds for the
versions in `Cargo.lock`, not for whatever a program ends up with. There is no vendored copy of the
dependencies: this is a library.

`Cargo.lock` has 84 packages: this crate, `xtask` and 82 others, of which 40 are normal or build
dependencies. The rest are only used by the tests, or by features that are not turned on.

## Checks

| Check | What it enforces | Latest result |
|---|---|---|
| `cargo deny` | sources: crates.io only; licenses: Apache-2.0, MIT, Unicode-3.0 and Zlib only; no wildcard versions; no known vulnerability, unmaintained or yanked crate | passes, with a warning for two versions of `syn` (2 and 3), both only used at build time |
| `cargo vet` | each dependency has a recorded audit, or an exemption | 27 audited, 55 exempted |
| `cargo geiger` | counts `unsafe` code in the dependencies; a count, not a check | see below |

### Audits

The audits are imported from seven organizations: Mozilla, Google, the Bytecode Alliance, Zcash,
ISRG, Embark Studios and Ariel OS. This project has written no audit of its own.

**55 crates are exempted.** The maintainer has looked through them and keeps them exempted, but no
audit of them is on record. They include crates this library runs, Ratatui itself among them; the
list is in `supply-chain/config.toml`.

### `unsafe` in the Dependencies

This crate has no `unsafe` code and forbids it. Its dependencies do have some, which `forbid` does
not reach. These are the crates with the most, counted once each, from `audit/geiger/geiger.txt`:

| Crate | Unsafe expressions used by the build / found | What it is here for |
|---|---|---|
| hashbrown 0.17.1 | 1399 / 1612 | hash tables in Ratatui's caches and its layout solver |
| allocator-api2 0.2.21 | 819 / 903 | allocation for hashbrown |
| lru 0.18.5 | 611 / 611 | Ratatui's layout cache |
| ryu 1.0.23 | 560 / 690 | formatting floats, for compact_str |
| compact_str 0.9.1 | 393 / 439 | the text of each cell in a `Buffer` |
| foldhash 0.2.0 | 202 / 202 | hashbrown's hasher |
| itoa 1.0.18 | 145 / 145 | formatting integers, for compact_str |
| syn 3.0.9 | 88 / 88 | parsing Rust, at build time only |
| syn 2.0.119 | 87 / 87 | parsing Rust, at build time only |
| castaway 0.2.4 | 31 / 31 | specialization in compact_str |
| itertools 0.14.0 | 22 / 49 | iterators in Ratatui |
| proc-macro2 1.0.107 | 14 / 14 | proc macros, at build time only |
| unicode-ident 1.0.27 | 4 / 4 | identifiers, at build time only |
| either 1.19.0 | 2 / 2 | for itertools |

Crates whose `unsafe` code the build does not use are left out, among them libc and rustix, which
only the tests and features that are off pull in. Miri runs this crate's unit tests through the
code of the crates above that they reach, which is not all of it.

## Code Run at Build Time

These run on the machine that builds a program, not in it.

| Kind | Crates |
|---|---|
| Procedural macros | darling_macro, indoc, instability, rustversion, strum_macros, thiserror-impl |
| Build scripts | proc-macro2, quote, rustversion, thiserror |

## Updating Dependencies

A new version of a dependency is taken only after it has been out for 7 days, and only when the
maintainer decides to: `cargo update -p <crate>` within what `Cargo.toml` allows, `cargo upgrade`
past it. Before it is taken, what changed is read for new build scripts and procedural macros, new
`unsafe` code, access to the network or the file system, obfuscated or base64 strings, and new
crates it brings in. It is then recorded with `cargo vet certify`, and `cargo xtask deny` and
`cargo xtask vet` run again.

## What Is Outside These Checks

`fuzz/` is a workspace of its own, with its own `Cargo.lock` of 52 packages. It is not in a user's
dependency tree and `cargo vet` does not cover it; `cargo deny` checks its advisories and sources.
`xtask/` adds no package to `Cargo.lock`, and is never built for a user. The tools the audit runs
are pinned in `xtask/src/pins.rs`, and nothing here audits those.
