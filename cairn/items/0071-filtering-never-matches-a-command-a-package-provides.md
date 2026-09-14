---
id: 71
title: Filtering never matches a command a package provides
type: bug
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-14
priority: p1
effort: s
area: filter
---

## What happens

Typing `rg` does not find `ripgrep`, even though ripgrep is what puts `rg` on
the path. It finds `xorgproto` and thirteen cargo plugins instead, all of them
matching on their own names.

## What should happen

A package should be findable by the command you actually type.

## Reproduction

1. `yoghurt`, then `/rg`
2. ripgrep is absent. "rg" is not a substring of "ripgrep", so nothing else
   could have matched it.

## Cause

`Provides` facts are attached to the path the walk found — the link at
`/opt/homebrew/bin/rg` — while a package owns the keg the link points into.
`Item::provides` reads `artifact.provides` for the paths a package *owns*, and a
keg directory has none. It is empty for every package on the machine, so the
clause never matches anything.

The graph already resolves this the other way round: `owners_of` follows a
symlink to find the owner. Attribution has to go in that direction too — from
each command-providing artifact to its owner — rather than from the package
outward.
