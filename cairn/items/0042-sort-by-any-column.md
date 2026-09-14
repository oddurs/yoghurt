---
id: 42
title: Sort by any column
type: feature
status: backlog
milestone: v0.3
depends_on:
- 19
- 33
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: m
area: list
---

## Problem

Grouping by size bucket tells you roughly where the disk went. "What are the ten
biggest things" is still a question you cannot ask.

## Proposal

`s` cycles the sort column; clicking a column header sorts by it, clicking again
reverses. The active sort is visible in the header, by arrow as well as colour.
Sorting applies within groups, so grouping and sorting compose rather than
fighting.

## Acceptance criteria

- [ ] Name, version, size, age and source all sort, ascending and descending
- [ ] Clicking a column header sorts; clicking it again reverses
- [ ] The active sort column and direction are visible without colour
- [ ] Sorting is stable, so equal rows keep their previous order
- [ ] Sort survives a change of filter
