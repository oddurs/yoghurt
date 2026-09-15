---
id: 38
title: Show only the contested names
type: feature
status: backlog
milestone: v0.3
depends_on:
- 16
- 37
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: m
area: path
---

## Problem

Six hundred rows is not a view, it is a dump. The value is in the fifteen where
something is actually contested.

## Proposal

Default to contested names only; `a` shows all. Left column the name, middle the
winner with its source and version, right the losers marked `⊘`.

The most useful thing this can show is a version conflict — a year-old `rg` from
`cargo install` quietly beating the current Homebrew one. Rank contests so those
sort to the top and wrapper-script noise sorts to the bottom.

## Acceptance criteria

- [ ] Only contested names show by default, with the count in the pane title
- [ ] `a` toggles showing all commands, and back
- [ ] Version conflicts rank above same-version duplicates
- [ ] Clicking the winner or a loser opens detail on that one
- [ ] An uncontested machine shows a sentence saying so, not an empty pane

## 2026-09-15

Spike 0025 measured 26 contested names of 2178 and found a third are noise: fzf, fzf-tmux and fzf-preview.sh appear twice because Homebrew's own opt symlink resolves to the same file. A contest whose providers resolve to the same file is not a contest and must not be listed.
