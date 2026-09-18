---
id: 91
title: Give a toolchain its own row shape
type: feature
status: backlog
milestone: v0.3
created: 2026-09-17
updated: 2026-09-17
priority: p2
area: list
effort: l
---

## Problem

## Proposal

## Acceptance criteria

- [ ]

## 2026-09-17

The rustup block does not read as packages because toolchains are not packages.

    shims                        rustup  -
    1.98.1-aarch64-apple-darwin  rustup  -
    nightly-aarch64-apple-darwin rustup  -

`shims` is rustup's dispatch directory, not a thing that was installed. The
others put a version in the name column and a dash where the version goes.

Decide the shape before patching the strings. A toolchain has a channel, a
host triple, a date for nightly, and a default flag — none of which the
package row has anywhere to put. Homebrew casks and App Store apps may want
the same treatment, so this is the general question of whether one row shape
fits every source.
