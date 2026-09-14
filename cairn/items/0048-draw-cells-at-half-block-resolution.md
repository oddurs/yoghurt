---
id: 48
title: Draw cells at half-block resolution
type: feature
status: backlog
milestone: v0.4
depends_on:
- 34
- 47
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: map
---

## Problem

At one cell per terminal row, a 24-row terminal has 24 vertical steps, and small
packages disappear entirely.

## Proposal

Draw with `▀`, setting foreground and background separately, so one terminal row
carries two logical rows. That doubles vertical resolution for the cost of one
character.

Hue carries the source, consistently with the other views: the colour that means
Homebrew in the list means Homebrew here. Lightness carries age, so a wall of dim
cells reads as old without a legend.

## Acceptance criteria

- [ ] Cells render at two logical rows per terminal row
- [ ] Source hue matches the list view exactly
- [ ] Labels draw inside a cell when it fits and are omitted when it does not
- [ ] Under `mono`, cells are distinguished by border and label, not fill
- [ ] A frame at 100x30 is asserted in the harness
