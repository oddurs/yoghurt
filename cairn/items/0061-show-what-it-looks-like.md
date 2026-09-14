---
id: 61
title: Show what it looks like
type: docs
status: backlog
milestone: v1.0
depends_on:
- 23
- 51
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: docs
---

## Problem

Nobody installs a terminal interface they have not seen. The README has one
screenshot from v0.1 and the tool has three views by now.

## Proposal

A screenshot of each view at a sensible size, taken from a real machine with
real numbers on it, in both a light and a dark terminal. Generate them from a
committed fixture so they can be regenerated rather than retaken by hand.

## Acceptance criteria

- [ ] Each of the three views has a screenshot in the README
- [ ] They are generated from a fixture by a script, not captured by hand
- [ ] Light and dark terminals both look correct
- [ ] Nothing in a screenshot shows a feature that does not exist
