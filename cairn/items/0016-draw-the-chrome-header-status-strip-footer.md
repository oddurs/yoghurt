---
id: 16
title: 'Draw the chrome: header, status strip, footer'
type: feature
status: done
milestone: v0.1
depends_on:
- 7
- 14
created: 2026-09-13
updated: 2026-09-14
priority: p0
effort: m
area: chrome
---

## Problem

Every view needs the same frame around it, and it has to say what is happening
before a single row is read.

## Proposal

Two fixed lines at the top, one at the bottom.

Line one: identity, totals, and freshness — `scanned 2m ago`, with a spinner
while a scan is in flight. The timestamp is how a reader knows whether they are
looking at the machine or at a memory of it.

Line two: the counts, one per facet. Line three down is the view. The footer is
the keys that apply right now, and it changes per view.

Below 80 columns the header drops to identity and totals only.

## Acceptance criteria

- [ ] Header shows hostname, package count, source count and total size
- [ ] Freshness is shown in words and updates without a rescan
- [ ] Status strip shows wanted, pulled in, orphan, shadowed and broken counts
- [ ] Footer lists only keys that do something in the current view
- [ ] Renders correctly at 60, 80, 100 and 200 columns, asserted in the harness
