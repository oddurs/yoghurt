---
id: 31
title: Open from cache and refresh behind you
type: feature
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 12
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: l
area: scan
---

## Problem

Eight adapters, several subprocesses, hundreds of files. Waiting for all of that
before the first frame makes a tool people stop opening.

## Proposal

The first frame comes from `~/.cache/yoghurt/scan.json` and draws immediately,
with the header saying how old it is. A refresh runs behind it and fills rows in
as sources land.

Freshness is shown, never guessed at. A person must always know whether they are
looking at the machine or at a memory of it.

## Acceptance criteria

- [ ] First frame renders in under 50ms from a warm cache
- [ ] The header states the cache age in words and updates live
- [ ] A cold start shows a scanning state, never a blank screen
- [ ] A corrupt or unreadable cache falls back to a full scan without an error
- [ ] The cache format carries a version and refuses to misread an older one
- [ ] `r` forces a full rescan
