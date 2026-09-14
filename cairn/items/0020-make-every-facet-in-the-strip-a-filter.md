---
id: 20
title: Make every facet in the strip a filter
type: feature
status: backlog
milestone: v0.1
depends_on:
- 13
- 16
created: 2026-09-13
updated: 2026-09-14
priority: p1
effort: m
area: chrome
---

## Problem

"Show me the problems" is the most common thing a person wants from this tool,
and making them learn a filter grammar to ask it is a failure.

## Proposal

Every count in the status strip is a button. Clicking `31 outdated` filters the
list to those thirty-one; clicking it again clears. The summary is the
navigation.

Each facet is a saved query over the graph rather than a hardcoded branch, so
adding one later is a line of configuration, not a new code path.

## Acceptance criteria

- [ ] Clicking a facet filters the list; clicking the same facet clears it
- [ ] The active facet is visibly active, by shape as well as colour
- [ ] `!` from the keyboard cycles the facets in the same order
- [ ] Facets are defined as queries in one place, not branched on at each call site
- [ ] A facet matching nothing says so rather than showing an empty pane
