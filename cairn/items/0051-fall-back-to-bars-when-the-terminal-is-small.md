---
id: 51
title: Fall back to bars when the terminal is small
type: feature
status: backlog
milestone: v0.4
depends_on:
- 46
- 48
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: map
---

## Problem

Below eighty columns, and under `mono`, a treemap stops communicating. The
design says it degrades rather than disappearing, and this is the item that
makes that true.

## Proposal

A sorted horizontal bar chart with the same data, the same hit rectangles and
the same interactions: the same click zooms, the same hover writes the same
status line, the same metric cycles. Switching is a renderer choice, not a
different feature.

If the spike concluded the bar chart is the better default, this stops being a
fallback and the treemap becomes the special case — the acceptance criteria do
not change either way.

## Acceptance criteria

- [ ] Below eighty columns the bar chart renders instead, automatically
- [ ] Under `mono` the bar chart renders regardless of width
- [ ] Hit rectangles, zoom, hover and metric behave identically in both
- [ ] Switching between them preserves the zoom path and the selection
- [ ] Both are asserted in the harness at the width where they swap
