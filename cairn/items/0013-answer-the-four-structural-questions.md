---
id: 13
title: Answer the four structural questions
type: feature
status: backlog
milestone: v0.1
depends_on:
- 10
- 12
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: graph
---

## Problem

The interface has seven states to show. Writing seven detectors means seven
places to be inconsistent.

## Proposal

Every state is a query over the graph, not a feature:

    orphan     an artifact with no owning package
    broken     a package owning an artifact that is not there
    pulled in  a package reachable only through `depends`, never from `wanted`
    shadowed   two artifacts providing one command; PATH order decides

Four queries generate every glyph in the design. `pulled in` is a reachability
question, which is also what makes the why-chain in the detail pane free: the
path from a package back to a wanted one is already computed.

## Acceptance criteria

- [ ] Each of the four is one function over `Graph` with a doc comment stating the rule
- [ ] Each is tested against a hand-built graph with a known answer
- [ ] `pulled in` returns the path back to the nearest wanted package, not just a bool
- [ ] A package that is both wanted and depended on counts as wanted
- [ ] All four run over a 300-node graph in under 10ms
