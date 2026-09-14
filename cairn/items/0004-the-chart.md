---
id: 4
key: v0.4
title: The chart
type: milestone
status: backlog
created: 2026-09-13
updated: 2026-09-13
priority: p2
due: 2027-02-16
---

## Ships

The Map view: a treemap of the machine where area is bytes on disk, and where
hovering anything tells you what it is and who pulled it in.

## Done when

- [ ] Cells are laid out squarified, so they stay close to square at any aspect
- [ ] Cells draw at half-block resolution, so small packages remain visible
- [ ] Clicking a cell zooms in; the breadcrumb zooms back out
- [ ] `m` cycles what area means: size, count, dependents
- [ ] Below eighty columns, or under `mono`, it becomes a sorted bar chart with
      the same hit rectangles
- [ ] Hovering writes a line naming the cell, its size, its share and its origin

## Explicitly not in this milestone

- Deleting anything from the map. yoghurt never writes to the machine

## Gated by

Spike 0046 decides whether a treemap is legible at eighty columns. If it is not,
the bar chart is the design and the treemap moves to `later`.
