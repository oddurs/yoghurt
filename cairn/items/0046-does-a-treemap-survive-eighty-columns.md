---
id: 46
title: Does a treemap survive eighty columns?
type: spike
status: backlog
milestone: v0.4
depends_on:
- 14
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: map
---

## Question

Is a treemap legible in a terminal at the widths people actually use, or is the
bar chart the real design and the treemap the decoration?

## Why it has to be answered before the work

The Map is the largest piece of work in the roadmap and the least certain to
survive contact with a real terminal. A fallback that gets used most of the time
is not a fallback — it is the design, and building the treemap first would be
building the wrong thing at the greatest cost.

## Options

- Half-block rendering gives enough resolution at 80x24 to be worth it
- It needs 120 columns, so it is a wide-terminal feature with a bar chart default
- Cell labels never fit, and a labelled bar chart beats an unlabelled treemap

## What would settle it

Render this machine's real 293 packages as a static treemap at 80x24, 100x30 and
160x50, using half-blocks, and look at them. Count how many cells can carry a
readable label at each size. Do the same for the bar chart and compare what each
actually communicates.

Timebox: one day, and the output is three screenshots and a decision.

## Answer
