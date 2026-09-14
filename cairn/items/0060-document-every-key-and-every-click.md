---
id: 60
title: Document every key and every click
type: docs
status: backlog
milestone: v1.0
depends_on:
- 43
- 45
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: docs
---

## Problem

A mouse-first tool has gestures that no footer lists and no manual page
convention covers. Undocumented gestures are gestures nobody uses.

## Proposal

One reference covering every key and every mouse gesture per view, generated
from the same binding table the interface reads, so it cannot drift. Plus the
config format, the theme roles, and what each state glyph means.

## Acceptance criteria

- [ ] Every binding is documented, generated from the binding table
- [ ] Mouse gestures are documented alongside their keyboard equivalents
- [ ] Every state glyph is listed with what it means and how to see one
- [ ] The config file is documented with a complete worked example
- [ ] A test fails if a binding exists that the reference does not carry
