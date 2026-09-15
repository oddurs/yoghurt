---
id: 25
title: Is shadowing common enough to deserve a view?
type: spike
status: done
milestone: v0.2
assignee: Oddur Sigurdsson
depends_on:
- 13
created: 2026-09-13
updated: 2026-09-15
priority: p0
effort: s
area: path
---

## Question

On a real machine, how many command names are provided by more than one thing?

## Why it has to be answered before the work

The Path view is the most speculative part of the design and the whole of v0.3.
Its value rests entirely on contested names being common enough to be
interesting. If the honest answer on a heavily-used machine is two, the view is
a column in the Inventory, not a milestone.

This spike is deliberately in v0.2 so that v0.3 is de-risked before it starts.

## Options

- Many contested names — the view is justified as designed
- A handful — it becomes a facet and a detail-pane line
- Effectively none — it moves to `later` and v0.3 becomes something else

## What would settle it

Once every adapter from v0.2 is wired, count contested names across this machine
and two others. Look at what the contests actually are: a real version conflict
is interesting, a wrapper script shadowing its own binary is noise.

Timebox: half a day. The answer is the count, the breakdown, and a decision.

## Answer

**Yes, but a third of it is noise, and the view should say which.**

Measured on this machine: **26 contested names out of 2178 commands.** The split
matters more than the count.

*Real conflicts*, where the winner is a different piece of software:
`bash` — Homebrew's 5.3 beats `/bin/bash` 3.2; `docker` and its two helpers —
`/usr/local/bin` beats OrbStack's; `ruby`, `python3` — Homebrew beats the system.

*Noise*, where a package shadows itself: `fzf`, `fzf-tmux`, `fzf-preview.sh`
reached both directly and through Homebrew's own `opt` symlink. Same file, two
paths.

So v0.3 proceeds, with one requirement the original design did not have: a
contest whose providers resolve to the **same file** is not a contest and must
not be listed. That alone removes roughly a third of the 26 here.

The count is also lower than the design assumed, which said "perhaps fifteen" of
592. It is 26 of 2178 — a smaller fraction of a much larger set, and still
enough to be worth a view, because the ones that are real are exactly the ones
that bite.
