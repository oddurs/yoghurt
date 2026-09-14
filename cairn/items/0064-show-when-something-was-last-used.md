---
id: 64
title: Show when something was last used
type: feature
status: backlog
milestone: later
created: 2026-09-13
updated: 2026-09-13
priority: p3
effort: m
area: graph
---

## Problem

"What have I not run in a year" is the most useful question a tool like this
could answer, and the original design had a `·` stale glyph for it.

## Proposal

Parked because the data may not exist. Access times are the obvious source and
they are unreliable: `noatime` and `relatime` are common, and on some
filesystems the field is meaningless. Shipping a glyph backed by a number that
is silently wrong is worse than not shipping it.

Before this becomes real, something has to establish where a trustworthy signal
could come from — atime where it is reliable, shell history, `mdfind` last-used
metadata on macOS — and what the hit rate is.

## Acceptance criteria

- [ ] A spike establishes whether a trustworthy last-used signal exists
- [ ] If it does, staleness is only shown where the signal is known good
- [ ] A machine where it cannot be determined says so rather than guessing
