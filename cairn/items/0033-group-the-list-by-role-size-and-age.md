---
id: 33
title: Group the list by role, size and age
type: feature
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 17
created: 2026-09-13
updated: 2026-09-14
priority: p1
effort: m
area: list
---

## Problem

Grouping by source answers "what is Homebrew responsible for". It does not
answer the two questions the numbers on this machine actually raise: what did I
choose, and where did 5.6 GB go.

## Proposal

`g` cycles the axis, and the axis name in the pane title is clickable:

    source   what each manager is responsible for
    role     wanted against pulled in against orphan — 47 against 122 against 36
    size     buckets: >100M, 10-100M, 1-10M, <1M
    age      installed this month, this quarter, this year, older
    health   outdated, shadowed, broken, fine

Group headers carry their own totals and a share-of-disk bar, which is most of
what a treemap would tell you and arrives two milestones earlier.

## Acceptance criteria

- [ ] All five axes work and are cycled by `g` and by clicking the title
- [ ] Group headers show count, total size and a share bar
- [ ] The selected item stays selected across a change of axis
- [ ] Collapse state is remembered per axis within a session

## 2026-09-14

Found a real inconsistency while checking this against the machine: the strip said 87 wanted and grouping by role said 65, because State collapsed Outdated over Fine. Being out of date does not change whether you asked for something. State is now provenance only, outdated is an orthogonal flag on the row, and the two numbers agree.
