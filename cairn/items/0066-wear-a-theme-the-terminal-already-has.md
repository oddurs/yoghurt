---
id: 66
title: Wear a theme the terminal already has
type: feature
status: backlog
milestone: later
created: 2026-09-13
updated: 2026-09-13
priority: p3
effort: s
area: theme
---

## Problem

`auto` and `mono` cover the honest cases. Somebody who has already chosen a
palette for their terminal should not have to transcribe it.

## Proposal

Read Ghostty theme files directly, as harrow does. Parked because `auto` already
inherits the terminal's ANSI palette, which gets most of the way for none of the
work.

## Acceptance criteria

- [ ] Ghostty theme files parse into the role set
- [ ] A theme that does not define a role falls back rather than failing
