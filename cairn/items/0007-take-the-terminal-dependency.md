---
id: 7
title: Take the terminal dependency
type: chore
status: backlog
milestone: v0.1
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: runtime
---

## Problem

yoghurt has no dependencies today, and the rule is that a dependency earns its
place. A terminal interface is not something to hand-roll: raw mode, alternate
screen, mouse reporting, resize signals and cell diffing are each a source of
bugs on somebody else's terminal emulator.

## Proposal

Take `ratatui` and `crossterm`, and nothing else. Both are the standard choice,
both are what harrow already uses, and mouse reporting in particular is not
worth reimplementing against `#![forbid(unsafe_code)]`.

Write the argument into the commit body, as the rules require.

## Acceptance criteria

- [ ] `ratatui` and `crossterm` are in `Cargo.toml` with the reason in the commit body
- [ ] No other dependency is added alongside them
- [ ] `scripts/task check` stays green and `cargo build --release` still strips and LTOs
- [ ] `unsafe_code = "forbid"` still holds
