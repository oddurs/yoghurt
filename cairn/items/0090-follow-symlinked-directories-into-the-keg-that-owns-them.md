---
id: 90
title: Follow symlinked directories into the keg that owns them
type: bug
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
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

## 2026-09-17

Fixed in the walk rather than the graph. A $PATH directory is canonicalised once, and every plain file under a directory that turned out to be a link gets a Resolves fact built by joining — so the cost is one canonicalize per $PATH entry, not one per executable. Resolving each entry was measured at 60ms of 160 when the walk was written, which is why the leaf-only test existed in the first place. On this machine the mise duplicate is gone, no previously-owned command became an orphan, and the scan still takes 4.14s. The TeX rows stay orphaned: texbin resolves into /usr/local/texlive, which the BasicTeX receipt does not claim. That is a separate gap.
