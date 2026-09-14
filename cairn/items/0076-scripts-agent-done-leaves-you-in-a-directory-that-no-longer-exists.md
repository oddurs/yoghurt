---
id: 76
title: scripts/agent done leaves you in a directory that no longer exists
type: bug
status: done
milestone: v0.1
assignee: Oddur Sigurdsson
created: 2026-09-14
updated: 2026-09-14
priority: p2
effort: s
area: cli
---

## What happens

`scripts/agent done` removes the worktree you are standing in, so the next
command fails with `fatal: Unable to read current working directory`. It has
happened on every single item in this milestone.

## What should happen

It cannot change the caller's directory — no child process can — but it can say
so clearly, and it can avoid removing the ground you are standing on until it
has told you where to go.

## Proposal

Print the `cd` target *before* removing the worktree, the way `start` prints one
after creating it. Detect when the caller is inside the worktree being removed
and make the message unmissable rather than a line of prose.
