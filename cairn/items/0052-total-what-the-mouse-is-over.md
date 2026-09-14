---
id: 52
title: Total what the mouse is over
type: feature
status: backlog
milestone: v0.4
depends_on:
- 48
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: mouse
---

## Problem

Most cells in a treemap are too small to label, so a cell without a hover
readout is a coloured rectangle that means nothing.

## Proposal

One line under the map, written by whatever the pointer is over:

    llvm@21 · homebrew · 1.5G · 27% of homebrew · pulled in by swift-format

That line is the treemap's entire status bar and it is where the answer usually
is. Keyboard navigation writes the same line for the selected cell, so the
feature is not mouse-only.

## Acceptance criteria

- [ ] Hovering any cell writes name, source, size, share and origin
- [ ] Keyboard selection writes the same line for the selected cell
- [ ] The line is stable while the pointer is still — no flicker
- [ ] Pointing at nothing shows the total for the current zoom level
