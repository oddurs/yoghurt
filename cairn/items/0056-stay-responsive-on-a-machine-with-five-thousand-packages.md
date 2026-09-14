---
id: 56
title: Stay responsive on a machine with five thousand packages
type: chore
status: backlog
milestone: v1.0
depends_on:
- 31
- 51
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: m
area: testing
---

## Problem

Every performance decision so far has been measured against 293 packages and 592
binaries. A developer machine ten years old with several language ecosystems on
it is an order of magnitude bigger.

## Proposal

Generate a synthetic fixture of 5000 packages and 10000 commands and measure
what actually degrades: graph assembly, the four queries, list scrolling,
treemap layout, filter typing.

Fix what is worse than linear. Record the numbers so a regression is visible.

## Acceptance criteria

- [ ] A synthetic fixture at that scale exists and is used in a benchmark
- [ ] First frame from cache stays under 50ms at that scale
- [ ] Scrolling and filtering stay responsive with no perceptible lag
- [ ] Nothing in the hot path is worse than linear in package count
- [ ] The benchmark runs in CI and fails on a significant regression
