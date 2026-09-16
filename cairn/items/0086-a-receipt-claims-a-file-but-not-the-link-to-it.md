---
id: 86
title: A receipt claims a file but not the link to it
type: bug
status: done
milestone: v0.3
assignee: Oddur Sigurdsson
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

## 2026-09-15

Unclaimed 53 -> 22. The first attempt followed Resolves facts, which was the wrong mechanism: /Library/TeX/texbin is a symlinked *directory*, so the 34 files under it are ordinary files whose parent is a link and which have no Resolves fact of their own. Asking pkgutil about the canonical path is what actually works.
