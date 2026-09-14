---
id: 49
title: Zoom into a cell and back out
type: feature
status: backlog
milestone: v0.4
depends_on:
- 19
- 48
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: map
---

## Problem

At machine scale, one cell is Homebrew and it is 90% of the picture. The
interesting question is what is inside it.

## Proposal

Clicking a cell zooms to it and lays out its children. A breadcrumb in the pane
title records the path, and clicking any segment of it zooms back out. `esc`
goes up one level.

Zoom is a path into the graph, not a separate mode, so a facet or filter set
elsewhere still applies inside the zoom.

## Acceptance criteria

- [ ] Clicking a cell zooms in; the breadcrumb shows the path
- [ ] Clicking a breadcrumb segment returns to that level
- [ ] `esc` goes up one level and does nothing at the top
- [ ] An active facet still applies inside a zoom
- [ ] Zooming into a cell with one child says so rather than showing one huge cell
