---
id: 40
title: Move between the views
type: feature
status: backlog
milestone: v0.3
depends_on:
- 38
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: chrome
---

## Problem

Until now there has been one view, so there has been nothing to move between.

## Proposal

`tab` cycles; the view names in the header are clickable. Each view keeps its
own cursor and scroll position, so moving away and back does not lose where you
were. A facet set in one view stays set in the other — the filter belongs to the
session, not to the pane.

## Acceptance criteria

- [ ] `tab` and `shift-tab` cycle views in both directions
- [ ] View names in the header are clickable and show which is active
- [ ] Each view keeps its own cursor and scroll position
- [ ] An active facet stays active across a view change
- [ ] The footer changes to the keys of the current view
