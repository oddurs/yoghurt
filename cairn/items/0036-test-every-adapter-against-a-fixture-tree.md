---
id: 36
title: Test every adapter against a fixture tree
type: chore
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 14
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: m
area: testing
---

## Problem

Eight adapters that can only be tested on the developer's machine are eight
adapters that break silently on everybody else's.

## Proposal

One fixture tree per adapter, checked into the repository, representing a
plausible installation including the awkward cases: a scoped npm package, a
keg-only formula, a rustup shim, a dangling symlink, a cask-installed app.
Adapters take a root path; no test ever touches the real machine.

## Acceptance criteria

- [ ] Every adapter takes a root path rather than assuming absolute locations
- [ ] Every adapter has a fixture tree and a test asserting its exact facts
- [ ] No test reads outside the fixture directory, enforced in CI
- [ ] The suite passes on a machine with no package managers installed at all
