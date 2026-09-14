---
id: 43
title: Explain every key and every click without leaving
type: feature
status: backlog
milestone: v0.3
depends_on:
- 40
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: chrome
---

## Problem

The footer shows six keys. There are more than six, and a mouse-first tool has
gestures a footer cannot list at all.

## Proposal

`?` opens an overlay: every key, every mouse gesture, grouped by view, generated
from the same table the footer and the dispatcher read. A key that exists but is
not in the overlay is a bug the generation makes impossible.

## Acceptance criteria

- [ ] `?` opens the overlay; `?`, `esc` or a click outside closes it
- [ ] The overlay is generated from the binding table, never hand-written
- [ ] Mouse gestures are listed alongside their keyboard equivalents
- [ ] A test asserts every binding in the table appears in the overlay
