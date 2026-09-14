---
id: 55
title: Survive a machine with nothing installed
type: chore
status: backlog
milestone: v1.0
depends_on:
- 32
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: runtime
---

## Problem

Every empty state in this tool has been designed against a machine with 293
packages on it. The first thing a new user runs it on may have almost nothing.

## Proposal

Walk every pane and decide what it says when it has nothing to show: no
packages, no contested commands, nothing on disk to draw, a single source, a
source that found zero items. A pane that says nothing is a bug.

## Acceptance criteria

- [ ] Every view has a designed empty state that says what would fill it
- [ ] A machine with no package managers at all starts and explains itself
- [ ] A source finding zero items is absent, not shown as a zero row
- [ ] The map with nothing to draw says so rather than rendering blank
- [ ] Each empty state is asserted in the harness
