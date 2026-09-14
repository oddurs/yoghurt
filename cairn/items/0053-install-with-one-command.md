---
id: 53
title: Install with one command
type: chore
status: backlog
milestone: v1.0
depends_on:
- 22
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: packaging
---

## Problem

"Clone it and run cargo build" is not an install path. A tool about package
managers that is awkward to install is an embarrassment it cannot afford.

## Proposal

`brew install oddurs/tap/yoghurt` as the primary path, because the audience
already has Homebrew by definition. `cargo install yoghurt` as the secondary.

## Acceptance criteria

- [ ] The formula exists in the tap and installs a working binary
- [ ] `cargo install yoghurt` works from a clean machine
- [ ] The crate publishes with correct metadata, licence and README
- [ ] The formula is updated by the release workflow, not by hand
- [ ] The README leads with the Homebrew line
