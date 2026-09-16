---
id: 87
title: Put every commit on my PATH, not only merges
type: chore
status: done
milestone: v0.3
assignee: Oddur Sigurdsson
created: 2026-09-16
updated: 2026-09-16
priority: p1
effort: s
area: packaging
---

## Problem

`post-merge` keeps the installed binary matching `main`, which is the wrong
thing to test. Work happens in a worktree on a branch, and until it merges the
`yoghurt` on the PATH is the *previous* version — so trying a change means
remembering to run `scripts/task install` from the right directory, which is
exactly the remembering the post-merge hook was added to remove.

## Proposal

A `post-commit` hook that installs from the working tree it ran in. Commit in a
branch, and that branch is what you run.

A warm install is 1.5 seconds and only happens when the commit touched something
that changes the binary, so a docs commit costs nothing.

`yoghurt --version` already reports the commit, so which one is on the PATH is
answerable rather than assumed.

## Acceptance criteria

- [ ] A commit in a worktree installs that worktree's code
- [ ] A commit touching only docs or cairn items does not rebuild
- [ ] Nothing installs during CI
- [ ] `--version` names the commit that is installed
