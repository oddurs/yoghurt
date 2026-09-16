---
id: 86
title: A receipt claims a file but not the link to it
type: bug
status: backlog
milestone: v0.3
created: 2026-09-15
updated: 2026-09-15
priority: p2
effort: s
area: source
---

## What happens

34 files under `/Library/TeX/texbin` are reported as unclaimed. They are
symlinks into `/usr/local/texlive`, which the installer receipt does claim, so
the target is owned and the link is not.

## What should happen

Following a link to something owned is exactly what `owners_of` already does for
Homebrew's `bin` links. The receipts pass should not need to ask `pkgutil` about
a path whose target is already claimed.

## Fix

Resolve before asking, and skip anything whose target already has an owner.
