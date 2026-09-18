---
id: 92
title: Say which duplicate wins on PATH
type: feature
status: backlog
milestone: v0.3
created: 2026-09-17
updated: 2026-09-17
priority: p2
area: path
effort: m
part_of:
- 37
---

## Problem

## Proposal

## Acceptance criteria

- [ ]

## 2026-09-17

Two real installations of the same name appear with no indication of which one
the shell reaches.

    vercel  homebrew  59.16.0  wanted
    vercel  npm       50.44.0  wanted

Both exist. `which -a` says Homebrew wins; the npm copy is nine major versions
behind, shadowed, and unreachable. That is the most useful fact about the pair
and the table omits it.

The Path view in v0.3 answers this properly. Until it lands the duplicate is
already visible without the signal, so consider a mark on the losing row.

Depends on 0037, which resolves each name on PATH to its winner.
