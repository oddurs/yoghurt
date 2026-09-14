---
id: 47
title: Lay out a treemap that squarifies
type: feature
status: backlog
milestone: v0.4
depends_on:
- 13
- 46
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: l
area: map
---

## Problem

A naive treemap produces slivers: cells one character wide and forty tall, which
carry no label and communicate nothing.

## Proposal

Squarified layout, which keeps cells close to square by greedily choosing
whether to add the next item to the current row or start a new one. Terminal
cells are roughly twice as tall as wide, so the aspect ratio target has to
account for that or every cell comes out wrong in the same direction.

Layout is pure: sizes and a rectangle in, rectangles out. That makes it testable
without drawing anything.

## Acceptance criteria

- [ ] Layout is a pure function: `(Vec<u64>, Rect) -> Vec<Rect>`
- [ ] Cell aspect ratios account for terminal cells being taller than wide
- [ ] Areas are proportional to sizes within one cell of rounding
- [ ] No cell is narrower than one column or shorter than one row
- [ ] Items too small to draw are collected into a single remainder cell
- [ ] Tested against known inputs with asserted rectangles
