---
id: 35
title: Adding a source is one file
type: docs
status: backlog
milestone: v0.2
depends_on:
- 28
created: 2026-09-13
updated: 2026-09-13
priority: p2
effort: s
area: docs
---

## Problem

The architecture only pays off if the next adapter is genuinely cheap to write.
That claim needs to be written down and checked against reality.

## Proposal

Document the `Source` trait and the `Fact` vocabulary as a contract: what an
adapter may assume, what it must never do, and how to test it against a fixture
tree. Walk through one real adapter end to end.

## Acceptance criteria

- [ ] `docs/sources.md` documents the trait, the facts, and the fixture convention
- [ ] It names the rules: no UI knowledge, no cross-adapter knowledge, no panics
- [ ] It walks through the cargo adapter as a worked example
- [ ] A reader could add a new package manager from this document alone
