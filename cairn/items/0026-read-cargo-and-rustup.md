---
id: 26
title: Read cargo and rustup
type: feature
status: backlog
milestone: v0.2
depends_on:
- 9
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: source
---

## Problem

22 binaries in `~/.cargo/bin` and 7 rustup toolchains, and the two are tangled:
a dozen of those binaries are rustup shims that nobody installed.

## Proposal

Two adapters. Cargo reads `~/.cargo/.crates2.json`, which records what was
installed, from where, and which binaries it produced — no subprocess needed.
Rustup reads `~/.rustup/toolchains`.

Shims are owned by rustup, not cargo, so they stop being counted twice.

## Acceptance criteria

- [ ] `.crates2.json` yields name, version, source and the binaries installed
- [ ] A cargo binary from a git source records the source, not just a version
- [ ] Rustup shims are attributed to rustup, never counted as installed crates
- [ ] Toolchains appear with their channel and target
- [ ] Neither being installed yields no facts and no error
- [ ] Parsed from a fixture in tests
