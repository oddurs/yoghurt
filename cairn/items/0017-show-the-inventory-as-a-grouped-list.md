---
id: 17
title: Show the inventory as a grouped list
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
depends_on:
- 13
- 16
created: 2026-09-13
updated: 2026-09-14
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
- [ ] `yoghurt` with a tty opens this instead of the table (moved from 0021)

## 2026-09-14

0013 measured 1652 orphan artifacts of 3270 on this machine. Most are macOS system binaries under /usr/bin, /bin and /sbin, which no package manager owns and which are not interesting. Showing them as orphans would bury the 35 applications that actually are. The list needs either a system pseudo-source or a rule that unowned paths under the system prefixes are not orphans.

## 2026-09-14

No per-row source column: the list is grouped by source, so the heading above every row already says it and repeating it cost twelve columns for nothing. It comes back when 0033 adds the other grouping axes.
