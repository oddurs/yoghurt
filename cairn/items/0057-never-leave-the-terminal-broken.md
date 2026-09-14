---
id: 57
title: Never leave the terminal broken
type: chore
status: backlog
milestone: v1.0
depends_on:
- 15
- 32
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: runtime
---

## Problem

v0.1 established the restore paths. v1.0 is the promise that they hold under
everything that actually happens: a resize mid-scan, a panic in a worker thread,
a terminal that closes underneath the process, a suspend and resume.

## Proposal

Enumerate the ways this can end and test each one deliberately rather than
hoping. A worker thread panicking must not leave the main loop holding raw mode
forever.

## Acceptance criteria

- [ ] A panic in any worker thread restores the terminal and reports the panic
- [ ] SIGTSTP and resume leave the screen correct
- [ ] A terminal closing underneath the process does not leave a process behind
- [ ] A resize during a scan, a zoom and a filter all render correctly
- [ ] Each of these has a test that would fail if the handling were removed
