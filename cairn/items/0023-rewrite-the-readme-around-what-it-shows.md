---
id: 23
title: Rewrite the README around what it shows
type: docs
status: backlog
milestone: v0.1
depends_on:
- 17
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: docs
---

## Problem

The README describes a table of source counts. By the end of v0.1 that is no
longer the product, and a README that describes an earlier version is worse than
none.

## Proposal

Lead with a screenshot of the first screen. Explain the one thing that is not
obvious from it — wanted against pulled in — and say plainly what yoghurt does
not do.

## Acceptance criteria

- [ ] A real terminal screenshot of the inventory, not an aspiration
- [ ] Install, first run, and what the first screen means
- [ ] "yoghurt never installs, never uninstalls, never writes to your machine"
- [ ] The keys and clicks that exist today, and nothing that does not
- [ ] No feature described that is not in the tag
