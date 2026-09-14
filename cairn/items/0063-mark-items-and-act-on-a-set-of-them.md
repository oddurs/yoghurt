---
id: 63
title: Mark items and act on a set of them
type: feature
status: backlog
milestone: later
created: 2026-09-13
updated: 2026-09-13
priority: p3
effort: l
area: list
---

## Problem

The original design had `space` to mark rows, drag-marquee in the map, and a
bulk uninstall composing one `brew uninstall a b c`.

## Proposal

Parked on purpose, and the reason is the point: yoghurt is permanently
read-only, and "it cannot break your machine" is a stronger promise than any
feature this would add. The detail pane already shows the uninstall command as
copyable text.

If this ever returns, it returns as marking plus composing a command to the
clipboard — never as executing one.

## Acceptance criteria

- [ ] Revisit only if the read-only promise is deliberately retired
- [ ] If it returns, it composes commands and never executes them
