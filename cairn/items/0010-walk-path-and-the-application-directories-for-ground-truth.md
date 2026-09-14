---
id: 10
title: Walk PATH and the application directories for ground truth
type: feature
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
depends_on:
- 9
created: 2026-09-13
updated: 2026-09-14
priority: p0
effort: m
area: scan
---

## Problem

Adapters make claims. Nothing yet says what is actually on the machine, so a
package manager that forgot about a file, or a file no manager ever knew about,
is invisible. On this machine that is 36 applications and an unknown share of
592 binaries.

## Proposal

The walk is not an adapter. It is ground truth: every entry on `$PATH`, every
bundle in `/Applications` and `~/Applications`, emitted as artifacts and the
commands they provide. Adapter claims are then checked against it, and an
orphan is simply what is left over rather than something anybody detects.

Resolve symlinks so that `/opt/homebrew/bin/rg` pointing into the Cellar is one
artifact, not two. A symlink whose target is gone is a broken artifact, not a
missing one.

## Acceptance criteria

- [ ] Every directory on `$PATH`, in order, becomes commands with their index
- [ ] Application bundles in both application directories become artifacts
- [ ] Symlinks resolve to their target; a dangling symlink is recorded as dangling
- [ ] An unreadable directory is skipped with a warning, never a panic
- [ ] Runs against a fixture tree in tests — never against the developer's machine
- [ ] Completes in under 100ms for 600 PATH entries

## 2026-09-14

Measured on this machine: 17ms for the whole walk, 2996 artifacts and 2177 distinct commands, plus 3ms to assemble the graph. The criterion assumed 600 PATH entries; the real number is 2177 commands, because 592 was only /opt/homebrew/bin. First cut took 159ms — canonicalize() was being called on every executable rather than only on symlinks, which is 9x of the cost for nothing.
