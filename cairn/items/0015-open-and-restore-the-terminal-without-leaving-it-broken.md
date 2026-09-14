---
id: 15
title: Open and restore the terminal without leaving it broken
type: feature
status: backlog
milestone: v0.1
depends_on:
- 7
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: m
area: runtime
---

## Problem

A TUI that panics in raw mode leaves the user with an invisible cursor, no echo
and a scrambled screen. Doing this to somebody once loses their trust in a tool
whose entire job is to be trustworthy about their machine.

## Proposal

Acquire raw mode, the alternate screen and mouse reporting on start; release all
three on every exit path. Install a panic hook that restores first and prints
the panic second. Handle SIGINT, SIGTERM and SIGHUP the same way.

## Acceptance criteria

- [ ] Normal quit restores the terminal
- [ ] A deliberate panic restores the terminal, then prints the panic message
- [ ] SIGTERM and SIGHUP restore the terminal
- [ ] A resize during a scan does not corrupt the screen
- [ ] Mouse reporting is switched off on exit, so the outer shell keeps selection
