---
id: 17
title: Show the inventory as a grouped list
type: feature
status: backlog
milestone: v0.1
depends_on:
- 13
- 16
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: l
area: list
---

## Problem

The main view. 293 rows on this machine, and a flat list of them answers
nothing.

## Proposal

Grouped by source, collapsible, with group headers carrying their own totals.
Everything right of the name is fixed width and right-aligned, so the eye runs
down a column rather than hunting along each row:

    ● ripgrep                    14.1.1   brew   leaf    5.2M   3mo   ↑

Columns drop as the pane narrows in order of how little they answer: age, then
role, then size, then version. Name, state glyph and source never drop.

Every state carries a glyph as well as a colour, so the screen still says
everything it needs to with no colour at all.

## Acceptance criteria

- [ ] Rows group by source with collapsible headers showing count and size
- [ ] Glyphs: ● fine, ↑ outdated, ◐ pulled in, ? orphan, ⊘ shadowed, ✕ broken
- [ ] Columns drop in the stated order at 96, 88, 76 and 64 columns
- [ ] The cursor stays on the same item when a group collapses or the list refilters
- [ ] The list does not jump when the cursor is already visible
- [ ] 300 rows scroll without a perceptible frame cost
