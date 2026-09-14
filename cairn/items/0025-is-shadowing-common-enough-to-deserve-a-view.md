---
id: 25
title: Is shadowing common enough to deserve a view?
type: spike
status: backlog
milestone: v0.2
depends_on:
- 13
created: 2026-09-13
updated: 2026-09-14
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

## 2026-09-14

Early evidence from 0013 on this machine: 26 contested command names out of 2178. Real conflicts (bash: homebrew beats /bin/bash; docker: /usr/local/bin beats orbstack) are mixed with noise (fzf beating /opt/homebrew/opt/fzf/bin/fzf, which is the same package reached through Homebrew's own opt symlink). The spike should count real conflicts separately from a package shadowing itself.
