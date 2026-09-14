---
id: 50
title: Say what the area means
type: feature
status: backlog
milestone: v0.4
depends_on:
- 49
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: m
area: map
---

## Problem

"Big" has three useful meanings on a machine, and a treemap that only shows
bytes answers only one of them.

## Proposal

`m` cycles the metric, and the pane title always names the current one:

    size        bytes. What to delete
    count       packages. Which manager sprawled
    dependents  how many things need it

The third is the one worth building. A big cell under `dependents` is
load-bearing: delete it and a dozen things break. It is the inverse of the size
map, and it is the view that stops a mistake rather than inviting one.

## Acceptance criteria

- [ ] All three metrics lay out correctly and are cycled by `m`
- [ ] The pane title names the active metric
- [ ] Scrolling over the map cycles the metric, as the design states
- [ ] `dependents` counts transitive dependents, not just direct ones
- [ ] The zoom level survives a metric change
