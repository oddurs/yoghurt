---
id: 34
title: Say everything without colour
type: feature
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 17
created: 2026-09-13
updated: 2026-09-15
priority: p1
effort: s
area: theme
---

## Problem

Six states distinguished by colour is a tool that lies to about one man in
twelve, and to anyone piping it, and to anyone in a terminal whose palette does
not match the assumption.

## Proposal

Colours are roles, not values. `auto` maps every role onto the terminal's own
ANSI palette and is the default, because yoghurt should look like the terminal
it runs in rather than like somebody else's screenshot. `mono` drops colour
entirely and loses nothing, because every state already carries a glyph.

## Acceptance criteria

- [ ] Every colour in the interface is a named role, not a literal
- [ ] `NO_COLOR` is honoured without configuration
- [ ] `--theme mono` renders every state distinguishably with no colour at all
- [ ] A frame rendered under `mono` is asserted in the harness to carry every glyph
- [ ] No information is available only through colour, anywhere
