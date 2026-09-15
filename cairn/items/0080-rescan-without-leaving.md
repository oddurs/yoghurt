---
id: 80
title: Rescan without leaving
type: feature
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: s
area: scan
---

## Problem

The header says "scanned 4m ago" and counts up forever. There is no way to pick
up a `brew install` you just ran in another pane, so the only refresh is quitting
and starting again — which loses the cursor, the grouping and the filter.

## Proposal

`r` rescans. The header says so while it runs, the cursor stays on whatever it
was pointing at, and the grouping, sort and filter survive.

Synchronous for now: the scan is two seconds and doing it properly is 0032.
Blocking for two seconds after an explicit keypress is honest; doing it without
saying so is not.

## Acceptance criteria

- [ ] `r` rescans and the header says it is scanning while it happens
- [ ] Grouping, sort, filter and collapse state all survive
- [ ] The cursor stays on the same package, or as close as it can
- [ ] A source that fails during a rescan leaves the previous answer intact
