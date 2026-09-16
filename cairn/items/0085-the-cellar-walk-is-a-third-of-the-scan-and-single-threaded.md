---
id: 85
title: The Cellar walk is a third of the scan and single threaded
type: chore
status: backlog
milestone: v0.3
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: s
area: source
---

## Problem

Making the sources concurrent took the scan from 4.5s to 3.1s, and what remains
is dominated by one thing: the Homebrew adapter walking 160,000 files under the
Cellar to measure 262 kegs, on one thread, inside its own `scan`.

Source-level concurrency cannot help, because this *is* one source.

## Proposal

Measure the kegs concurrently. They are independent directory walks and the work
is I/O rather than computation.

## Acceptance criteria

- [ ] Keg sizes are measured on more than one thread
- [ ] The sizes are identical to the sequential ones
- [ ] The Homebrew scan is measurably faster, with the number recorded
