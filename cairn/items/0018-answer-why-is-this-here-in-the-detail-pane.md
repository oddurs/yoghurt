---
id: 18
title: Answer "why is this here" in the detail pane
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
depends_on:
- 13
- 17
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: m
area: detail
---

## Problem

Knowing that `glib` is a dependency is half an answer. The question a person
actually has is which thing they installed dragged it in — and that is the
question no package manager answers in one step.

## Proposal

For a leaf, detail is facts. For a dependency, it prints the path back to the
nearest package that was wanted:

    glib
    └ gtk+3
      └ inkscape  ← you installed this

That path is already computed by the `pulled in` query, so this costs layout
rather than logic.

Below 96 columns the pane stops paying for itself: `↵` opens it as a
full-screen overlay instead of halving a pane that is already too narrow.

## Acceptance criteria

- [ ] A leaf shows version, source, size, install date and what it provides
- [ ] A dependency additionally shows the chain back to a wanted package
- [ ] A package wanted by nothing and depended on by nothing says so plainly
- [ ] The uninstall command is shown as copyable text — yoghurt does not run it
- [ ] Below 96 columns the pane becomes an overlay, asserted in the harness
- [ ] Long chains scroll rather than widening the pane
