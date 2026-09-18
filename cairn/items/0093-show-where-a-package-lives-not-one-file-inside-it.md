---
id: 93
title: Show where a package lives, not one file inside it
type: bug
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
created: 2026-09-17
updated: 2026-09-17
priority: p2
area: list
effort: s
---

## What happens

## What should happen

## Reproduction

1.

## 2026-09-17

The path column shows one artifact, chosen because it sorts first, where the
reader expects the package's location.

    org.tug.mactex.basictex2025  installer  330M  /Library/TeX/texbin/afm2tfm

330M is the package. `afm2tfm` is one binary inside it. Every package holding
more than one artifact reads this way.

Show the install prefix when the artifacts share one, and say how many
artifacts are under it when they do not.

## 2026-09-17

Fixed with Package::home(), which both the table and the list now call instead of each repeating the selection. Rule: one artifact is itself; a Homebrew formula is the keg whose directory name matches the reported version; anything else is the directory holding the most of its artifacts. The common ancestor was tried first and rejected — BasicTeX spans /Library and /usr/local, so its honest ancestor is /. Two rows on this machine change: BasicTeX from a single binary to /Library/TeX/texbin, and rustup shims to ~/.cargo/bin. Homebrew kegs are unaffected.
