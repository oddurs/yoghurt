---
id: 39
title: Show PATH itself as the explanation
type: feature
status: backlog
milestone: v0.3
depends_on:
- 19
- 38
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: path
---

## Problem

Knowing that `/opt/homebrew/bin/python3` wins is half the answer. The other half
is why, and the why is the order of `$PATH`.

## Proposal

The footer of the Path view is `$PATH` itself, in order, as clickable segments.
Clicking one filters the table to what that entry provides. That makes the
ordering visible as the cause, rather than something a person has to hold in
their head.

yoghurt does not write to your shell configuration, so there is no reordering —
the entry's index is shown and that is all.

## Acceptance criteria

- [ ] The footer shows PATH entries in order, abbreviated with `~`
- [ ] Clicking an entry filters to commands it provides
- [ ] The entry that supplies the selected winner is marked
- [ ] A PATH too long for the width truncates from the middle, keeping both ends
