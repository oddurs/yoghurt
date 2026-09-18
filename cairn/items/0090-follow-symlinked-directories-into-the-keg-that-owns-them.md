---
id: 90
title: Follow symlinked directories into the keg that owns them
type: bug
status: backlog
milestone: v0.2
created: 2026-09-17
updated: 2026-09-17
priority: p1
area: graph
effort: m
---

## What happens

## What should happen

## Reproduction

1.

## 2026-09-17

`mise` appears twice: once from the Cellar as wanted, once as an orphan at
`/opt/homebrew/opt/mise/bin/mise`. That path resolves to exactly the first.

The cause is a symlinked *directory*: `opt/mise` is a link, `bin/mise` inside
it is not, so no `Resolves` fact is emitted and the prefix test against the
Cellar never matches. This is the same root cause as the `/Library/TeX/texbin`
fix, which was applied to pkgutil receipts only and never to Homebrew.

Canonicalise the directory chain, not just the leaf. Scope on this machine is
one Cellar row plus three TeX rows, where `dvipng` lists twice and `man` is a
directory counted as a command.
