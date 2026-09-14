---
id: 75
title: Take a screenshot without a terminal
type: feature
status: backlog
milestone: v0.1
created: 2026-09-14
updated: 2026-09-14
priority: p2
effort: s
area: cli
---

## Problem

Looking at the interface during development means writing a throwaway
`examples/snap.rs`, building it, running it, and deleting it. I have done this
four times in one day and deleted it four times, and each round costs a release
build of the example.

## Proposal

`yoghurt --screenshot 96x30` renders one frame to stdout and exits. The harness
that makes it possible already exists for the tests, so this is an argument and
a call.

Useful beyond development: it is how the README screenshots get made in 0061
without anybody photographing a terminal.

## Acceptance criteria

- [ ] `--screenshot WxH` prints one frame and exits 0
- [ ] It accepts the grouping, sort and filter arguments so any view is reachable
- [ ] No terminal is required, so it works over a pipe and in CI
- [ ] The output is exactly the width asked for, every line
