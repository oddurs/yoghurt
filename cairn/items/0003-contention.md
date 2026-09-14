---
id: 3
key: v0.3
title: Contention
type: milestone
status: backlog
created: 2026-09-13
updated: 2026-09-13
priority: p2
due: 2027-01-12
---

## Ships

The Path view: which of the 592 binaries on your PATH actually wins when you
type its name, and what loses.

## Done when

- [ ] Every command on PATH resolves to exactly one winner
- [ ] By default only contested names are shown; `a` shows all of them
- [ ] The footer is PATH itself, in order, and clicking a segment filters to it
- [ ] `tab` and the header tabs move between Inventory and Path
- [ ] `/` filters any view by typing
- [ ] `?` explains every key and every click without leaving the interface

## Explicitly not in this milestone

- Editing PATH. yoghurt does not write to your shell configuration
- The Map view

## Gated by

Spike 0025 decides whether contested names are common enough to justify a whole
view. If the answer is no, this milestone becomes a column in the Inventory and
the rest moves to `later`.
