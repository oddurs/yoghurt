---
id: 59
title: Write down what yoghurt will never do
type: docs
status: backlog
milestone: v1.0
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: s
area: docs
---

## Problem

The strongest thing this tool can say about itself is what it refuses to do. A
program that reads your whole machine has to be explicit about its limits, and
the limits have to be findable, not implied.

## Proposal

A short, unhedged section in the README and a longer one in the docs:

- yoghurt never installs, uninstalls, upgrades or deletes anything
- yoghurt never writes to your shell configuration
- yoghurt never contacts the network unless explicitly asked
- yoghurt is macOS only, and says so rather than half-working elsewhere
- yoghurt reads; the only thing it writes is its own cache and config

Also state what is stable at 1.0: the CLI surface, the `--plain` output format,
and the config file. Those are the things people build on.

## Acceptance criteria

- [ ] The non-goals are in the README, in plain language, near the top
- [ ] What is stable at 1.0 is stated explicitly
- [ ] The cache and config paths are documented, with what is in them
- [ ] Anything deliberately deferred points at its item in `later`
