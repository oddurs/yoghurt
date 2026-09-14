---
id: 58
title: Report a scan that only partly worked
type: feature
status: backlog
milestone: v0.2
depends_on:
- 32
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: scan
---

## Problem

Eight sources, each of which can fail independently: a manager mid-upgrade, a
permission denied, a subprocess that hangs, a JSON format that changed. Silently
showing seven-eighths of the machine is the worst possible behaviour for a tool
whose only job is to tell you the truth about it.

## Proposal

A partial scan is a first-class state, not an error. The header says the result
is incomplete, the strip names which source failed, and the detail of the
failure is one click away. Counts say they are partial rather than quietly
undercounting.

A hung subprocess is killed on a timeout and reported as timed out.

## Acceptance criteria

- [ ] A failed source is named in the interface, with its error reachable
- [ ] Totals visibly state that they are partial
- [ ] A subprocess that hangs is killed after a timeout and reported
- [ ] A partial result is never written to the cache as though it were complete
- [ ] The exit code of `--plain` reflects a partial scan
