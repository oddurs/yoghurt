---
id: 12
title: Assemble the facts into one graph
type: feature
status: backlog
milestone: v0.1
depends_on:
- 9
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: graph
---

## Problem

Facts arrive from several sources, unordered, possibly contradictory, and
possibly about the same artifact. Something has to be the single place that
reconciles them.

## Proposal

Three node kinds and four edge kinds, which is the whole model:

    package  --owns-->     artifact
    artifact --provides--> command
    package  --depends-->  package
    (you)    --wanted-->   package

All logic lives here. Views are projections over it and hold no rules of their
own. Adding a package manager must never require touching this file.

## Acceptance criteria

- [ ] `Graph::from_facts(Vec<Fact>) -> Graph`, deterministic regardless of fact order
- [ ] Two sources claiming one artifact is representable, not a panic
- [ ] Node and edge counts are asserted against a hand-written fact set in tests
- [ ] A cycle in the dependency edges does not hang any traversal
