---
id: 21
title: Print a table when stdout is not a terminal
type: feature
status: backlog
milestone: v0.1
depends_on:
- 13
created: 2026-09-13
updated: 2026-09-13
priority: p1
effort: s
area: cli
---

## Problem

`yoghurt | grep cargo` should work. A TUI that refuses to be piped is a tool
that cannot be scripted, and the table it prints today would be a regression to
lose.

## Proposal

A tty gets the interface, a pipe gets a table. Same data, same code path up to
the renderer; `--plain` forces the table even on a tty.

## Acceptance criteria

- [ ] `yoghurt` with a tty opens the interface
- [ ] `yoghurt | cat` prints a table and exits 0
- [ ] `--plain` forces the table on a tty
- [ ] The table is one row per package, tab-separated, no ANSI escapes
- [ ] A closed pipe exits 0 rather than reporting a broken pipe
