---
id: 37
title: Resolve every command on PATH to its winner
type: feature
status: backlog
milestone: v0.3
depends_on:
- 10
- 13
- 25
created: 2026-09-13
updated: 2026-09-13
priority: p0
effort: l
area: path
---

## Problem

592 binaries in one directory alone, and nothing on the machine tells you which
one wins when you type a name. That is the resolution gap, and it is the third
of the three the design exists to close.

## Proposal

For every command name, order its providers by the index of their PATH entry.
The first is the winner; the rest are shadowed. The resolution is exactly what
the shell would do, including the cases people get wrong: a later duplicate PATH
entry, a symlink chain, a directory that does not exist.

## Acceptance criteria

- [ ] Resolution matches `type -a` for every command on this machine, verified
- [ ] Duplicate PATH entries resolve the way the shell resolves them
- [ ] A non-existent or unreadable PATH directory is skipped, not fatal
- [ ] A shadowed provider records why it lost: its PATH index
- [ ] Resolution over 600 commands completes in under 50ms
