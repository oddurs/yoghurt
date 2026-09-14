---
id: 7
title: Take the terminal dependency
type: chore
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-13
updated: 2026-09-14
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

## 2026-09-14

ratatui 0.30 + crossterm 0.29 takes the lock file from 12 crates to 184. Almost all of it is ratatui's own tree (unicode segmentation and width tables, cassowary for layout, compact_str). Accepted: the alternative is hand-rolling terminal control and grapheme-aware width under unsafe_code=forbid, which is a bug farm for no gain.
