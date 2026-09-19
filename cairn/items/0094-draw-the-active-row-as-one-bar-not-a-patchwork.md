---
id: 94
title: Draw the active row as one bar, not a patchwork
type: bug
status: done
milestone: v0.2
created: 2026-09-18
updated: 2026-09-18
priority: p1
area: theme
effort: m
---

## What happens

## What should happen

## Reproduction

1.

## 2026-09-18

The active row used REVERSED over spans that each carried their own foreground, and REVERSED turns a foreground into a background — so the bar was white under the name, dark grey under the size and green under the glyph. Dropping the span colours before reversing makes it one bar; modifiers are kept so a group heading stays bold inside it. Reversing rather than setting a background colour is deliberate: a fixed colour can collide with the terminal's own, a reversed bar cannot. The facet chip had the opposite problem — only the count digit was reversed, so the label read as muted as every inactive one; a facet has one hue, so the whole chip carries it. Hover was tracked for rows only, leaving every other clickable thing silent under the pointer in an interface whose stated premise is mouse first; it is now any hit region, underlined by one shared helper.
