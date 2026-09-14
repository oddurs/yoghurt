---
id: 15
title: Open and restore the terminal without leaving it broken
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
depends_on:
- 7
created: 2026-09-13
updated: 2026-09-14
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

## 2026-09-14

signal-hook was already in the tree via crossterm, so declaring it costs no new supply chain. Needed because unsafe_code=forbid rules out calling libc::signal directly. Handlers set a flag rather than acting: the only safe thing to do in a handler is set a flag, and the event loop is where the terminal can be given back in order.
