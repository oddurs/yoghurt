---
id: 19
title: Drive every one of those with the mouse
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
depends_on:
- 16
- 17
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: l
area: mouse
---

## Problem

Mouse-first is the product thesis, not a convenience. A TUI where the mouse only
scrolls is a keyboard TUI that tolerates a mouse.

## Proposal

Every drawable region registers a hit rectangle during draw, and events resolve
against that list. Hover is tracked, not just clicks — that is the difference
between mouse-first and mouse-tolerated.

    hover        row highlights; hovering a facet previews its count
    click        select a row, collapse a group, toggle a facet
    double       open detail
    right-click  context menu
    scroll       three rows; over a group header, collapse it

## Acceptance criteria

- [ ] Regions register during draw; nothing hard-codes a coordinate
- [ ] Hover highlights the row under the pointer without selecting it
- [ ] Clicking a group header collapses it; clicking a row selects it
- [ ] Scroll moves three rows and stops cleanly at both ends
- [ ] Every mouse action has a key that does the same thing, and the reverse
- [ ] Interaction is asserted through the test harness, with no terminal
