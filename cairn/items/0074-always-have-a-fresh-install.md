---
id: 74
title: Always have a fresh install
type: chore
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-14
priority: p1
effort: s
area: packaging
---

## Problem

The installed `yoghurt` is whatever was last built by hand. Working on this
repository means merging a branch and then remembering to run `cargo install`,
and forgetting means running yesterday's binary while reading today's code —
which is how you debug something that was fixed an hour ago.

## Proposal

`scripts/task install` joins the seam, and a `post-merge` hook runs it when what
you just merged is not what you have installed. Nothing to remember.

The binary says which commit it is, so "is this current" is answerable rather
than assumed.

## Acceptance criteria

- [ ] `scripts/task install` builds and installs from the working tree
- [ ] `yoghurt --version` reports the commit it was built from
- [ ] A merge that changes the source reinstalls without being asked
- [ ] A merge that changes nothing but docs does not rebuild
- [ ] It works in a worktree as well as the primary checkout
- [ ] Nothing reinstalls during CI
