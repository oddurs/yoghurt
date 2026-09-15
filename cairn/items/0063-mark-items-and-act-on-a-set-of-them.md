---
id: 63
title: Mark items and act on a set of them
type: feature
status: backlog
milestone: v0.3
created: 2026-09-13
updated: 2026-09-15
priority: p2
effort: l
area: list
---

## Problem

Marking a set of rows and acting on all of them at once is the difference
between a tool you read and a tool you use. It was parked when yoghurt was
permanently read-only.

## Proposal

Unparked: the read-only promise was retired deliberately, and this item's own
criterion said to revisit only if that happened.

`space` marks a row; the strip becomes the action bar. Whatever the action is,
it composes **one** command for the whole set, shows it verbatim, and requires a
typed confirmation — never a keystroke, and never in bulk without the list being
visible first.

## Acceptance criteria

- [ ] `space` marks and unmarks; the count and total size are shown
- [ ] An action on a marked set composes one command, not one per row
- [ ] The command is shown verbatim before anything runs
- [ ] Confirmation is typed, and the default is to do nothing
- [ ] Marks are cleared when the filter changes, so you cannot act on rows you
      can no longer see
