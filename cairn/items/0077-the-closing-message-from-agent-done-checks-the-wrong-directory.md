---
id: 77
title: The closing message from agent done checks the wrong directory
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

`scripts/agent done` warns correctly *before* removing a worktree you are
standing in, but its closing message says "Done. <main> is on main" as though
nothing were wrong. The one case the closing message exists for never fires.

## What should happen

When the caller's shell is inside the worktree that was removed, the last thing
printed should say so, because that is the line still on screen when the next
command fails.

## Cause

`cmd_done` runs `cd "$main"` partway through so it can remove the worktree. The
closing check then tests `$PWD`, which by that point is `$main` and never
matches. Whether the *caller* was inside has to be captured at entry, before the
script moves.
