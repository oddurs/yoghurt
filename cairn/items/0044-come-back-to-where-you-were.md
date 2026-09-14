---
id: 44
title: Come back to where you were
type: feature
status: backlog
milestone: v0.3
depends_on:
- 40
created: 2026-09-13
updated: 2026-09-13
priority: p2
effort: s
area: config
---

## Problem

A tool kept open in a pane beside the work is reopened constantly, and landing
back at the top of an alphabetical list every time is a small tax paid often.

## Proposal

Remember the view, the group axis, the sort and the collapse state between runs,
in `~/.config/yoghurt/`. Do not remember the filter or the facet — those are
answers to a question just asked, and restoring them would be confusing.

## Acceptance criteria

- [ ] View, axis, sort and collapse state persist across runs
- [ ] Filters and facets deliberately do not persist
- [ ] A missing or malformed state file starts at defaults without an error
- [ ] `--fresh` ignores saved state for this run
