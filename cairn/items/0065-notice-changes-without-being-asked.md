---
id: 65
title: Notice changes without being asked
type: feature
status: backlog
milestone: later
created: 2026-09-13
updated: 2026-09-13
priority: p3
effort: m
area: scan
---

## Problem

A tool kept open in a pane beside the work goes stale while a `brew install`
runs in another one.

## Proposal

Watch the Cellar, the Caskroom, `~/.cargo/bin` and the PATH directories, and
invalidate the cache when they change. Parked because `notify` is a dependency
with a silent failure mode, and `r` plus a visible cache age already solves the
problem honestly.

## Acceptance criteria

- [ ] A change in a watched directory invalidates the cache
- [ ] A watcher that dies is reported, not silently stopped
- [ ] The feature can be switched off
